# Demod

> A local-first dictation app for Windows. Press a hotkey, talk, get text — anywhere on your desktop.

[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)](https://github.com/tj-guruvelli/demod)
[![Version](https://img.shields.io/badge/version-0.1.0-purple)](https://github.com/tj-guruvelli/demod/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-orange)](https://tauri.app)

---

Demod (short for "demodulator" — the circuit that recovers signal from a carrier wave) is a global-hotkey dictation tool. Hit `Ctrl+Shift+Space`, talk, and your words are typed directly into whatever app is focused. No clipboard juggling. No browser tab. No subscription.

Transcription runs **fully on-device** via whisper.cpp. Groq cloud is available as an optional fast path when you have an API key.

Built with **Tauri v2 + vanilla TypeScript**. No Electron, no Node runtime in production.

---

## Architecture

![Demod Architecture](docs/architecture.png)

See [`docs/demod-architecture.html`](docs/demod-architecture.html) for the interactive SVG diagram.

```
┌─────────────────────────────────────────────────────────────┐
│                        User Desktop                          │
│                                                              │
│  Global Hotkey (Ctrl+Shift+Space)                           │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────────┐     ┌──────────────────────────────────┐   │
│  │  Overlay    │     │         Tauri v2 Shell (Rust)    │   │
│  │  Pill UI    │◄────│  recorder.rs  │  main.rs         │   │
│  │  (WebView2) │     │  audio.rs     │  settings.rs     │   │
│  └─────────────┘     │  sounds.rs    │  autostart.rs    │   │
│                      └───────┬───────┴──────────────────┘   │
│                              │                               │
│              ┌───────────────┴───────────────┐              │
│              │                               │              │
│              ▼                               ▼              │
│   ┌──────────────────┐           ┌───────────────────────┐  │
│   │  Local Engine    │           │   Cloud Engine (Groq) │  │
│   │  whisper.cpp     │           │   whisper-large-v3    │  │
│   │  sidecar process │           │   turbo / standard    │  │
│   │  ggml-*.bin      │           │   api.groq.com        │  │
│   └──────────────────┘           └───────────────────────┘  │
│              │                                               │
│              ▼                                               │
│   ┌──────────────────┐                                       │
│   │  SendInput       │  ← types transcript directly          │
│   │  (Win32 API)     │    into focused app                   │
│   └──────────────────┘                                       │
└─────────────────────────────────────────────────────────────┘

Config/Data: %APPDATA%\com.demod.app\
  ├── config.json       (settings, API key)
  ├── ggml-small.bin    (466 MB, local ASR)
  ├── ggml-medium.bin   (1.5 GB, local ASR)
  └── dmod.log          (runtime events)
```

---

## Features

- **Global hotkey dictation** — `Ctrl+Shift+Space` from any app. Three modes:
  - **Auto** (default): tap to toggle, hold for push-to-talk
  - **Toggle**: one press starts, next press stops
  - **Push-to-talk**: held = recording, released = transcribe
- **Types directly into any app** — injects text as Win32 `SendInput` Unicode keystrokes. Works in terminals, browsers, editors, Electron apps — anywhere, regardless of paste shortcut.
- **Two transcription engines**
  - **Local (whisper.cpp)** — fully on-device, zero network, no API key needed. Models: Small (466 MB, fast), Medium (1.5 GB, balanced), Large-v3 (2.9 GB, best quality).
  - **Cloud (Groq)** — near-real-time via `whisper-large-v3-turbo`. Free tier with your own key at [console.groq.com](https://console.groq.com).
- **Animated overlay pill** — bottom-center of screen, always on top:
  - **Idle**: flat lozenge bar, hover expands, click opens settings
  - **Recording**: red pill with 5-bar live waveform tracking voice level
  - **Transcribing**: wave-loader ripple animation
  - **Done**: green tick flash (400ms), back to idle
  - **Notice**: full error message displayed in pill (no silent failures)
  - **Cancel**: X-circle button discards in-progress recording
- **Esc to cancel** — under 30s: instant discard. Over 30s: confirm-discard prompt (prevents accidental loss of long recordings).
- **Auto-paste with focus guard** — captures foreground window at recording stop; skips paste if you switched apps during transcription (clipboard fallback instead).
- **Optional text cleanup**
  - Filler-word removal (local, instant)
  - AI auto-edit via Groq `llama-3.1-8b-instant` (opt-in)
  - **Both OFF by default** — raw transcript unless you enable them.
- **Interaction sounds** — start/stop pings (opt-out in settings).
- **Searchable history** — local transcript log, size-capped, optional auto-delete window.
- **Single-instance** — named Win32 mutex prevents hotkey conflicts from duplicate launches.
- **Launch at startup** — registry Run key (removes legacy Startup-folder shortcut if found).
- **Per-user isolation** — all data under `%APPDATA%\com.demod.app\`, nothing system-wide.

---

## Tech Stack

| Layer | Choice | Notes |
|---|---|---|
| Shell | Tauri v2 (Rust) | No Electron, native WebView2 |
| Frontend | Vanilla TypeScript + Vite | No framework |
| Overlay | WebView2 (inline HTML/CSS) | CSS-only animations |
| Local ASR | whisper.cpp sidecar | ggml format, 3 model sizes |
| Cloud ASR | Groq Whisper API | Optional, requires key |
| Auto-edit | Groq llama-3.1-8b-instant | Opt-in |
| Audio | cpal (cross-platform audio) | WASAPI on Windows |
| Text injection | Win32 SendInput | Works in all apps |
| Mouse hook | WH_MOUSE_LL (Win32) | Pill hit-test / drag |
| Platform | Windows 10/11 x64 | Only target |

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org) 18+
- [Rust](https://rustup.rs) stable (MSVC toolchain: `rustup default stable-x86_64-pc-windows-msvc`)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/) — WebView2 runtime (ships with Windows 11; install on Win 10)

### Build & Run

```bash
git clone https://github.com/tj-guruvelli/demod
cd demod
npm install
npm run tauri dev       # dev mode (hot-reload frontend)
npm run tauri build     # production exe + NSIS/MSI installer
```

Release binary: `src-tauri/target/release/demod.exe`
Installers: `src-tauri/target/release/bundle/`

### Download a Voice Model

The app will prompt you on first run. Models are downloaded from [archive.org](https://archive.org/download/whisper-models/) (small/medium) or [HuggingFace](https://huggingface.co/ggerganov/whisper.cpp) (large-v3).

| Model | Size | Speed | Best for |
|---|---|---|---|
| Small | 466 MB | Fast | Quick notes, strong accent tolerance |
| Medium | 1.5 GB | Balanced | Everyday dictation (recommended) |
| Large-v3 | 2.9 GB | Slower | Maximum accuracy, technical vocabulary |

Or drop a model file manually:
```
%APPDATA%\com.demod.app\ggml-small.bin
%APPDATA%\com.demod.app\ggml-medium.bin
%APPDATA%\com.demod.app\ggml-large-v3.bin
```

---

## Configuration

All settings live in `%APPDATA%\com.demod.app\config.json` and are managed through the Settings UI.

Key settings:

| Setting | Default | Description |
|---|---|---|
| `engine` | `"local"` | `"local"` or `"cloud"` |
| `whisperModel` | `"small"` | `"small"`, `"medium"`, or `"large-v3"` |
| `microphone` | `""` | Device name or `""` for system default |
| `hotkey` | `"CmdOrCtrl+Shift+Space"` | Global recording shortcut |
| `recordingMode` | `"auto"` | `"auto"`, `"toggle"`, or `"push-to-talk"` |
| `groqApiKey` | `""` | Required for cloud engine |
| `autoPaste` | `true` | Auto-type transcript on stop |
| `removeFillers` | `false` | Strip um/uh/like locally |
| `autoEdits` | `false` | AI cleanup via Groq LLM |

> **Security:** `config.json` is local-only and never transmitted. Your Groq API key is stored in plaintext — do not commit or share this file.

---

## Privacy

| Data | Where it goes |
|---|---|
| Audio (local engine) | Never leaves your device |
| Audio (cloud engine) | Sent to Groq for transcription only |
| Transcript history | Stored locally, never uploaded |
| Settings / API key | Local only (`%APPDATA%\com.demod.app\`) |
| Telemetry | None |

---

## Known Limitations

- **Windows only** — macOS/Linux not supported (Win32 APIs throughout: SendInput, WH_MOUSE_LL, WASAPI).
- **Corporate proxy (Zscaler etc.)** — `api.groq.com` and `huggingface.co` may be blocked. Use local engine + download models manually via `archive.org` mirror.
- **Large-v3 model** — only available on HuggingFace (no mirror). Download off-network if HF is blocked.
- **VS SDK damage** — if AV quarantines MSVC CRT libs, use `stable-x86_64-pc-windows-msvc` + `lld-link` workaround in `.cargo/config.toml`.

---

## Project Structure

```
demod/
├── src/                    # TypeScript frontend
│   ├── main.ts             # Settings UI, hotkey capture, history
│   ├── overlay.html        # Overlay pill (self-contained HTML/CSS/JS)
│   └── style.css           # Settings window styles
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # App setup, hotkey, overlay, menu commands
│   │   ├── recorder.rs     # Recording state machine, transcription dispatch
│   │   ├── audio.rs        # cpal microphone capture, WAV writing
│   │   ├── transcribe_local.rs   # whisper.cpp sidecar, model download
│   │   ├── transcribe_groq.rs    # Groq API client
│   │   ├── settings.rs     # Config load/save, defaults
│   │   ├── sounds.rs       # Start/stop pings (winmm)
│   │   ├── pill_hittest.rs # WH_MOUSE_LL hook, drag, click-through
│   │   ├── autostart.rs    # Registry Run key management
│   │   ├── history.rs      # Transcript log (atomic writes, size cap)
│   │   ├── applog.rs       # Runtime event log (dmod.log)
│   │   └── downloader.rs   # Streaming model download with progress
│   ├── binaries/           # Bundled whisper.cpp sidecars + DLLs
│   ├── .cargo/config.toml  # Linker / SDK workarounds
│   └── tauri.conf.json     # App config, bundle settings
├── docs/                   # Architecture diagrams, runbooks
├── index.html              # Settings window shell
└── progress.txt            # Dev log / session notes
```

---

## IDE Setup

[VS Code](https://code.visualstudio.com/) +
[Tauri extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) +
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

---

## Status

`v0.1.0` — personal project, actively developed. Windows 10/11 x64 only.
