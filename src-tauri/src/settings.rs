use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    // FIX 4: these six original fields had NO #[serde(default)], while every
    // field added later did. serde_json::from_str fails the WHOLE object if
    // any one field is missing/malformed, and Settings::load previously
    // swallowed that with unwrap_or_default() - so a single bad/missing
    // field (or a write truncated mid-flight) silently discarded everything,
    // including the user's Groq API key. Defaults here mean a malformed or
    // partial config degrades gracefully field-by-field instead of nuking
    // the whole file.
    #[serde(default = "default_microphone")]
    pub microphone: String,
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(rename = "whisperModel", default = "default_whisper_model")]
    pub whisper_model: String,
    #[serde(rename = "groqApiKey", default)]
    pub groq_api_key: String,
    /// "auto" (default, US-005 tap/hold gesture model), or the legacy
    /// explicit overrides "toggle" / "push-to-talk".
    #[serde(rename = "recordingMode", default = "default_recording_mode")]
    pub recording_mode: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    /// Spoken language passed to whisper.cpp via `-l`. This was hardcoded to
    /// "en", which silently mis-transcribed anyone dictating in another
    /// language - there was no setting and no indication. "auto" lets whisper
    /// detect it. Defaults to "en" so existing configs and existing behaviour
    /// are unchanged.
    #[serde(default = "default_language")]
    pub language: String,
    /// Words the recogniser should get right - names, jargon, product terms.
    /// Passed to the engine as an initial prompt, which biases recognition
    /// toward these spellings. Wispr calls this a dictionary.
    #[serde(default)]
    pub dictionary: String,
    /// Groq model id. Defaults to whisper-large-v3-turbo (TJ 2026-07-21:
    /// "real-time no latency ... the highest free model ... possible with the
    /// turbo model") - Groq runs turbo at real-time speed on its free tier.
    /// NOTE: an earlier pass moved OFF turbo to full whisper-large-v3 over a
    /// concern that turbo mangled some words. Full large-v3 stays one click away
    /// in the Models tab if accuracy ever matters more than latency.
    #[serde(rename = "cloudModel", default = "default_cloud_model")]
    pub cloud_model: String,
    /// Paste the transcript into the focused app (true, the default) or only
    /// place it on the clipboard for the user to paste themselves.
    #[serde(rename = "autoPaste", default = "default_true")]
    pub auto_paste: bool,
    /// Append a trailing space so back-to-back dictations do not run together.
    #[serde(rename = "trailingSpace", default)]
    pub trailing_space: bool,
    /// Capitalise the first letter of each sentence.
    #[serde(default = "default_true")]
    pub capitalize: bool,
    /// Add a full stop when the transcript ends without terminal punctuation.
    #[serde(rename = "ensurePunctuation", default = "default_true")]
    pub ensure_punctuation: bool,
    /// Overlay window top-left in physical px, persisted after the user
    /// drags the pill (US-002). None = default bottom-center placement.
    /// Owned by Rust (pill_hittest), not the settings UI — save_settings
    /// re-asserts the live values over whatever the frontend sends.
    #[serde(rename = "overlayX", default)]
    pub overlay_x: Option<i32>,
    #[serde(rename = "overlayY", default)]
    pub overlay_y: Option<i32>,
    /// Whether the start/stop pings play on the hotkey path (US-004).
    /// Frontend-owned; round-trips normally through save_settings.
    #[serde(rename = "interactionSounds", default = "default_true")]
    pub interaction_sounds: bool,
    /// Whether the overlay pill window is shown at all (US-004/US-010).
    /// Frontend-owned; main.rs shows/hides the overlay window when this
    /// changes in save_settings.
    #[serde(rename = "overlayVisible", default = "default_true")]
    pub overlay_visible: bool,
    /// Silence gate: audio at or below this level reads as silence (flat
    /// overlay line). Lower = picks up quieter speech.
    #[serde(rename = "voiceSensitivity", default = "default_voice_sensitivity")]
    pub voice_sensitivity: f32,
    /// Below this level counts as a whisper (wave animation); at/above is
    /// normal talking (bars).
    #[serde(rename = "whisperThreshold", default = "default_whisper_threshold")]
    pub whisper_threshold: f32,
    /// Save a LOCAL-ONLY history of transcripts (History tab). Off means no
    /// transcript text is ever persisted (the app's original privacy stance).
    /// Default on per the user's request; nothing written here ever leaves the
    /// machine - it sits in history.json beside config.json.
    #[serde(rename = "historyEnabled", default = "default_true")]
    pub history_enabled: bool,
    /// Auto-delete history entries older than this many days. 0 = keep forever.
    /// Drives the Privacy tab retention control.
    #[serde(rename = "historyRetentionDays", default)]
    pub history_retention_days: u32,
    /// Remove vocalized filler words (um, uh, er, ...) from every transcript.
    /// Local and instant (no network); on by default.
    #[serde(rename = "removeFillers", default = "default_true")]
    pub remove_fillers: bool,
    /// AI auto-edits (Wispr-style "clean output"): a cloud LLM pass that fixes
    /// self-corrections and formatting. Cloud-only, adds ~0.5-1s; on by default.
    #[serde(rename = "autoEdits", default = "default_true")]
    pub auto_edits: bool,
}

fn default_true() -> bool {
    true
}

fn default_microphone() -> String {
    "default".to_string()
}

fn default_engine() -> String {
    // Cloud (Groq) by default: real-time transcription with no local model
    // download. Falls back gracefully to a "add your Groq key" notice if no key
    // is set, and local whisper stays selectable for offline use.
    "cloud".to_string()
}

fn default_whisper_model() -> String {
    "small".to_string()
}

fn default_recording_mode() -> String {
    "auto".to_string()
}

fn default_hotkey() -> String {
    "CmdOrCtrl+Shift+Space".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_cloud_model() -> String {
    "whisper-large-v3-turbo".to_string()
}

fn default_voice_sensitivity() -> f32 {
    0.035
}

fn default_whisper_threshold() -> f32 {
    0.5
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            microphone: "default".to_string(),
            engine: "cloud".to_string(),
            whisper_model: "small".to_string(),
            groq_api_key: String::new(),
            recording_mode: "auto".to_string(),
            hotkey: "CmdOrCtrl+Shift+Space".to_string(),
            language: "en".to_string(),
            dictionary: String::new(),
            cloud_model: "whisper-large-v3-turbo".to_string(),
            auto_paste: true,
            trailing_space: false,
            capitalize: true,
            ensure_punctuation: true,
            overlay_x: None,
            overlay_y: None,
            interaction_sounds: true,
            overlay_visible: true,
            voice_sensitivity: 0.035,
            whisper_threshold: 0.5,
            history_enabled: true,
            history_retention_days: 0,
            remove_fillers: true,
            auto_edits: true,
        }
    }
}

impl Settings {
    pub fn config_path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("config.json")
    }

    /// Path a config that failed to parse is copied aside to before falling
    /// back to defaults (FIX 4), so the user's Groq key and other settings
    /// are recoverable from disk rather than silently destroyed.
    pub fn corrupt_backup_path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("config.json.corrupt")
    }

    pub fn load(app_dir: &PathBuf) -> Self {
        let path = Self::config_path(app_dir);
        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(settings) => settings,
                Err(_) => {
                    // Preserve the unparseable file rather than letting the
                    // fallback-to-defaults silently discard it - the user's
                    // API key and other settings may still be recoverable
                    // from it by hand even though this build couldn't parse
                    // it.
                    let _ = fs::copy(&path, Self::corrupt_backup_path(app_dir));
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    /// Write via a temp file + rename so a process interrupted mid-write
    /// (crash, kill, power loss) can never leave `config.json` truncated -
    /// the rename is atomic, so readers only ever see the old complete file
    /// or the new complete file, never a partial one.
    pub fn save(&self, app_dir: &PathBuf) -> Result<(), String> {
        let path = Self::config_path(app_dir);
        let tmp_path = app_dir.join("config.json.tmp");
        fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        fs::rename(&tmp_path, &path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.microphone, "default");
        assert_eq!(settings.engine, "cloud");
        assert_eq!(settings.whisper_model, "small");
        assert_eq!(settings.groq_api_key, "");
        assert_eq!(settings.recording_mode, "auto");
        assert_eq!(settings.hotkey, "CmdOrCtrl+Shift+Space");
    }

    #[test]
    fn test_save_and_load() {
        let dir = temp_dir().join("demod_test_settings");
        let _ = fs::remove_dir_all(&dir);

        let mut settings = Settings::default();
        settings.engine = "cloud".to_string();
        settings.groq_api_key = "test-key-123".to_string();

        settings.save(&dir).unwrap();
        let loaded = Settings::load(&dir);

        assert_eq!(loaded.engine, "cloud");
        assert_eq!(loaded.groq_api_key, "test-key-123");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = temp_dir().join("demod_test_missing");
        let _ = fs::remove_dir_all(&dir);
        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn test_load_pre_overlay_pos_config_gives_none() {
        // Config written before US-002 has no overlayX/overlayY keys.
        let dir = temp_dir().join("demod_test_no_overlay_pos");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.json"),
            r#"{"microphone":"default","engine":"local","whisperModel":"small","groqApiKey":"","recordingMode":"toggle","hotkey":"CmdOrCtrl+Shift+Space"}"#,
        )
        .unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings.overlay_x, None);
        assert_eq!(settings.overlay_y, None);
        assert_eq!(settings.hotkey, "CmdOrCtrl+Shift+Space");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_pre_us004_config_gives_true_defaults() {
        // Config written before US-004 has no interactionSounds/overlayVisible keys.
        let dir = temp_dir().join("demod_test_no_sounds_visible");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.json"),
            r#"{"microphone":"default","engine":"local","whisperModel":"small","groqApiKey":"","recordingMode":"toggle","hotkey":"CmdOrCtrl+Shift+Space"}"#,
        )
        .unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings.interaction_sounds, true);
        assert_eq!(settings.overlay_visible, true);
        // US-005: the new "auto" default must not silently override an
        // explicit recordingMode saved by a pre-US-005 build.
        assert_eq!(settings.recording_mode, "toggle");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_interaction_sounds_and_overlay_visible_roundtrip() {
        let dir = temp_dir().join("demod_test_sounds_visible_roundtrip");
        let _ = fs::remove_dir_all(&dir);

        let mut settings = Settings::default();
        settings.interaction_sounds = false;
        settings.overlay_visible = false;

        settings.save(&dir).unwrap();
        let loaded = Settings::load(&dir);

        assert_eq!(loaded.interaction_sounds, false);
        assert_eq!(loaded.overlay_visible, false);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_overlay_pos_roundtrip() {
        let dir = temp_dir().join("demod_test_overlay_pos");
        let _ = fs::remove_dir_all(&dir);

        let mut settings = Settings::default();
        settings.overlay_x = Some(-120);
        settings.overlay_y = Some(980);

        settings.save(&dir).unwrap();
        let loaded = Settings::load(&dir);

        assert_eq!(loaded.overlay_x, Some(-120));
        assert_eq!(loaded.overlay_y, Some(980));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_corrupt_json_returns_default() {
        let dir = temp_dir().join("demod_test_corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.json"), "not json").unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());

        let _ = fs::remove_dir_all(&dir);
    }

    // FIX 4 -----------------------------------------------------------

    #[test]
    fn test_load_corrupt_json_preserves_file_for_recovery() {
        // The user's API key must be recoverable even when this build can't
        // parse the file - falling back to defaults must not destroy it.
        let dir = temp_dir().join("demod_test_corrupt_backup");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.json"), r#"{"groqApiKey": "unterminated"#).unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());

        let backup = fs::read_to_string(Settings::corrupt_backup_path(&dir)).unwrap();
        assert!(backup.contains("unterminated"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_partial_json_with_only_new_fields_recovers_original_fields() {
        // A config missing the six ORIGINAL fields entirely (not just the
        // ones added later) must still parse, defaulting only what's
        // missing, rather than the whole object failing to deserialize and
        // falling back to Settings::default() (which would discard, e.g., a
        // groqApiKey that WAS present under a typo'd key elsewhere in a real
        // partial write).
        let dir = temp_dir().join("demod_test_partial_original_fields");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.json"),
            r#"{"trailingSpace": true}"#,
        )
        .unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings.microphone, "default");
        assert_eq!(settings.engine, "cloud");
        assert_eq!(settings.whisper_model, "small");
        assert_eq!(settings.groq_api_key, "");
        assert_eq!(settings.recording_mode, "auto");
        assert_eq!(settings.hotkey, "CmdOrCtrl+Shift+Space");
        assert_eq!(settings.trailing_space, true);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_is_atomic_no_tmp_file_left_behind() {
        let dir = temp_dir().join("demod_test_atomic_save");
        let _ = fs::remove_dir_all(&dir);

        let settings = Settings::default();
        settings.save(&dir).unwrap();

        assert!(dir.join("config.json").exists());
        assert!(!dir.join("config.json.tmp").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_overwrites_previous_config_completely() {
        // Guards the atomic write path itself: after a second save, loading
        // must reflect only the SECOND settings object, not a merge/partial
        // leftover of the first.
        let dir = temp_dir().join("demod_test_atomic_save_overwrite");
        let _ = fs::remove_dir_all(&dir);

        let mut first = Settings::default();
        first.groq_api_key = "first-key".to_string();
        first.save(&dir).unwrap();

        let mut second = Settings::default();
        second.groq_api_key = "second-key".to_string();
        second.save(&dir).unwrap();

        let loaded = Settings::load(&dir);
        assert_eq!(loaded.groq_api_key, "second-key");

        let _ = fs::remove_dir_all(&dir);
    }
}
