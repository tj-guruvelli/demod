//! Model download with integrity verification.
//!
//! Bytes are streamed to a per-attempt `<dest>.<pid>-<nanos>.part` file while
//! a SHA-256 is computed on the fly. After the stream ends the part file is
//! hashed again from disk; the two digests must agree with each other and with
//! the pinned value for that source before the file is renamed into place.
//! Otherwise the part file is deleted and the error names the source, so a
//! tampered, corrupted, or concurrently modified file is never loaded by
//! whisper.cpp. The unique part name means two attempts can never share a
//! temp file.

use crate::models::{digest_matches, sha256_file, to_hex, ModelSource};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

/// Why a single source failed. Network and HTTP failures fall through to the
/// next source; a digest mismatch aborts the whole download.
#[derive(Debug, PartialEq, Eq)]
pub enum SourceFailure {
    Unreachable(String),
    Corrupt(String),
}

/// Path of the temporary file one download attempt streams into. Unique per
/// process and per call, so no two attempts (or two processes) ever write the
/// same part file.
pub fn part_path(dest: &Path) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut name = dest
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(format!(".{}-{}.part", std::process::id(), nanos));
    dest.with_file_name(name)
}

/// Hash the finished part file from disk and require it to equal the digest
/// computed while streaming. A difference means the bytes on disk are not the
/// bytes that were received (concurrent writer, disk error), which is treated
/// like a corrupt download.
pub fn confirm_on_disk(part: &Path, streamed_sha256: &str, source: &ModelSource) -> Result<String, SourceFailure> {
    let on_disk = match sha256_file(part) {
        Ok(h) => h,
        Err(e) => {
            let _ = std::fs::remove_file(part);
            return Err(SourceFailure::Unreachable(e));
        }
    };
    if !digest_matches(&on_disk, streamed_sha256) {
        let _ = std::fs::remove_file(part);
        return Err(SourceFailure::Corrupt(format!(
            "Model file from {} changed on disk while downloading (streamed SHA-256 {}, on disk {}).              The download was discarded.",
            source.label, streamed_sha256, on_disk
        )));
    }
    Ok(on_disk)
}

/// Final step of a download: keep the part file only if its digest matches.
/// On mismatch the part file is removed and an error naming the source is
/// returned. Pure file-system logic so it is unit-testable without a network.
pub fn finalize_download(
    part: &Path,
    dest: &Path,
    actual_sha256: &str,
    source: &ModelSource,
) -> Result<(), SourceFailure> {
    if !digest_matches(actual_sha256, source.sha256) {
        let _ = std::fs::remove_file(part);
        return Err(SourceFailure::Corrupt(format!(
            "Model file from {} failed integrity check (expected SHA-256 {}, got {}). \
             The download was discarded.",
            source.label, source.sha256, actual_sha256
        )));
    }
    std::fs::rename(part, dest).map_err(|e| {
        let _ = std::fs::remove_file(part);
        SourceFailure::Unreachable(format!(
            "Could not move verified model into place at {}: {}",
            dest.display(),
            e
        ))
    })
}

async fn download_from_source(
    app: &AppHandle,
    source: &ModelSource,
    dest: &Path,
) -> Result<(), SourceFailure> {
    let client = reqwest::Client::new();
    let response = client
        .get(source.url)
        .send()
        .await
        .map_err(|e| SourceFailure::Unreachable(format!("{}: request failed: {}", source.label, e)))?;

    if !response.status().is_success() {
        return Err(SourceFailure::Unreachable(format!(
            "{}: HTTP {}",
            source.label,
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(0);

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SourceFailure::Unreachable(e.to_string()))?;
    }

    let part = part_path(dest);
    let mut file = std::fs::File::create(&part)
        .map_err(|e| SourceFailure::Unreachable(format!("Cannot create {}: {}", part.display(), e)))?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                return Err(SourceFailure::Unreachable(format!(
                    "{}: stream error: {}",
                    source.label, e
                )));
            }
        };
        if let Err(e) = file.write_all(&chunk) {
            let _ = std::fs::remove_file(&part);
            return Err(SourceFailure::Unreachable(e.to_string()));
        }
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;

        let percent = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                downloaded,
                total,
                percent,
            },
        );
    }
    drop(file);

    let streamed = to_hex(&hasher.finalize());
    let on_disk = confirm_on_disk(&part, &streamed, source)?;
    finalize_download(&part, dest, &on_disk, source)
}

/// Download a model trying each pinned source in order.
///
/// Unreachable sources (network error, non-2xx) fall through to the next one.
/// A digest mismatch stops immediately: a wrong file from one source is a
/// reason to stop trusting the download, not to quietly try elsewhere.
pub async fn download_model(
    app: AppHandle,
    sources: &[ModelSource],
    dest: &PathBuf,
) -> Result<(), String> {
    if sources.is_empty() {
        return Err("No download source is known for this model.".to_string());
    }
    let mut failures: Vec<String> = Vec::new();
    for source in sources {
        match download_from_source(&app, source, dest).await {
            Ok(()) => return Ok(()),
            Err(SourceFailure::Corrupt(msg)) => return Err(msg),
            Err(SourceFailure::Unreachable(msg)) => failures.push(msg),
        }
    }
    Err(format!(
        "Could not download the model from any source: {}",
        failures.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sha256_file;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("demod-dl-test-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn source_with(sha: &'static str) -> ModelSource {
        ModelSource {
            label: "test source",
            url: "https://example.invalid/ggml-test.bin",
            sha256: sha,
        }
    }

    #[test]
    fn part_path_is_unique_and_stays_in_dest_dir() {
        let dest = Path::new("C:/x/y/ggml-small.bin");
        let a = part_path(dest);
        let b = part_path(dest);
        assert_eq!(a.parent(), dest.parent());
        let name = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("ggml-small.bin."), "{}", name);
        assert!(name.ends_with(".part"), "{}", name);
        assert!(name.contains(&std::process::id().to_string()), "{}", name);
        assert_ne!(a, b, "two attempts must never share a part file");
    }

    #[test]
    fn confirm_on_disk_accepts_matching_bytes() {
        let d = temp_dir("disk-ok");
        let part = d.join("m.part");
        std::fs::write(&part, b"hello").unwrap();
        let h = confirm_on_disk(&part, HELLO_SHA256, &source_with(HELLO_SHA256)).unwrap();
        assert_eq!(h, HELLO_SHA256);
        assert!(part.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn confirm_on_disk_rejects_and_deletes_when_bytes_changed_after_streaming() {
        let d = temp_dir("disk-bad");
        let part = d.join("m.part");
        // Streamed digest says "hello"; someone swapped the file afterwards.
        std::fs::write(&part, b"swapped").unwrap();
        let err = confirm_on_disk(&part, HELLO_SHA256, &source_with(HELLO_SHA256)).unwrap_err();
        match err {
            SourceFailure::Corrupt(msg) => assert!(msg.contains("changed on disk"), "{}", msg),
            other => panic!("expected Corrupt, got {:?}", other),
        }
        assert!(!part.exists(), "swapped part file must be deleted");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn finalize_moves_verified_file_into_place() {
        let d = temp_dir("ok");
        let part = d.join("ggml-test.bin.part");
        let dest = d.join("ggml-test.bin");
        std::fs::write(&part, b"hello").unwrap();
        let actual = sha256_file(&part).unwrap();

        finalize_download(&part, &dest, &actual, &source_with(HELLO_SHA256)).unwrap();

        assert!(dest.exists(), "verified file must be renamed into place");
        assert!(!part.exists(), "part file must not linger");
        assert_eq!(std::fs::read(&dest).unwrap(), b"hello");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn finalize_deletes_file_on_digest_mismatch() {
        let d = temp_dir("bad");
        let part = d.join("ggml-test.bin.part");
        let dest = d.join("ggml-test.bin");
        std::fs::write(&part, b"hello, tampered").unwrap();
        let actual = sha256_file(&part).unwrap();

        let err = finalize_download(&part, &dest, &actual, &source_with(HELLO_SHA256)).unwrap_err();

        match err {
            SourceFailure::Corrupt(msg) => {
                assert!(msg.contains("test source"), "{}", msg);
                assert!(msg.contains(HELLO_SHA256), "{}", msg);
                assert!(msg.contains(&actual), "{}", msg);
            }
            other => panic!("expected Corrupt, got {:?}", other),
        }
        assert!(!dest.exists(), "a corrupt file must never reach the model path");
        assert!(!part.exists(), "corrupt part file must be deleted");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn finalize_accepts_uppercase_pinned_digest() {
        let d = temp_dir("case");
        let part = d.join("m.part");
        let dest = d.join("m");
        std::fs::write(&part, b"hello").unwrap();
        let actual = sha256_file(&part).unwrap();
        let upper: &'static str =
            Box::leak(HELLO_SHA256.to_ascii_uppercase().into_boxed_str());
        finalize_download(&part, &dest, &actual, &source_with(upper)).unwrap();
        assert!(dest.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn finalize_does_not_overwrite_when_digest_wrong() {
        let d = temp_dir("keep");
        let part = d.join("m.part");
        let dest = d.join("m");
        std::fs::write(&dest, b"good existing model").unwrap();
        std::fs::write(&part, b"evil").unwrap();
        let actual = sha256_file(&part).unwrap();
        let _ = finalize_download(&part, &dest, &actual, &source_with(HELLO_SHA256));
        assert_eq!(std::fs::read(&dest).unwrap(), b"good existing model");
        let _ = std::fs::remove_dir_all(&d);
    }
}
