#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

use typr_lib::audio;
use typr_lib::downloader;
use typr_lib::paste::{self, PasteOutcome};
use typr_lib::recorder::{Recorder, RecordingState};
use typr_lib::settings::Settings;
use typr_lib::transcribe_local;

mod pill_hittest;
mod sounds;

/// Fixed-size transparent overlay host (Wispr "Status" window pattern),
/// logical px. The OS window is never resized; the pill morphs inside it.
const OVERLAY_W: f64 = 440.0;
const OVERLAY_H: f64 = 160.0;

/// US-005 "auto" recording mode gesture state, one enum behind one lock so
/// the read-decide-mutate step for a Pressed/Released pair is a single
/// atomic transaction (two independently-locked fields would let a fast
/// second tap observe an in-between combination that never actually
/// occurred in wall-clock order).
enum HotkeyGesture {
    Idle,
    /// A hotkey Pressed started a fresh recording at this instant; the
    /// matching Released classifies tap (<300ms) vs hold.
    Pressed(Instant),
    /// A tap-classified Released left recording running hands-free; the
    /// next Pressed is the second tap that stops it.
    HandsFree,
}

struct AppState {
    recorder: Recorder,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
    /// Persistent whisper-server sidecar (keeps the model loaded in RAM so
    /// each dictation skips the ~1.5s model load). None when engine is cloud
    /// or no model is downloaded yet.
    whisper_server: Mutex<Option<CommandChild>>,
    hotkey_gesture: Mutex<HotkeyGesture>,
}

/// (Re)start the persistent whisper-server for the current settings.
/// Kills any prior instance first. No-op (server stays down) for cloud
/// engine or when the model file is missing — transcribe_local falls back
/// to the one-shot CLI in that case anyway.
fn respawn_whisper_server(app: &tauri::AppHandle, state: &AppState) {
    if let Some(old) = state.whisper_server.lock().unwrap().take() {
        let _ = old.kill();
    }

    let settings = state.settings.lock().unwrap().clone();
    if settings.engine != "local" {
        println!("[Typr] Engine is cloud — whisper-server not started");
        return;
    }
    let model_path = state
        .app_dir
        .join(transcribe_local::model_filename(&settings.whisper_model));
    if !model_path.exists() {
        println!("[Typr] Model not downloaded — whisper-server not started");
        return;
    }

    let threads = transcribe_local::whisper_threads().to_string();
    let port = transcribe_local::SERVER_PORT.to_string();
    match app.shell().sidecar("whisper-server") {
        Ok(cmd) => {
            match cmd
                .args([
                    "-m",
                    model_path.to_str().unwrap(),
                    "--host",
                    "127.0.0.1",
                    "--port",
                    &port,
                    "-t",
                    &threads,
                ])
                .spawn()
            {
                Ok((_rx, child)) => {
                    println!("[Typr] whisper-server started (pid {})", child.pid());
                    *state.whisper_server.lock().unwrap() = Some(child);
                }
                Err(e) => eprintln!("[Typr] Failed to spawn whisper-server: {}", e),
            }
        }
        Err(e) => eprintln!("[Typr] whisper-server sidecar not found: {}", e),
    }
}

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.typr.app")
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    state: State<AppState>,
    mut settings: Settings,
) -> Result<(), String> {
    let (old_engine, old_model, old_overlay_visible) = {
        let s = state.settings.lock().unwrap();
        // The overlay position is owned by the drag handler (pill_hittest),
        // not the settings UI; re-assert the live values so a stale frontend
        // snapshot can't revert a dragged pill.
        settings.overlay_x = s.overlay_x;
        settings.overlay_y = s.overlay_y;
        (s.engine.clone(), s.whisper_model.clone(), s.overlay_visible)
    };
    settings.save(&state.app_dir)?;
    let engine_changed = old_engine != settings.engine || old_model != settings.whisper_model;
    let overlay_visible_changed = old_overlay_visible != settings.overlay_visible;
    let new_overlay_visible = settings.overlay_visible;
    *state.settings.lock().unwrap() = settings;
    if engine_changed {
        respawn_whisper_server(&app, state.inner());
    }
    if overlay_visible_changed {
        if let Some(overlay) = app.get_webview_window("overlay") {
            let result = if new_overlay_visible {
                overlay.show()
            } else {
                overlay.hide()
            };
            if let Err(e) = result {
                eprintln!("[Typr] Failed to toggle overlay visibility: {}", e);
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn list_microphones() -> Vec<audio::MicDevice> {
    audio::list_microphones()
}

#[tauri::command]
fn get_recording_state(state: State<AppState>) -> RecordingState {
    state.recorder.get_state()
}

#[tauri::command]
fn check_model_downloaded(state: State<AppState>, model_size: String) -> bool {
    let model_file = transcribe_local::model_filename(&model_size);
    state.app_dir.join(&model_file).exists()
}

#[tauri::command]
async fn download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_size: String,
) -> Result<(), String> {
    let url = transcribe_local::model_download_url(&model_size);
    let model_file = transcribe_local::model_filename(&model_size);
    let dest = state.app_dir.join(&model_file);
    downloader::download_model(app.clone(), &url, &dest).await?;
    // Model just became available — warm up the persistent server for it.
    respawn_whisper_server(&app, state.inner());
    Ok(())
}

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    do_toggle_recording(&app, &state).await
}

/// Shared logic for toggle recording, used by both the Tauri command and hotkey handler.
async fn do_toggle_recording(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<String, String> {
    let current_state = state.recorder.get_state();
    match current_state {
        RecordingState::Ready => {
            let mic = state.settings.lock().unwrap().microphone.clone();
            state.recorder.start_recording(app, &mic)?;
            Ok("recording".to_string())
        }
        RecordingState::Recording => {
            let settings = state.settings.lock().unwrap().clone();
            let result = state
                .recorder
                .stop_and_transcribe(app, &settings, &state.app_dir)
                .await?;
            Ok(result)
        }
        RecordingState::Transcribing => {
            Err("Currently transcribing, please wait".to_string())
        }
    }
}

/// Same one-line eval pattern recorder.rs uses for the pill's data-state
/// (see recorder.rs::set_overlay_state) — kept as a small local duplicate
/// here rather than exporting recorder's private helper, since recorder.rs
/// is under separate review right now.
fn set_overlay_pill_state(app: &tauri::AppHandle, name: &str) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let js = format!("document.getElementById('mic').dataset.state = '{}';", name);
        let _ = overlay.eval(&js);
    }
}

/// US-009 quick-menu item: "Hide for 1 hour". Hides the overlay window (the
/// hotkey keeps working while hidden — recording never depends on overlay
/// visibility) and un-hides it after a single tokio sleep, no polling. If the
/// user has separately turned the overlay off in Settings while it was
/// hidden, respect that and stay hidden instead of popping back up.
#[tauri::command]
fn menu_hide_hour(app: tauri::AppHandle) -> Result<(), String> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())?;
    overlay.hide().map_err(|e| e.to_string())?;

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        let state = handle.state::<AppState>();
        let overlay_visible = state.settings.lock().unwrap().overlay_visible;
        if overlay_visible {
            if let Some(overlay) = handle.get_webview_window("overlay") {
                if let Err(e) = overlay.show() {
                    eprintln!("[Typr] Failed to re-show overlay after hide-for-1-hour: {}", e);
                }
            }
        }
    });

    Ok(())
}

/// US-009 quick-menu item: "Paste last transcript". Reuses the exact US-007
/// paste path (capture_foreground_window + paste_text with the same
/// clipboard-save/restore + focus-guard behavior) rather than a second paste
/// implementation. If there is no last transcript yet, this is a real no-op
/// (nothing to paste), not a fake/dead button.
#[tauri::command]
fn menu_paste_last(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    let Some(text) = state.recorder.get_last_transcript() else {
        return Err("No transcript yet".to_string());
    };
    let target = paste::capture_foreground_window();
    match paste::paste_text(&text, target) {
        PasteOutcome::Pasted => {
            set_overlay_pill_state(&app, "done");
            Ok(())
        }
        PasteOutcome::Fallback => {
            set_overlay_pill_state(&app, "paste-fallback");
            Ok(())
        }
    }
}

/// US-009 quick-menu item: "Settings". Shows and focuses the existing main
/// settings window (default label "main" — the only window() entry in
/// tauri.conf.json).
#[tauri::command]
fn menu_open_settings(app: tauri::AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    main.show().map_err(|e| e.to_string())?;
    main.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// US-009 quick-menu item: "Quit". app.exit() drives the existing
/// RunEvent::Exit handler in main(), which already kills the whisper-server
/// sidecar and uninstalls the pill hit-test hook — no duplicate cleanup here.
#[tauri::command]
fn menu_quit(app: tauri::AppHandle) {
    app.exit(0);
}

/// US-009: overlay.html's openMenu/closeMenu report the quick menu's open
/// state so pill_hittest.rs can widen the pill's OS-level hit rect while the
/// menu is open (data-menu="open") - otherwise the pointer moving off the
/// narrow pill rect and onto the menu above it would flip the overlay back
/// to click-through and the menu would become unclickable / dismiss itself.
#[tauri::command]
fn set_menu_open(open: bool) {
    pill_hittest::set_menu_open(open);
}

fn main() {
    let app_dir = get_app_dir();
    let settings = Settings::load(&app_dir);
    let initial_hotkey = settings.hotkey.clone();
    let saved_overlay_pos = (settings.overlay_x, settings.overlay_y);
    let initial_overlay_visible = settings.overlay_visible;

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            recorder: Recorder::new(),
            settings: Mutex::new(settings),
            app_dir,
            whisper_server: Mutex::new(None),
            hotkey_gesture: Mutex::new(HotkeyGesture::Idle),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            get_recording_state,
            check_model_downloaded,
            download_model,
            toggle_recording,
            menu_hide_hour,
            menu_paste_last,
            menu_open_settings,
            menu_quit,
            set_menu_open,
        ])
        .setup(move |app| {
            // Create the overlay window: a Wispr-style status pill, bottom-center,
            // always on top. Draggable via data-tauri-drag-region in overlay.html.
            // FIXED-SIZE transparent host (Wispr "Status" window pattern): the
            // visible pill morphs entirely inside the webview via data-state
            // CSS; the OS window is never resized or repositioned per state
            // (per-state window resizing = flicker + WM churn).
            let monitor = app.primary_monitor().ok().flatten();
            let (x, y) = if let Some(m) = monitor {
                let size = m.size();
                let scale = m.scale_factor();
                let logical_w = size.width as f64 / scale;
                let logical_h = size.height as f64 / scale;
                (
                    (logical_w - OVERLAY_W) / 2.0,
                    // Pill is bottom-anchored inside the host; this puts it
                    // just above the taskbar.
                    logical_h - OVERLAY_H - 56.0,
                )
            } else {
                (740.0, 864.0)
            };

            let overlay = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("src/overlay.html".into()),
            )
            .title("")
            .inner_size(OVERLAY_W, OVERLAY_H)
            .position(x, y)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .shadow(false)
            .build();

            match overlay {
                Ok(overlay) => {
                    println!("[Typr] Overlay window created");
                    // Reapply the user's dragged pill position (clamped to
                    // the work area) before the first paint.
                    if let (Some(px), Some(py)) = saved_overlay_pos {
                        pill_hittest::restore_position(app.handle(), &overlay, px, py);
                    }
                    // Click-through by default; the WH_MOUSE_LL hook flips
                    // interactivity only while the cursor is over the pill.
                    pill_hittest::install(app.handle(), &overlay);

                    if !initial_overlay_visible {
                        if let Err(e) = overlay.hide() {
                            eprintln!("[Typr] Failed to hide overlay: {}", e);
                        }
                    }
                }
                Err(e) => eprintln!("[Typr] Failed to create overlay: {}", e),
            }

            // Warm up the persistent whisper-server so dictation is fast from
            // the first utterance.
            {
                let handle = app.handle();
                let state = handle.state::<AppState>();
                respawn_whisper_server(handle, state.inner());
            }

            let handle = app.handle().clone();

            println!("[Typr] Registering global shortcut: {}", initial_hotkey);

            match app.global_shortcut().on_shortcut(
                initial_hotkey.as_str(),
                move |_app, shortcut, event| {
                    println!("[Typr] Hotkey event: {:?} state={:?}", shortcut, event.state);
                    let handle = handle.clone();
                    let state = handle.state::<AppState>();
                    let (mode, interaction_sounds) = {
                        let s = state.settings.lock().unwrap();
                        (s.recording_mode.clone(), s.interaction_sounds)
                    };
                    println!("[Typr] Recording mode: {}", mode);

                    match event.state {
                        ShortcutState::Pressed => {
                            // US-004: on Pressed, react instantly — set overlay
                            // state + fire the start ping synchronously, BEFORE
                            // spawning the async task that does the actual
                            // (heavier) capture start. SND_ASYNC returns
                            // immediately so this adds no latency of its own.
                            let current = handle.state::<AppState>().recorder.get_state();

                            // US-005 "auto" mode, gesture 3: a Pressed while
                            // already hands-free recording is the second tap —
                            // stop and transcribe right away, no keyup needed.
                            // Read-and-clear the gesture under one lock so this
                            // decision is atomic with Released's own read.
                            let second_tap_stop = mode == "auto" && current == RecordingState::Recording && {
                                let mut gesture = state.hotkey_gesture.lock().unwrap();
                                if matches!(*gesture, HotkeyGesture::HandsFree) {
                                    *gesture = HotkeyGesture::Idle;
                                    true
                                } else {
                                    false
                                }
                            };

                            if second_tap_stop {
                                tauri::async_runtime::spawn(async move {
                                    let state = handle.state::<AppState>();
                                    if state.recorder.get_state() == RecordingState::Recording {
                                        sounds::play_stop(interaction_sounds);
                                        let settings = state.settings.lock().unwrap().clone();
                                        match state.recorder.stop_and_transcribe(
                                            &handle,
                                            &settings,
                                            &state.app_dir,
                                        ).await {
                                            Ok(result) => println!("[Typr] Transcription: {}", result),
                                            Err(e) => eprintln!("[Typr] Transcription error: {}", e),
                                        }
                                    }
                                });
                            } else {
                                let will_start = matches!(mode.as_str(), "toggle" | "push-to-talk" | "auto")
                                    && current == RecordingState::Ready;
                                if will_start {
                                    if let Some(overlay) = handle.get_webview_window("overlay") {
                                        let _ = overlay.eval(
                                            "document.getElementById('mic').dataset.state = 'listening';",
                                        );
                                    }
                                    sounds::play_start(interaction_sounds);
                                    if mode == "auto" {
                                        // US-005 gestures 1/2: keydown always starts
                                        // capture; record when, so the matching
                                        // Released can classify tap (<300ms) vs hold.
                                        *state.hotkey_gesture.lock().unwrap() =
                                            HotkeyGesture::Pressed(Instant::now());
                                    }
                                }

                                tauri::async_runtime::spawn(async move {
                                    let state = handle.state::<AppState>();
                                    match mode.as_str() {
                                        "toggle" => {
                                            println!("[Typr] Toggle mode: calling do_toggle_recording");
                                            if state.recorder.get_state() == RecordingState::Recording {
                                                sounds::play_stop(interaction_sounds);
                                            }
                                            match do_toggle_recording(&handle, state.inner()).await {
                                                Ok(result) => println!("[Typr] Toggle result: {}", result),
                                                Err(e) => eprintln!("[Typr] Toggle error: {}", e),
                                            }
                                        }
                                        "push-to-talk" | "auto" => {
                                            let current = state.recorder.get_state();
                                            println!("[Typr] {} mode, current state: {:?}", mode, current);
                                            if current == RecordingState::Ready {
                                                let mic = state
                                                    .settings
                                                    .lock()
                                                    .unwrap()
                                                    .microphone
                                                    .clone();
                                                match state.recorder.start_recording(&handle, &mic) {
                                                    Ok(_) => println!("[Typr] Recording started"),
                                                    Err(e) => eprintln!("[Typr] Start recording error: {}", e),
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                });
                            }
                        }
                        ShortcutState::Released => {
                            if mode == "push-to-talk" {
                                tauri::async_runtime::spawn(async move {
                                    let state = handle.state::<AppState>();
                                    let current = state.recorder.get_state();
                                    if current == RecordingState::Recording {
                                        // Finalize/stop point — ping before the
                                        // (heavier) transcription is kicked off.
                                        sounds::play_stop(interaction_sounds);
                                        let settings =
                                            state.settings.lock().unwrap().clone();
                                        match state.recorder.stop_and_transcribe(
                                            &handle,
                                            &settings,
                                            &state.app_dir,
                                        ).await {
                                            Ok(result) => println!("[Typr] Transcription: {}", result),
                                            Err(e) => eprintln!("[Typr] Transcription error: {}", e),
                                        }
                                    }
                                });
                            } else if mode == "auto" {
                                // US-005 gestures 1/2: classify this Released
                                // against the Pressed that started it, under
                                // the same lock second-tap-stop reads — a
                                // fast second Pressed can never observe this
                                // mid-classification. Idle here means it
                                // follows a second-tap stop already finalized
                                // on the Pressed edge above — nothing to do.
                                let is_hold = {
                                    let mut gesture = state.hotkey_gesture.lock().unwrap();
                                    match *gesture {
                                        HotkeyGesture::Pressed(pressed_at) => {
                                            let tap = pressed_at.elapsed() < Duration::from_millis(300);
                                            *gesture = if tap { HotkeyGesture::HandsFree } else { HotkeyGesture::Idle };
                                            Some(!tap)
                                        }
                                        _ => None,
                                    }
                                };
                                if let Some(true) = is_hold {
                                    // Hold: push-to-talk semantics — stop and
                                    // transcribe now.
                                    tauri::async_runtime::spawn(async move {
                                        let state = handle.state::<AppState>();
                                        if state.recorder.get_state() == RecordingState::Recording {
                                            sounds::play_stop(interaction_sounds);
                                            let settings =
                                                state.settings.lock().unwrap().clone();
                                            match state.recorder.stop_and_transcribe(
                                                &handle,
                                                &settings,
                                                &state.app_dir,
                                            ).await {
                                                Ok(result) => println!("[Typr] Transcription: {}", result),
                                                Err(e) => eprintln!("[Typr] Transcription error: {}", e),
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                },
            ) {
                Ok(_) => println!("[Typr] Global shortcut registered successfully"),
                Err(e) => eprintln!("[Typr] ERROR: Failed to register global shortcut: {}", e),
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                pill_hittest::uninstall();
                // Don't leave the whisper-server orphaned.
                let state = app.state::<AppState>();
                let child = state.whisper_server.lock().unwrap().take();
                if let Some(child) = child {
                    let _ = child.kill();
                }
            }
        });
}
