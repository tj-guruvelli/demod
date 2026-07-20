use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub microphone: String,
    pub engine: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "groqApiKey")]
    pub groq_api_key: String,
    /// "auto" (default, US-005 tap/hold gesture model), or the legacy
    /// explicit overrides "toggle" / "push-to-talk".
    #[serde(rename = "recordingMode")]
    pub recording_mode: String,
    pub hotkey: String,
    /// Spoken language passed to whisper.cpp via `-l`. This was hardcoded to
    /// "en", which silently mis-transcribed anyone dictating in another
    /// language - there was no setting and no indication. "auto" lets whisper
    /// detect it. Defaults to "en" so existing configs and existing behaviour
    /// are unchanged.
    #[serde(default = "default_language")]
    pub language: String,
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
}

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            microphone: "default".to_string(),
            engine: "local".to_string(),
            whisper_model: "small".to_string(),
            groq_api_key: String::new(),
            recording_mode: "auto".to_string(),
            hotkey: "CmdOrCtrl+Shift+Space".to_string(),
            language: "en".to_string(),
            overlay_x: None,
            overlay_y: None,
            interaction_sounds: true,
            overlay_visible: true,
        }
    }
}

impl Settings {
    pub fn config_path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("config.json")
    }

    pub fn load(app_dir: &PathBuf) -> Self {
        let path = Self::config_path(app_dir);
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, app_dir: &PathBuf) -> Result<(), String> {
        let path = Self::config_path(app_dir);
        fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
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
        assert_eq!(settings.engine, "local");
        assert_eq!(settings.whisper_model, "small");
        assert_eq!(settings.groq_api_key, "");
        assert_eq!(settings.recording_mode, "auto");
        assert_eq!(settings.hotkey, "CmdOrCtrl+Shift+Space");
    }

    #[test]
    fn test_save_and_load() {
        let dir = temp_dir().join("typr_test_settings");
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
        let dir = temp_dir().join("typr_test_missing");
        let _ = fs::remove_dir_all(&dir);
        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn test_load_pre_overlay_pos_config_gives_none() {
        // Config written before US-002 has no overlayX/overlayY keys.
        let dir = temp_dir().join("typr_test_no_overlay_pos");
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
        let dir = temp_dir().join("typr_test_no_sounds_visible");
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
        let dir = temp_dir().join("typr_test_sounds_visible_roundtrip");
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
        let dir = temp_dir().join("typr_test_overlay_pos");
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
        let dir = temp_dir().join("typr_test_corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.json"), "not json").unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());

        let _ = fs::remove_dir_all(&dir);
    }
}
