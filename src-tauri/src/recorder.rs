use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState as HotkeyState};

use crate::audio::AudioRecorder;
use crate::cleanup::cleanup_text;
use crate::paste::{capture_foreground_window, paste_text, PasteOutcome};
use crate::settings::Settings;
use crate::transcribe_local;
use crate::transcribe_groq;

/// US-006 Esc cancel guard: under this many seconds of recording, Esc
/// discards instantly; past it, a first Esc arms a confirm-discard prompt
/// that a second Esc (within the confirm window) commits, or that
/// auto-reverts to listening if it times out.
const CANCEL_GUARD_SECS: u64 = 30;
const CONFIRM_DISCARD_SECS: u64 = 3;
const CANCEL_SHORTCUT: &str = "Escape";

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}

/// Whether a past-guard Esc has armed the confirm-discard prompt, and when
/// (used to debounce a stale auto-revert timer against a fresher arm/clear).
#[derive(Clone, Copy, PartialEq)]
enum CancelGuard {
    None,
    ConfirmDiscard(Instant),
}

/// The overlay host window is fixed-size (see main.rs); the pill morphs
/// entirely inside the webview off this data-state attribute. Never resize
/// or reposition the OS window here - that is the flicker Wispr fixed.
fn set_overlay_state(app: &AppHandle, name: &str) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let js = format!("document.getElementById('mic').dataset.state = '{}';", name);
        let _ = overlay.eval(&js);
    }
}

fn update_overlay(app: &AppHandle, state: &RecordingState) {
    let name = match state {
        RecordingState::Ready => "idle",
        RecordingState::Recording => "listening",
        RecordingState::Transcribing => "processing",
    };
    set_overlay_state(app, name);
}

/// Discard the in-progress recording and go back to idle (US-006). Called
/// only from a spawned task, never synchronously from inside the Escape
/// shortcut's dispatch closure - the global-shortcut plugin holds its
/// shortcuts-map mutex for the duration of that closure, and unregister()
/// below re-locks the same mutex, which would self-deadlock on that thread.
fn cancel_recording(
    app: &AppHandle,
    state: &Arc<Mutex<RecordingState>>,
    audio_recorder: &Arc<Mutex<AudioRecorder>>,
    recording_started_at: &Arc<Mutex<Option<Instant>>>,
    cancel_guard: &Arc<Mutex<CancelGuard>>,
) {
    if *state.lock().unwrap() != RecordingState::Recording {
        return;
    }
    audio_recorder.lock().unwrap().cancel();
    *recording_started_at.lock().unwrap() = None;
    *cancel_guard.lock().unwrap() = CancelGuard::None;
    // Unregister BEFORE state flips to Ready: Ready is the sole gate
    // start_recording checks, so a fresh recording (and its own Escape
    // registration) must never be able to start while this session's Escape
    // shortcut might still be mid-teardown - that ordering would risk the
    // new registration losing a race with the stale unregister, leaving Esc
    // silently unregistered for the new recording.
    if let Err(e) = app.global_shortcut().unregister(CANCEL_SHORTCUT) {
        eprintln!("[Typr] Failed to unregister cancel (Esc) shortcut: {}", e);
    }
    *state.lock().unwrap() = RecordingState::Ready;
    let _ = app.emit("recording-state", RecordingState::Ready);
    update_overlay(app, &RecordingState::Ready);
}

pub struct Recorder {
    state: Arc<Mutex<RecordingState>>,
    audio_recorder: Arc<Mutex<AudioRecorder>>,
    /// When the current recording started (US-006 30s guard); None while idle.
    recording_started_at: Arc<Mutex<Option<Instant>>>,
    cancel_guard: Arc<Mutex<CancelGuard>>,
    /// Last successfully transcribed (non-empty) text (US-007), for the
    /// Shift+Alt+Z re-paste shortcut and the quick menu's "Paste last
    /// transcript" item. Written once, at the single site that produces a
    /// transcript, rather than duplicated across every hotkey call site.
    last_transcript: Arc<Mutex<Option<String>>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Ready)),
            audio_recorder: Arc::new(Mutex::new(AudioRecorder::new())),
            recording_started_at: Arc::new(Mutex::new(None)),
            cancel_guard: Arc::new(Mutex::new(CancelGuard::None)),
            last_transcript: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_state(&self) -> RecordingState {
        self.state.lock().unwrap().clone()
    }

    pub fn get_last_transcript(&self) -> Option<String> {
        self.last_transcript.lock().unwrap().clone()
    }

    /// Register the Esc shortcut for the lifetime of one recording (US-006).
    /// Registered only while Recording, unregistered as soon as it ends
    /// (cancel_recording, or the start of stop_and_transcribe) - Esc must
    /// never be globally registered while idle.
    fn arm_cancel_shortcut(&self, app: &AppHandle) {
        let state = self.state.clone();
        let audio_recorder = self.audio_recorder.clone();
        let recording_started_at = self.recording_started_at.clone();
        let cancel_guard = self.cancel_guard.clone();

        let result = app.global_shortcut().on_shortcut(CANCEL_SHORTCUT, move |app, _shortcut, event| {
            if event.state != HotkeyState::Pressed {
                return;
            }
            if *state.lock().unwrap() != RecordingState::Recording {
                return;
            }
            let started_at = match *recording_started_at.lock().unwrap() {
                Some(t) => t,
                None => return,
            };
            let already_armed = matches!(*cancel_guard.lock().unwrap(), CancelGuard::ConfirmDiscard(_));
            let past_guard = started_at.elapsed() >= Duration::from_secs(CANCEL_GUARD_SECS);

            if !past_guard || already_armed {
                // Under the guard: instant discard. Past the guard with the
                // prompt already armed: this is the second Esc - discard.
                // Deferred (see cancel_recording's doc comment).
                let app = app.clone();
                let state = state.clone();
                let audio_recorder = audio_recorder.clone();
                let recording_started_at = recording_started_at.clone();
                let cancel_guard = cancel_guard.clone();
                tauri::async_runtime::spawn(async move {
                    cancel_recording(&app, &state, &audio_recorder, &recording_started_at, &cancel_guard);
                });
                return;
            }

            // First Esc past the guard: arm the confirm-discard prompt and
            // start its 3s auto-revert timer. No polling - a single tokio
            // sleep, debounced against a fresher arm/clear via armed_at.
            let armed_at = Instant::now();
            *cancel_guard.lock().unwrap() = CancelGuard::ConfirmDiscard(armed_at);
            set_overlay_state(app, "confirm-discard");

            let app = app.clone();
            let state = state.clone();
            let cancel_guard = cancel_guard.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(CONFIRM_DISCARD_SECS)).await;
                let mut guard = cancel_guard.lock().unwrap();
                if matches!(*guard, CancelGuard::ConfirmDiscard(t) if t == armed_at) {
                    *guard = CancelGuard::None;
                    drop(guard);
                    // A concurrent second Esc may have already discarded (and
                    // flipped state away from Recording) between the guard
                    // check above and here; only revert the overlay if this
                    // recording is actually still live, so a confirmed
                    // discard can't be visually clobbered back to "listening".
                    if *state.lock().unwrap() == RecordingState::Recording {
                        set_overlay_state(&app, "listening");
                    }
                }
            });
        });

        if let Err(e) = result {
            eprintln!("[Typr] Failed to register cancel (Esc) shortcut: {}", e);
        }
    }

    pub fn start_recording(&self, app: &AppHandle, mic_name: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if *state != RecordingState::Ready {
            return Err("Already recording or transcribing".to_string());
        }

        let mut recorder = self.audio_recorder.lock().unwrap();
        recorder.start(app, mic_name)?;

        *state = RecordingState::Recording;
        let _ = app.emit("recording-state", RecordingState::Recording);
        update_overlay(app, &RecordingState::Recording);

        *self.recording_started_at.lock().unwrap() = Some(Instant::now());
        *self.cancel_guard.lock().unwrap() = CancelGuard::None;
        self.arm_cancel_shortcut(app);

        Ok(())
    }

    pub async fn stop_and_transcribe(
        &self,
        app: &AppHandle,
        settings: &Settings,
        app_dir: &PathBuf,
    ) -> Result<String, String> {
        // Stop recording
        {
            let mut state = self.state.lock().unwrap();
            if *state != RecordingState::Recording {
                return Err("Not currently recording".to_string());
            }
            *state = RecordingState::Transcribing;
            let _ = app.emit("recording-state", RecordingState::Transcribing);
            update_overlay(app, &RecordingState::Transcribing);
        }
        // US-007 focus guard: capture the foreground window now, at the
        // moment capture stops, not after the slow transcription step below
        // - by paste time the user may have switched apps, and pasting
        // there instead of where they dictated is the bug this guards
        // against.
        let paste_target = capture_foreground_window();
        // Leaving Recording: the Esc cancel guard no longer applies.
        *self.recording_started_at.lock().unwrap() = None;
        *self.cancel_guard.lock().unwrap() = CancelGuard::None;
        if let Err(e) = app.global_shortcut().unregister(CANCEL_SHORTCUT) {
            eprintln!("[Typr] Failed to unregister cancel (Esc) shortcut: {}", e);
        }

        let temp_path = app_dir.join("temp_recording.wav");

        // Save audio
        {
            let mut recorder = self.audio_recorder.lock().unwrap();
            recorder.stop_and_save(&temp_path)?;
        }

        // Transcribe
        let raw_text = match settings.engine.as_str() {
            "local" => {
                let model_path = app_dir.join(transcribe_local::model_filename(&settings.whisper_model));
                transcribe_local::transcribe_local(app, &model_path, &temp_path).await?
            }
            "cloud" => {
                transcribe_groq::transcribe_groq(&settings.groq_api_key, &temp_path).await?
            }
            _ => return Err(format!("Unknown engine: {}", settings.engine)),
        };

        // Cleanup temp file
        let _ = std::fs::remove_file(&temp_path);

        // Clean up text
        let cleaned = cleanup_text(&raw_text);

        // Auto-paste into the window that was focused when capture stopped
        // (US-007). None = nothing to paste, so there is no outcome to report.
        let paste_outcome = if cleaned.is_empty() {
            None
        } else {
            *self.last_transcript.lock().unwrap() = Some(cleaned.clone());
            Some(paste_text(&cleaned, paste_target))
        };

        // Reset state
        {
            let mut state = self.state.lock().unwrap();
            *state = RecordingState::Ready;
            let _ = app.emit("recording-state", RecordingState::Ready);
            match paste_outcome {
                // Refocus or the paste itself failed: the transcript is sitting
                // on the clipboard and the overlay says so. Hold that message
                // instead of snapping to idle - idle here would silently discard
                // the only signal the user gets that they must paste manually.
                Some(PasteOutcome::Fallback) => set_overlay_state(app, "paste-fallback"),
                _ => update_overlay(app, &RecordingState::Ready),
            }
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_ready() {
        let recorder = Recorder::new();
        assert_eq!(recorder.get_state(), RecordingState::Ready);
    }
}
