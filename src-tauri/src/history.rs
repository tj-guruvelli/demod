//! Local-only transcription history (US-013 build-out, TJ 2026-07-21).
//!
//! The rest of the app is privacy-first by construction: the temp WAV is
//! deleted on every exit path, and neither `applog` nor `log_transcript` ever
//! persists the transcript TEXT. This module deliberately reverses that for the
//! one case the user asked for - a personal history of their own dictations -
//! and contains that reversal to a single, clearly-named place:
//!
//!   * It writes ONLY to `history.json` inside the app data dir (next to
//!     `config.json`). Nothing here ever leaves the machine - no network, no
//!     telemetry. The text stays exactly where the user's config already lives.
//!   * `history_enabled` is ON by default (the user asked for history), but is
//!     gated at the call site (`recorder.rs`), so turning it off writes nothing.
//!   * Writes are best-effort: a failed history write must never fail or delay
//!     a dictation (same philosophy as `applog`).
//!
//! Persistence mirrors `settings.rs` exactly: pretty JSON via serde, written to
//! a `.tmp` sibling and atomically renamed, so a crash/power-loss can never
//! leave `history.json` half-written. A hard `MAX_ITEMS` cap drops the oldest
//! entries so the file can never grow without bound.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Serializes every read-modify-write of history.json. The append path (the
/// recorder's async task) and the delete/clear commands (Tauri command threads)
/// can run concurrently; without this a delete racing an append could resurrect
/// a deleted item or drop a fresh one. Rendering-only reads may run unlocked.
static HISTORY_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` with exclusive access to the history file. Poison-tolerant, so a
/// panic in one holder never permanently wedges history writes.
pub fn with_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

/// Newest N transcripts kept. Older ones are dropped on append, so the file is
/// bounded regardless of how much the user dictates.
const MAX_ITEMS: usize = 500;

/// One stored transcript plus the metadata available at the moment it was
/// produced. Serialized with camelCase keys so the TS frontend reads it
/// directly. Every field is plain data - no secrets, no handles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    /// Stable unique id: `"{ts_ms}-{seq}"`. Used by the UI for copy/delete.
    pub id: String,
    /// The final, cleaned transcript text (post dictionary/formatting cleanup).
    pub text: String,
    /// Wall-clock creation time, epoch milliseconds (SystemTime, not Instant).
    #[serde(rename = "tsMs")]
    pub ts_ms: u128,
    /// `"local"` or `"cloud"`.
    pub engine: String,
    /// Whisper model (local) or cloud model id, for the user's reference.
    pub model: String,
    /// Character count of `text` (cheap for the UI, avoids re-counting).
    #[serde(rename = "charCount")]
    pub char_count: usize,
    /// Recording length in ms, if it was captured before the timer reset.
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<u64>,
}

/// The whole on-disk history: a flat, newest-last list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryStore {
    #[serde(default)]
    pub items: Vec<HistoryItem>,
    /// Monotonic counter making ids unique even within the same millisecond.
    #[serde(default)]
    seq: u64,
}

impl HistoryStore {
    fn path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("history.json")
    }

    fn tmp_path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("history.json.tmp")
    }

    /// Load the store, or an empty one if the file is missing or unreadable.
    /// A corrupt file degrades to empty rather than crashing - losing view
    /// history is annoying, blocking dictation over it would be worse.
    pub fn load(app_dir: &PathBuf) -> Self {
        match std::fs::read_to_string(Self::path(app_dir)) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Atomically persist: write to `history.json.tmp`, then rename over
    /// `history.json`. Mirrors `Settings::save`.
    pub fn save(&self, app_dir: &PathBuf) -> Result<(), String> {
        std::fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let tmp = Self::tmp_path(app_dir);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, Self::path(app_dir)).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Build a new item from a produced transcript + metadata and append it,
    /// dropping the oldest entries past `MAX_ITEMS`. Returns the new item's id.
    pub fn append(
        &mut self,
        text: String,
        engine: String,
        model: String,
        duration_ms: Option<u64>,
    ) -> String {
        let ts_ms = now_ms();
        self.seq = self.seq.wrapping_add(1);
        let id = format!("{}-{}", ts_ms, self.seq);
        let char_count = text.chars().count();
        self.items.push(HistoryItem {
            id: id.clone(),
            text,
            ts_ms,
            engine,
            model,
            char_count,
            duration_ms,
        });
        let n = self.items.len();
        if n > MAX_ITEMS {
            self.items.drain(0..(n - MAX_ITEMS));
        }
        id
    }

    /// Remove one item by id (no-op if absent).
    pub fn delete(&mut self, id: &str) {
        self.items.retain(|it| it.id != id);
    }

    /// Drop every stored transcript.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Drop entries older than `max_age_ms` relative to now. Returns how many
    /// were removed. Used by the Privacy retention control (auto-delete).
    pub fn prune_older_than(&mut self, max_age_ms: u128) -> usize {
        if max_age_ms == 0 {
            return 0;
        }
        let cutoff = now_ms().saturating_sub(max_age_ms);
        let before = self.items.len();
        self.items.retain(|it| it.ts_ms >= cutoff);
        before - self.items.len()
    }

    /// Newest-first copy for the UI (the store keeps newest-last).
    pub fn newest_first(&self) -> Vec<HistoryItem> {
        let mut out = self.items.clone();
        out.reverse();
        out
    }
}

/// Wall-clock epoch milliseconds. `SystemTime`, never `Instant` (which is
/// monotonic and cannot be rendered as a date). Same pattern as `applog`.
pub fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_caps_at_max_and_keeps_newest() {
        let mut store = HistoryStore::default();
        for i in 0..(MAX_ITEMS + 25) {
            store.append(format!("t{}", i), "local".into(), "small".into(), None);
        }
        assert_eq!(store.items.len(), MAX_ITEMS);
        // Oldest 25 dropped -> first surviving item is t25.
        assert_eq!(store.items.first().unwrap().text, "t25");
        assert_eq!(
            store.items.last().unwrap().text,
            format!("t{}", MAX_ITEMS + 24)
        );
    }

    #[test]
    fn ids_are_unique_within_same_millis() {
        let mut store = HistoryStore::default();
        let a = store.append("a".into(), "local".into(), "small".into(), None);
        let b = store.append("b".into(), "local".into(), "small".into(), None);
        assert_ne!(a, b);
    }

    #[test]
    fn delete_and_clear() {
        let mut store = HistoryStore::default();
        let id = store.append("keep-or-not".into(), "local".into(), "small".into(), None);
        store.append("other".into(), "local".into(), "small".into(), None);
        store.delete(&id);
        assert_eq!(store.items.len(), 1);
        assert_eq!(store.items[0].text, "other");
        store.clear();
        assert!(store.items.is_empty());
    }

    #[test]
    fn newest_first_reverses() {
        let mut store = HistoryStore::default();
        store.append("first".into(), "local".into(), "small".into(), None);
        store.append("second".into(), "local".into(), "small".into(), None);
        let nf = store.newest_first();
        assert_eq!(nf[0].text, "second");
        assert_eq!(nf[1].text, "first");
    }
}
