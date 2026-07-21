use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState as HotkeyState};

use crate::audio::AudioRecorder;
use crate::cleanup::{cleanup_text, CleanupOptions};
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

/// US-008: dictations that finish under this budget never show the
/// "processing" pill at all - only the status dot flips (starting -> done)
/// while the pill stays on whatever it was already showing.
const PROCESSING_REVEAL_MS: u64 = 300;
/// US-008: how long the "done" tick holds before relaxing back to idle.
const DONE_HOLD_MS: u64 = 400;
/// Hard ceiling on a single transcription. The 300ms budget above only decides
/// when the processing pill appears; it re-awaits the SAME future afterwards,
/// so without this a hung engine or stalled network never returns and the
/// recorder stays in Transcribing forever - the same permanently-dead-hotkey
/// failure as the early-return bug, just reached slowly instead of quickly.
const TRANSCRIBE_TIMEOUT_SECS: u64 = 180;

/// Deletes the recorded WAV when dropped, regardless of which
/// `stop_and_transcribe` exit path (success, engine error, timeout) runs -
/// see its construction site for why this replaced a single success-only
/// `remove_file` call.
struct TempWavGuard(PathBuf);

impl Drop for TempWavGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

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

/// What an Esc press should do, decided purely from timing + current guard
/// state (no locks, no I/O) so the rule itself - not just "it compiled" -
/// can be unit tested. Mirrors the US-006 30s cancel guard exactly: under
/// the guard, Esc always discards instantly; past the guard, the first Esc
/// arms a confirm-discard prompt and a second Esc (while still armed)
/// discards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscAction {
    /// Discard the recording immediately.
    Discard,
    /// Arm the confirm-discard prompt and start its auto-revert timer.
    ArmConfirmDiscard,
}

/// Pure decision for a single Esc press. `elapsed` is how long the current
/// recording has been running; `already_armed` is whether a confirm-discard
/// prompt from an earlier Esc is still active.
fn decide_esc_action(elapsed: Duration, already_armed: bool) -> EscAction {
    let past_guard = elapsed >= Duration::from_secs(CANCEL_GUARD_SECS);
    if !past_guard || already_armed {
        EscAction::Discard
    } else {
        EscAction::ArmConfirmDiscard
    }
}

/// Whether a pending auto-revert timer (armed at `armed_at`) is still the
/// current one. A fresher Esc (a new arm, or a discard that cleared the
/// guard) between the timer starting and firing must make this false, so
/// the stale timer does nothing instead of clobbering newer state.
fn is_current_confirm_discard(guard: CancelGuard, armed_at: Instant) -> bool {
    matches!(guard, CancelGuard::ConfirmDiscard(t) if t == armed_at)
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

/// US-008: the corner status dot is a second, independent signal from the
/// pill layer - driven via emit_to (overlay-scoped listener, see
/// overlay.html) rather than eval, matching the existing audio-level event.
/// payload is one of 'starting' | 'processing' | 'done' | 'off'.
fn set_status_dot(app: &AppHandle, dot: &str) {
    let _ = app.emit_to("overlay", "status-dot", dot);
}

/// Fixed, actionable message for a `start_recording` failure reached from the
/// hotkey path (main.rs). Never the raw engine error (e.g. "Microphone 'X'
/// not found", "Failed to get default input config: ...") - that string can
/// vary per device and is not meant for the webview; a mic-unavailable
/// message the user can act on is.
pub const MIC_START_FAILURE_MESSAGE: &str =
    "Couldn't start recording: the microphone is unavailable or in use by another app. Check the selected microphone in Demod settings.";

/// Map an engine error to a fixed, actionable message.
///
/// The raw error is deliberately NOT shown: it can embody engine output or a
/// Groq response body, and only fixed strings may reach the webview (see
/// `fail_to_ready`). But a bare "Transcription failed" throws away the one
/// thing the user could act on, and the engines already know exactly what is
/// wrong. The commonest case is the very first dictation on a fresh install:
/// the default engine is local and no model ships with the app, so that first
/// attempt fails and without this says nothing about how to fix it.
fn transcribe_failure_message(err: &str) -> &'static str {
    let e = err.to_ascii_lowercase();
    if e.contains("model not found") || e.contains("download a model") {
        // Deliberately "the selected model", not "no model": the error is per
        // model path, and a real config here had ggml-small.bin downloaded
        // while the selected model was medium. "No speech model yet" would
        // have been simply false, and would send the user looking for the
        // wrong problem.
        "That speech model isn't downloaded. Open Demod settings to get it."
    } else if e.contains("api key") {
        "Groq needs an API key. Add one in Demod settings."
    } else if e.contains("groq api error") {
        "The cloud transcription service rejected that request."
    } else if e.contains("timed out") || e.contains("connect") || e.contains("dns") {
        "Could not reach the transcription service."
    } else {
        "Transcription failed."
    }
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
        eprintln!("[Demod] Failed to unregister cancel (Esc) shortcut: {}", e);
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

            if decide_esc_action(started_at.elapsed(), already_armed) == EscAction::Discard {
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
                if is_current_confirm_discard(*guard, armed_at) {
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
            eprintln!("[Demod] Failed to register cancel (Esc) shortcut: {}", e);
            crate::applog::log_event_default("hotkey", &format!("Esc cancel shortcut registration failed: {}", e));
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

    /// End an in-progress recording that no longer has a live owner - used
    /// when the global hotkey is rebound (`set_hotkey` in main.rs) while the
    /// user is still physically holding the OLD combination. Once the old
    /// accelerator is unregistered its Released event can never arrive, so
    /// without this the recorder is stranded in Recording forever (the same
    /// permanently-dead-hotkey failure class `fail_to_ready` guards against
    /// elsewhere, reached a different way). Cancels rather than transcribes:
    /// a rebind is not a clean stop point, so discarding is the safe outcome.
    /// No-op if nothing is currently recording.
    pub fn cancel_if_recording(&self, app: &AppHandle) {
        cancel_recording(
            app,
            &self.state,
            &self.audio_recorder,
            &self.recording_started_at,
            &self.cancel_guard,
        );
    }

    /// Return to Ready after a dictation that could not finish, and tell the
    /// user what happened.
    ///
    /// Every fallible step in stop_and_transcribe used `?`, which returned
    /// early while the state was still Transcribing - and start_recording
    /// accepts ONLY Ready. So a single failure (a missing or corrupt model, an
    /// empty Groq key, a network blip, a mic unplugged mid-sentence) left the
    /// hotkey permanently rejecting every press, the pill frozen mid
    /// animation, and nothing on screen explaining why. The app had to be
    /// restarted to dictate again.
    ///
    /// `user_message` must be a fixed string, never raw engine output: it is
    /// shown to the user, and the underlying error goes to the log instead.
    pub fn fail_to_ready(&self, app: &AppHandle, user_message: &str) {
        {
            let mut state = self.state.lock().unwrap();
            *state = RecordingState::Ready;
        }
        let _ = app.emit("recording-state", RecordingState::Ready);
        set_status_dot(app, "off");
        // Sent as an event payload (assigned with textContent in the webview)
        // rather than built into an eval string, so nothing here is a script
        // injection surface.
        let _ = app.emit_to("overlay", "notice", user_message.to_string());
    }

    pub async fn stop_and_transcribe(
        &self,
        app: &AppHandle,
        settings: &Settings,
        app_dir: &PathBuf,
    ) -> Result<String, String> {
        // Stop recording. Internal state flips to Transcribing immediately
        // (other commands gate on it), but the overlay PILL stays as-is for
        // now - US-008 only reveals the "processing" layer if transcription
        // is still running past PROCESSING_REVEAL_MS, so short dictations
        // never flicker through it. The status dot is a separate, earlier
        // signal: it goes yellow ("starting") right away.
        {
            let mut state = self.state.lock().unwrap();
            if *state != RecordingState::Recording {
                return Err("Not currently recording".to_string());
            }
            *state = RecordingState::Transcribing;
            let _ = app.emit("recording-state", RecordingState::Transcribing);
            set_status_dot(app, "starting");
        }
        // US-007 focus guard: capture the foreground window now, at the
        // moment capture stops, not after the slow transcription step below
        // - by paste time the user may have switched apps, and pasting
        // there instead of where they dictated is the bug this guards
        // against.
        let paste_target = capture_foreground_window();
        // Capture the recording length for the history entry BEFORE the timer
        // is cleared just below. Instant is monotonic; we only take the delta.
        let recording_duration_ms = self
            .recording_started_at
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_millis() as u64);
        // Leaving Recording: the Esc cancel guard no longer applies.
        *self.recording_started_at.lock().unwrap() = None;
        *self.cancel_guard.lock().unwrap() = CancelGuard::None;
        if let Err(e) = app.global_shortcut().unregister(CANCEL_SHORTCUT) {
            eprintln!("[Demod] Failed to unregister cancel (Esc) shortcut: {}", e);
        }

        let temp_path = app_dir.join("temp_recording.wav");

        // Save audio
        {
            let result = {
                let mut recorder = self.audio_recorder.lock().unwrap();
                recorder.stop_and_save(&temp_path)
            };
            if let Err(e) = result {
                self.fail_to_ready(app, "Could not save the recording");
                return Err(e);
            }
        }
        // Guarantee the WAV is deleted on every exit from here on - success,
        // engine error, or timeout. Before this guard, only the success path
        // removed it (see the old single `remove_file` call further down),
        // so a failed or timed-out dictation left the user's recorded audio
        // sitting in the app data dir indefinitely: a leak, and a privacy
        // problem since it is exactly what they said.
        let _temp_wav_guard = TempWavGuard(temp_path.clone());

        // Transcribe. Race the actual engine call against the 300ms
        // "instant feel" budget (US-008, research secs 1 & 7.5): if it
        // finishes within the budget, the processing pill is never shown at
        // all; only past the budget do we reveal it (and flip the dot to
        // "processing" blue) before awaiting the rest of the same future.
        let engine = settings.engine.clone();
        let whisper_model = settings.whisper_model.clone();
        let groq_api_key = settings.groq_api_key.clone();
        let language = settings.language.clone();
        let dictionary = settings.dictionary.clone();
        let cloud_model = settings.cloud_model.clone();
        let temp_path_for_engine = temp_path.clone();
        let app_dir_for_engine = app_dir.clone();
        let app_for_engine = app.clone();
        let transcribe_fut = async move {
            match engine.as_str() {
                "local" => {
                    let model_path =
                        app_dir_for_engine.join(transcribe_local::model_filename(&whisper_model));
                    transcribe_local::transcribe_local(
                        &app_for_engine,
                        &model_path,
                        &temp_path_for_engine,
                        &language,
                        &dictionary,
                    )
                    .await
                }
                "cloud" => {
                    transcribe_groq::transcribe_groq(
                        &groq_api_key,
                        &temp_path_for_engine,
                        &language,
                        &dictionary,
                        &cloud_model,
                    )
                    .await
                }
                other => Err(format!("Unknown engine: {}", other)),
            }
        };
        tokio::pin!(transcribe_fut);
        let raw_text = match tokio::time::timeout(
            Duration::from_millis(PROCESSING_REVEAL_MS),
            &mut transcribe_fut,
        )
        .await
        {
            Ok(result) => match result {
                Ok(text) => text,
                Err(e) => {
                    self.fail_to_ready(app, transcribe_failure_message(&e));
                    return Err(e);
                }
            },
            Err(_elapsed) => {
                update_overlay(app, &RecordingState::Transcribing);
                set_status_dot(app, "processing");
                match tokio::time::timeout(
                    Duration::from_secs(TRANSCRIBE_TIMEOUT_SECS),
                    &mut transcribe_fut,
                )
                .await
                {
                    Ok(Ok(text)) => text,
                    Ok(Err(e)) => {
                        self.fail_to_ready(app, transcribe_failure_message(&e));
                        return Err(e);
                    }
                    Err(_) => {
                        // Never leave the user stranded in Transcribing.
                        self.fail_to_ready(app, "Transcription timed out.");
                        return Err("transcription timed out".to_string());
                    }
                }
            }
        };

        // Clean up text
        // Post-processing is now the user's choice rather than hardcoded.
        // Filler removal (local, instant) on the raw text BEFORE cleanup, so
        // capitalization/punctuation apply to the already-trimmed result.
        let pre_clean = if settings.remove_fillers {
            crate::cleanup::remove_fillers(&raw_text)
        } else {
            raw_text.clone()
        };
        let mut cleaned = cleanup_text(
            &pre_clean,
            &CleanupOptions {
                capitalize: settings.capitalize,
                ensure_punctuation: settings.ensure_punctuation,
                trailing_space: settings.trailing_space,
            },
        );
        // AI auto-edits (Wispr-style "clean output"): a cloud LLM pass that fixes
        // self-corrections and formatting. Cloud-only + best-effort - on any
        // failure/timeout we keep the locally-cleaned text and never fail the
        // dictation over it. Adds ~0.5-1s, which is why it is a user toggle.
        if settings.auto_edits
            && settings.engine == "cloud"
            && !settings.groq_api_key.trim().is_empty()
            && !cleaned.is_empty()
        {
            match transcribe_groq::groq_auto_edit(&settings.groq_api_key, &cleaned).await {
                Ok(edited) if !edited.trim().is_empty() => {
                    cleaned = edited;
                    // The LLM strips a trailing space; re-honour the setting.
                    if settings.trailing_space && !cleaned.ends_with(' ') {
                        cleaned.push(' ');
                    }
                }
                Ok(_) => {}
                Err(e) => eprintln!("[Demod] auto-edit skipped: {}", e),
            }
        }

        // Auto-paste into the window that was focused when capture stopped
        // (US-007). None = nothing to paste, so there is no outcome to report.
        let paste_outcome = if cleaned.is_empty() {
            None
        } else {
            *self.last_transcript.lock().unwrap() = Some(cleaned.clone());
            // Local-only transcript history (ON by default; gated by
            // settings.history_enabled). Best-effort: a history write must never
            // fail or delay a dictation, so errors are logged and swallowed.
            // Nothing here leaves the machine. Serialized via history::with_lock
            // so a concurrent delete/clear command cannot race this append.
            if settings.history_enabled {
                let model = if settings.engine == "cloud" {
                    settings.cloud_model.clone()
                } else {
                    settings.whisper_model.clone()
                };
                let entry_text = cleaned.clone();
                let engine = settings.engine.clone();
                let retention_days = settings.history_retention_days;
                crate::history::with_lock(|| {
                    let mut store = crate::history::HistoryStore::load(app_dir);
                    store.append(entry_text, engine, model, recording_duration_ms);
                    // Enforce the Privacy "auto-delete after N days" retention on
                    // every write (0 = keep forever). Also applied at startup.
                    if retention_days > 0 {
                        store.prune_older_than(retention_days as u128 * 86_400_000);
                    }
                    if let Err(e) = store.save(app_dir) {
                        eprintln!("[Demod] history save failed: {}", e);
                    }
                });
            }
            if settings.auto_paste {
                Some(paste_text(&cleaned, paste_target))
            } else {
                // Copy-only mode: the user asked us not to type into their app,
                // so put it on the clipboard and say so rather than pasting.
                // The write itself can fail (clipboard held by another app,
                // etc.) - that must surface as Failed, never be reported as
                // the same "copied, paste manually" Fallback state when
                // nothing was actually copied.
                if crate::paste::copy_only(&cleaned) {
                    Some(PasteOutcome::Fallback)
                } else {
                    Some(PasteOutcome::Failed)
                }
            }
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
                Some(PasteOutcome::Fallback) => {
                    set_overlay_state(app, "paste-fallback");
                    set_status_dot(app, "off");
                }
                // The transcript was not delivered anywhere - not even the
                // clipboard. Must never fall into the success arm below:
                // that would show the "done" tick for a dictation the user
                // never actually received.
                Some(PasteOutcome::Failed) => {
                    drop(state);
                    self.fail_to_ready(app, "Could not copy the transcript to the clipboard.");
                }
                // US-008: success shows the "done" tick + green dot for
                // DONE_HOLD_MS, then relaxes to idle - deferred via spawn
                // (not a blocking sleep here) so this command returns to the
                // caller immediately; debounced against state so a fresh
                // recording started during the hold isn't clobbered back to
                // idle by the stale timer.
                _ => {
                    set_overlay_state(app, "done");
                    set_status_dot(app, "done");
                    drop(state);
                    let app = app.clone();
                    let state = self.state.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(DONE_HOLD_MS)).await;
                        if *state.lock().unwrap() == RecordingState::Ready {
                            update_overlay(&app, &RecordingState::Ready);
                            set_status_dot(&app, "off");
                        }
                    });
                }
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

    // US-006 Esc cancel guard: decide_esc_action ------------------------

    #[test]
    fn test_esc_under_guard_not_armed_discards_instantly() {
        assert_eq!(
            decide_esc_action(Duration::from_secs(1), false),
            EscAction::Discard
        );
    }

    #[test]
    fn test_esc_under_guard_already_armed_still_discards() {
        // Can't normally happen (the prompt only arms past the guard), but
        // the rule must not accidentally re-arm on it - discard wins.
        assert_eq!(
            decide_esc_action(Duration::from_secs(1), true),
            EscAction::Discard
        );
    }

    #[test]
    fn test_esc_past_guard_first_press_arms_confirm_discard() {
        assert_eq!(
            decide_esc_action(Duration::from_secs(CANCEL_GUARD_SECS + 1), false),
            EscAction::ArmConfirmDiscard
        );
    }

    #[test]
    fn test_esc_past_guard_second_press_discards() {
        assert_eq!(
            decide_esc_action(Duration::from_secs(CANCEL_GUARD_SECS + 1), true),
            EscAction::Discard
        );
    }

    #[test]
    fn test_esc_exactly_at_guard_boundary_counts_as_past_guard() {
        // The real check is `>=`; this pins that boundary so a future
        // off-by-one (e.g. switching to `>`) fails a test instead of only
        // shifting the guard by one instant in production.
        assert_eq!(
            decide_esc_action(Duration::from_secs(CANCEL_GUARD_SECS), false),
            EscAction::ArmConfirmDiscard
        );
    }

    #[test]
    fn test_esc_just_under_guard_boundary_discards() {
        let just_under = Duration::from_secs(CANCEL_GUARD_SECS) - Duration::from_millis(1);
        assert_eq!(decide_esc_action(just_under, false), EscAction::Discard);
    }

    // US-006 auto-revert debounce: is_current_confirm_discard ------------

    #[test]
    fn test_stale_timer_check_true_when_guard_still_matches_armed_at() {
        let armed_at = Instant::now();
        assert!(is_current_confirm_discard(
            CancelGuard::ConfirmDiscard(armed_at),
            armed_at
        ));
    }

    #[test]
    fn test_stale_timer_check_false_when_guard_already_cleared() {
        let armed_at = Instant::now();
        assert!(!is_current_confirm_discard(CancelGuard::None, armed_at));
    }

    #[test]
    fn test_stale_timer_check_false_when_a_fresher_arm_replaced_it() {
        let stale_armed_at = Instant::now();
        std::thread::sleep(Duration::from_millis(2));
        let fresher_armed_at = Instant::now();
        // The guard now holds a newer arm than the timer that's checking -
        // this stale timer must not treat itself as current.
        assert!(!is_current_confirm_discard(
            CancelGuard::ConfirmDiscard(fresher_armed_at),
            stale_armed_at
        ));
    }
}
