# Demod

> A local-first dictation app for Windows. Press a hotkey, talk, get text. Works anywhere on your desktop.

[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)](https://github.com/tj-guruvelli/demod)
[![Version](https://img.shields.io/badge/version-0.1.1-purple)](https://github.com/tj-guruvelli/demod/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-orange)](https://tauri.app)

---

Demod (short for "demodulator", the circuit that recovers signal from a carrier wave) is a global-hotkey dictation tool. Hit `Ctrl+Shift+Space`, talk, and your words are typed directly into whatever app is focused. No clipboard juggling. No browser tab. No subscription.

Transcription runs **fully on-device** via whisper.cpp. Groq cloud is available as an optional fast path when you have an API key.

Built with **Tauri v2 + vanilla TypeScript**. No Electron, no Node runtime in production.

---

## Install (use Demod on any PC)

**You do not need to clone this repository, or install Rust, Node, Visual Studio, or anything else, to use Demod.** Demod ships as a normal Windows installer, like any other desktop app. The source code is only for people who want to change it.

1. Open the [Releases page](https://github.com/tj-guruvelli/demod/releases).
2. Download `Demod_<version>_x64-setup.exe` (or the `.msi` for a per-machine install).
3. Run it. Demod installs to `%LOCALAPPDATA%\Demod` and adds a Start menu shortcut. No admin rights needed.
4. Launch Demod, pick an engine in **Settings**, and press `Ctrl+Shift+Space` in any text field to dictate.

| Included in the installer | Not needed on the target PC |
|---|---|
| `demod.exe`, the whisper.cpp sidecars and DLLs | Git, this repo, Rust, Node, Visual Studio |
| The C runtime (statically linked) | The VC++ redistributable |
| Settings UI, overlay, history store | Admin rights (per-user install) |

The only download after install is the whisper model the first time you use the **Local** engine (466 MB for Small); Demod fetches it itself into `%APPDATA%\com.demod.app`. The **Cloud** engine needs no model, only a Groq API key. WebView2 is already part of Windows 11; on Windows 10 the installer pulls it in if missing.

Moving to another PC later: install Demod there the same way, then copy `%APPDATA%\com.demod.app\config.json` across if you want the same settings.

> **Identifier is load-bearing.** Tauri derives the on-disk settings path from the bundle identifier (`com.demod.app`), so changing it would orphan the saved config and the whisper model. Keep `com.demod.app` stable.

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

- **Global hotkey dictation**: `Ctrl+Shift+Space` from any app. Three modes:
  - **Auto** (default): tap to toggle, hold for push-to-talk
  - **Toggle**: one press starts, next press stops
  - **Push-to-talk**: held = recording, released = transcribe
- **Types directly into any app**: injects text as Win32 `SendInput` Unicode keystrokes. Works in terminals, browsers, editors, Electron apps, anywhere, regardless of paste shortcut.
- **Two transcription engines**
  - **Local (whisper.cpp)**: fully on-device, zero network, no API key needed. Models: Small (466 MB, fast), Medium (1.5 GB, balanced), Large-v3 (2.9 GB, best quality).
  - **Cloud (Groq)**: near-real-time via `whisper-large-v3-turbo`. Free tier with your own key at [console.groq.com](https://console.groq.com).
- **Animated overlay pill**: bottom-center of screen, always on top:
  - **Idle**: flat lozenge bar, hover expands, click opens settings
  - **Recording**: red pill with 5-bar live waveform tracking voice level
  - **Transcribing**: wave-loader ripple animation
  - **Done**: green tick flash (400ms), back to idle
  - **Notice**: full error message displayed in pill (no silent failures)
  - **Cancel**: X-circle button discards in-progress recording
- **Esc to cancel**: under 30s: instant discard. Over 30s: confirm-discard prompt (prevents accidental loss of long recordings).
- **Auto-paste with focus guard**: captures foreground window at recording stop; skips paste if you switched apps during transcription (clipboard fallback instead).
- **Optional text cleanup**
  - Filler-word removal (local, instant)
  - AI auto-edit via Groq `llama-3.1-8b-instant` (opt-in)
  - **Both OFF by default.** Raw transcript unless you enable them.
- **Interaction sounds**: start/stop pings (opt-out in settings).
- **Searchable history**: local transcript log, size-capped, optional auto-delete window.
- **Single-instance**: named Win32 mutex prevents hotkey conflicts from duplicate launches.
- **Launch at startup**: registry Run key (removes legacy Startup-folder shortcut if found).
- **Per-user isolation**: all data under `%APPDATA%\com.demod.app\`, nothing system-wide.

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

## Building from source (developers only)

Skip this whole section if you only want to use Demod. See **Install** above.

### Prerequisites

- [Node.js](https://nodejs.org) 18+
- [Rust](https://rustup.rs) stable (MSVC toolchain: `rustup default stable-x86_64-pc-windows-msvc`)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/): Visual Studio 2022 Build Tools with "Desktop development with C++" and a Windows 10/11 SDK; WebView2 runtime (ships with Windows 11; install on Win 10)

### Build & Run

```bash
git clone https://github.com/tj-guruvelli/demod
cd demod
npm install
npm run tauri dev       # dev mode (hot-reload frontend)
npm run tauri build     # production exe + NSIS/MSI installer
```

Release binary: `src-tauri/target/release/demod.exe`
Installers: `src-tauri/target/release/bundle/` (`nsis/` and `msi/`). These are what go on the Releases page.

Run `cargo check` / `cargo test` from inside `src-tauri/`: cargo reads `.cargo/config.toml` from the current directory, not from the manifest path.

Always build via `npm run tauri build`, never a plain `cargo build --release`: a Tauri exe built outside the Tauri CLI embeds the dev-server URL and ships a dead WebView. See `docs/DEPLOY.md` for the deploy runbook.

### No hard-coded paths (repo rule)

The repository must build from **any checkout location on any machine**, so:

- No absolute filesystem paths (`C:\Users\<name>\...`, `/home/...`) anywhere in code, build config, scripts, prompts, or docs. Use repo-relative paths, `CARGO_MANIFEST_DIR`, `%LOCALAPPDATA%` / `%APPDATA%`, or Tauri's path API.
- `src-tauri/.cargo/config.toml` is committed and portable (`+crt-static` only, so the exe needs no VC redist). It must never grow a machine-specific entry.
- Before committing, grep for your own user name or drive letter. A hit means the change is not portable yet.

### Machine-specific toolchain workarounds

Some machines have a damaged MSVC install (antivirus quarantining CRT import libs such as `msvcrt.lib` or SDK headers), or need a different linker. Fix that **on the machine, never in the repo**: put the workaround in your user-level `%USERPROFILE%\.cargo\config.toml`, which cargo merges with the repo config and which git never sees. Example:

```toml
# %USERPROFILE%\.cargo\config.toml  (this machine only)
[target.x86_64-pc-windows-msvc]
rustflags = [
  "-C", "linker=lld-link.exe",                      # if link.exe is broken
  "-L", "native=<folder with your copied .lib files>",
]

[env]
CL = "/I<folder with your copied SDK headers>"       # if a header is missing
```

Keep the copied `.lib` / `.h` files in a folder under your home directory (for example `%USERPROFILE%\.cargo\msvc-patch`), not in the repo. Delete the file once the toolchain is repaired.

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

> **Security:** `config.json` is local-only and never transmitted. Your Groq API key is stored in plaintext. Do not commit or share this file.

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

- **Windows only**: macOS/Linux not supported (Win32 APIs throughout: SendInput, WH_MOUSE_LL, WASAPI).
- **Corporate proxy (Zscaler etc.)**: `api.groq.com` and `huggingface.co` may be blocked. Use local engine + download models manually via `archive.org` mirror.
- **Large-v3 model**: only available on HuggingFace (no mirror). Download off-network if HF is blocked.
- **Damaged MSVC toolchain (AV quarantine)**: affects building only, never the installed app. Do not patch the repo; see *Machine-specific toolchain workarounds* above.

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
│   ├── .cargo/config.toml  # Portable build flags only (+crt-static); no machine paths
│   └── tauri.conf.json     # App config, bundle settings
├── docs/                   # Architecture diagrams, runbooks
├── tasks/                  # Research notes, lessons, todo
├── CLAUDE.md               # Repo rules for Claude Code (no hard-coded paths, etc.)
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

`v0.1.1`. Personal project, actively developed. Windows 10/11 x64 only.
