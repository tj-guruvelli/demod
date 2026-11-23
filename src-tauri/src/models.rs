//! Whisper model catalogue: where each ggml model may be downloaded from and
//! the SHA-256 digest the downloaded bytes must match.
//!
//! Every source is pinned. A download whose digest does not match is deleted
//! and reported, never loaded into whisper.cpp. Sources are tried in order:
//! the official Hugging Face upload first, then (for small/medium only) an
//! archive.org mirror that some corporate proxies allow when Hugging Face is
//! blocked. The mirror files are NOT byte-identical to the official ones
//! (older Aug-2023 conversions, 17 bytes longer), so each source carries its
//! own digest. A digest mismatch on one source is treated as tampering and
//! aborts the whole download; only network/HTTP failures fall through to the
//! next source.

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// One place a model file can be fetched from, with the digest it must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSource {
    /// Short human label used in error messages ("Hugging Face", "archive.org").
    pub label: &'static str,
    pub url: &'static str,
    /// Lowercase hex SHA-256 of the complete file.
    pub sha256: &'static str,
}

const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const MIRROR_BASE: &str = "https://archive.org/download/whisper-models";

// Official digests: Hugging Face LFS object ids for ggerganov/whisper.cpp,
// read from https://huggingface.co/api/models/ggerganov/whisper.cpp/tree/main
// on 2026-11-22. ggml-small.bin was additionally confirmed against a locally
// downloaded copy.
const HF_SMALL: ModelSource = ModelSource {
    label: "Hugging Face",
    url: concat!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main",
        "/ggml-small.bin"
    ),
    sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
};
const HF_MEDIUM: ModelSource = ModelSource {
    label: "Hugging Face",
    url: concat!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main",
        "/ggml-medium.bin"
    ),
    sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
};
const HF_LARGE_V3: ModelSource = ModelSource {
    label: "Hugging Face",
    url: concat!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main",
        "/ggml-large-v3.bin"
    ),
    sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
};

// Mirror digests: computed 2026-11-22 from a full download of each file,
// cross-checked against the SHA-1 archive.org publishes in
// whisper-models_files.xml (small 8a6f743c..., medium 635bfd68...).
const MIRROR_SMALL: ModelSource = ModelSource {
    label: "archive.org mirror",
    url: concat!("https://archive.org/download/whisper-models", "/ggml-small.bin"),
    sha256: "a46a837a38c3847b469270db9a5f1d737625928d0e487e97e0665941f49efc00",
};
const MIRROR_MEDIUM: ModelSource = ModelSource {
    label: "archive.org mirror",
    url: concat!("https://archive.org/download/whisper-models", "/ggml-medium.bin"),
    sha256: "84b573e702358589b87257d145638f5e6383ac0fe474be723b0cf3977c64ce0b",
};

/// Ordered download sources for a model size. Empty for unknown sizes.
pub fn model_sources(model_size: &str) -> Vec<ModelSource> {
    match model_size {
        "small" => vec![HF_SMALL, MIRROR_SMALL],
        "medium" => vec![HF_MEDIUM, MIRROR_MEDIUM],
        "large-v3" => vec![HF_LARGE_V3],
        _ => Vec::new(),
    }
}

/// Lowercase hex encoding of a digest.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Streaming SHA-256 of a file on disk, as lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open {} for hashing: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("Read error while hashing {}: {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(&hasher.finalize()))
}

/// Compare two hex digests, ignoring case.
pub fn digest_matches(actual_hex: &str, expected_hex: &str) -> bool {
    actual_hex.eq_ignore_ascii_case(expected_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_sha256_hex(s: &str) -> bool {
        s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    }

    #[test]
    fn hf_base_and_mirror_base_match_sources() {
        assert!(HF_SMALL.url.starts_with(HF_BASE));
        assert!(MIRROR_SMALL.url.starts_with(MIRROR_BASE));
    }

    #[test]
    fn small_and_medium_try_official_then_mirror() {
        let s = model_sources("small");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].url, "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin");
        assert_eq!(s[1].url, "https://archive.org/download/whisper-models/ggml-small.bin");
        let m = model_sources("medium");
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].url, "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin");
        assert_eq!(m[1].url, "https://archive.org/download/whisper-models/ggml-medium.bin");
    }

    #[test]
    fn large_v3_is_official_only() {
        let l = model_sources("large-v3");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].url, "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin");
    }

    #[test]
    fn unknown_size_has_no_sources() {
        assert!(model_sources("tiny").is_empty());
        assert!(model_sources("").is_empty());
    }

    #[test]
    fn every_source_has_a_real_pinned_digest() {
        for size in ["small", "medium", "large-v3"] {
            for src in model_sources(size) {
                assert!(
                    is_sha256_hex(src.sha256),
                    "{} {} has no valid pinned SHA-256: {:?}",
                    size,
                    src.label,
                    src.sha256
                );
            }
        }
    }

    #[test]
    fn mirror_and_official_digests_differ() {
        // The mirror files are different bytes; if these ever become equal the
        // mirror was re-uploaded and the comment in this module is stale.
        assert_ne!(HF_SMALL.sha256, MIRROR_SMALL.sha256);
        assert_ne!(HF_MEDIUM.sha256, MIRROR_MEDIUM.sha256);
    }

    #[test]
    fn sha256_file_hashes_known_content() {
        let dir = std::env::temp_dir().join(format!("demod-models-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("hello.bin");
        std::fs::write(&p, b"hello").unwrap();
        let h = sha256_file(&p).unwrap();
        assert_eq!(h, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha256_file_reports_missing_file() {
        let err = sha256_file(Path::new("Z:/definitely/not/here.bin")).unwrap_err();
        assert!(err.contains("Cannot open"), "{}", err);
    }

    #[test]
    fn digest_matches_is_case_insensitive_and_strict() {
        assert!(digest_matches("ABCDEF", "abcdef"));
        assert!(!digest_matches("abcdef", "abcde0"));
        assert!(!digest_matches("", "abcdef"));
    }

    #[test]
    fn to_hex_is_lowercase_two_chars_per_byte() {
        assert_eq!(to_hex(&[0x00, 0xff, 0x0a]), "00ff0a");
    }
}
