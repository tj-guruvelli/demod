//! Durable, timestamped event log for failure classes that previously left
//! zero evidence behind.
//!
//! The only durable channel before this was `paste.rs`'s `paste.log`, which
//! has no timestamps and covers exactly one feature. In the release build
//! `windows_subsystem = "windows"` means stdout/stderr (`println!`/
//! `eprintln!`) go nowhere, so a mic failure, a hotkey that silently failed
//! to register, a crashed sidecar, etc. left a user with "it just didn't
//! work" and nothing to debug from.
//!
//! ABSOLUTE PRIVACY CONSTRAINT: never log transcript text or the Groq API
//! key here. Character counts and error classes only — same rule
//! `log_transcript` in main.rs already follows.
//!
//! Written to `dmod.log` next to `config.json`/`paste.log` in the app data
//! dir, with a size cap and single-file rotation (`dmod.log` ->
//! `dmod.log.old`) so it cannot grow unbounded on a long-running install.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Rotate once the current log file would exceed this size.
const MAX_LOG_BYTES: u64 = 1_000_000; // 1 MB

fn log_path(app_dir: &Path) -> PathBuf {
    app_dir.join("dmod.log")
}

fn rotated_path(app_dir: &Path) -> PathBuf {
    app_dir.join("dmod.log.old")
}

/// Append one timestamped line: `[unix_seconds][category] message`.
///
/// Best-effort: failures to create the directory or open/write the file are
/// swallowed (there is no more-durable channel to report a logging failure
/// to), matching the existing behavior of `paste.rs::log_paste_decision`.
/// Resolve the app data dir the same way `main.rs::get_app_dir` and
/// `paste.rs::log_paste_decision` do, for call sites that have no
/// already-loaded `AppState`/`app_dir` in scope (e.g. a Recorder method with
/// no reference to the binary crate's `AppState`, or a low-level hook
/// callback). Never change this path — it doubles as the config/API-key
/// directory (see main.rs::get_app_dir's own warning).
pub fn default_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.typr.app")
}

/// Convenience wrapper over `log_event` for call sites without an app_dir
/// already in scope.
pub fn log_event_default(category: &str, message: &str) {
    log_event(&default_app_dir(), category, message);
}

pub fn log_event(app_dir: &Path, category: &str, message: &str) {
    if std::fs::create_dir_all(app_dir).is_err() {
        return;
    }

    let path = log_path(app_dir);
    rotate_if_needed(app_dir, &path);

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{}][{}] {}", ts, category, message);
    }
}

/// Single-file rotation: if the log is already at/over the cap, move it to
/// `dmod.log.old` (replacing any previous rotation) and start a fresh file.
fn rotate_if_needed(_app_dir: &Path, path: &Path) {
    let Ok(meta) = std::fs::metadata(path) else {
        return; // doesn't exist yet - nothing to rotate
    };
    if meta.len() < MAX_LOG_BYTES {
        return;
    }
    let old = rotated_path(path.parent().unwrap_or_else(|| Path::new(".")));
    let _ = std::fs::remove_file(&old);
    let _ = std::fs::rename(path, &old);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_log_event_writes_timestamped_line() {
        let dir = temp_dir().join("dmod_test_applog_basic");
        let _ = std::fs::remove_dir_all(&dir);

        log_event(&dir, "test", "hello");
        let contents = std::fs::read_to_string(log_path(&dir)).unwrap();
        assert!(contents.contains("[test] hello"));
        // timestamp bracket present
        assert!(contents.starts_with('['));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_log_event_never_contains_secrets_by_construction() {
        // Not a real secret-scan (the caller controls the message) - this
        // just pins that the helper itself adds nothing beyond category +
        // message + timestamp.
        let dir = temp_dir().join("dmod_test_applog_shape");
        let _ = std::fs::remove_dir_all(&dir);

        log_event(&dir, "mic", "start failed");
        let contents = std::fs::read_to_string(log_path(&dir)).unwrap();
        let line = contents.lines().next().unwrap();
        assert!(line.contains("[mic] start failed"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_log_rotates_when_over_cap() {
        let dir = temp_dir().join("dmod_test_applog_rotate");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Pre-fill the log past the cap.
        let big = vec![b'x'; (MAX_LOG_BYTES as usize) + 10];
        std::fs::write(log_path(&dir), &big).unwrap();

        log_event(&dir, "test", "after rotation");

        let old_len = std::fs::metadata(rotated_path(&dir)).unwrap().len();
        assert!(old_len >= MAX_LOG_BYTES);

        let fresh = std::fs::read_to_string(log_path(&dir)).unwrap();
        assert!(fresh.contains("after rotation"));
        assert!(fresh.len() < big.len());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
