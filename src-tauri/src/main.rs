#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

use typr_lib::audio;
use typr_lib::downloader;
use typr_lib::recorder::{Recorder, RecordingState};
use typr_lib::settings::Settings;
use typr_lib::transcribe_local;

struct AppState {
    recorder: Recorder,
    settings: Mutex<Settings>,
    app_dir: PathBuf,
    /// Persistent whisper-server sidecar (keeps the model loaded in RAM so
    /// each dictation skips the ~1.5s model load). None when engine is cloud
    /// or no model is downloaded yet.
    whisper_server: Mutex<Option<CommandChild>>,
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
    settings: Settings,
) -> Result<(), String> {
    let (old_engine, old_model) = {
        let s = state.settings.lock().unwrap();
        (s.engine.clone(), s.whisper_model.clone())
    };
    settings.save(&state.app_dir)?;
    let engine_changed = old_engine != settings.engine || old_model != settings.whisper_model;
    *state.settings.lock().unwrap() = settings;
    if engine_changed {
        respawn_whisper_server(&app, state.inner());
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

fn main() {
    let app_dir = get_app_dir();
    let settings = Settings::load(&app_dir);
    let initial_hotkey = settings.hotkey.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            recorder: Recorder::new(),
            settings: Mutex::new(settings),
            app_dir,
            whisper_server: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            get_recording_state,
            check_model_downloaded,
            download_model,
            toggle_recording,
        ])
        .setup(move |app| {
            // Create the overlay window: a Wispr-style status pill, bottom-center,
            // always on top. Draggable via data-tauri-drag-region in overlay.html.
            // FIXED-SIZE transparent host (Wispr "Status" window pattern): the
            // visible pill morphs entirely inside the webview via data-state
            // CSS; the OS window is never resized or repositioned per state
            // (per-state window resizing = flicker + WM churn).
            const OVERLAY_W: f64 = 440.0;
            const OVERLAY_H: f64 = 160.0;
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
                Ok(_) => println!("[Typr] Overlay window created"),
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
                    let mode = state.settings.lock().unwrap().recording_mode.clone();
                    println!("[Typr] Recording mode: {}", mode);

                    match event.state {
                        ShortcutState::Pressed => {
                            tauri::async_runtime::spawn(async move {
                                let state = handle.state::<AppState>();
                                match mode.as_str() {
                                    "toggle" => {
                                        println!("[Typr] Toggle mode: calling do_toggle_recording");
                                        match do_toggle_recording(&handle, state.inner()).await {
                                            Ok(result) => println!("[Typr] Toggle result: {}", result),
                                            Err(e) => eprintln!("[Typr] Toggle error: {}", e),
                                        }
                                    }
                                    "push-to-talk" => {
                                        let current = state.recorder.get_state();
                                        println!("[Typr] PTT mode, current state: {:?}", current);
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
                        ShortcutState::Released => {
                            if mode == "push-to-talk" {
                                tauri::async_runtime::spawn(async move {
                                    let state = handle.state::<AppState>();
                                    let current = state.recorder.get_state();
                                    if current == RecordingState::Recording {
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
                // Don't leave the whisper-server orphaned.
                let state = app.state::<AppState>();
                let child = state.whisper_server.lock().unwrap().take();
                if let Some(child) = child {
                    let _ = child.kill();
                }
            }
        });
}
