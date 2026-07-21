# Demod

> **Formerly "Typr". Renamed to `Demod` on 2026-07-20** — directory, bundle
> identifier, and on-disk user data all moved together, so the rename is complete
> rather than cosmetic: checkout is `tools/demod`, the identifier is
> `com.demod.app`, and the pre-existing `com.typr.app` config + whisper model were
> migrated into it. Tauri derives the settings path from the identifier, so any
> FURTHER identifier change would orphan the saved config (engine, API key, hotkey)
> and the 487 MB model unless those directories are moved again in lockstep.

Demod (a clipping of "demodulator" — the circuit that recovers information from a
carrier wave) is a local-first dictation app for Windows: press a hotkey, talk, and
your speech is transcribed and typed straight into whatever text field is focused.
Transcription runs **locally on your machine** via whisper.cpp, with an optional
cloud engine (Groq) when you want lower latency. With the local engine, your audio
never leaves the device.

Built with Tauri v2 + vanilla TypeScript. Wispr-Flow-style overlay, no Electron.

## Features

- **Push-to-dictate hotkey** — toggle recording with a global shortcut
  (default `Ctrl+Shift+Space`), from any app.
- **Types the transcript directly** — injects the text as Unicode keystrokes
  (`SendInput`), so it lands in terminals, editors, browsers, and anywhere else
  regardless of that app's paste shortcut. The clipboard is also mirrored as a
  manual fallback.
- **Two engines**
  - **Local** — whisper.cpp runs the model on-device; audio never leaves the machine.
  - **Cloud** — Groq, defaulting to `whisper-large-v3-turbo` for near-real-time
    transcription. All Groq whisper models are free to use with your own key.
- **Animated overlay pill**
  - Idle: a flat deep-purple bar; hover reveals the logo; click opens the app.
  - Recording: a red pill with a live 5-bar **Wave** — a traveling ripple whose
    height tracks your voice level.
  - Transcribing: a **Wave Loader** ripple while the text comes back.
  - An X-circle **cancel** button discards an in-progress recording so it is never
    transcribed or pasted.
- **Voice detection** — tunable microphone sensitivity and a whisper threshold.
- **History / Privacy / Transcription tabs** — searchable local dictation history
  (atomic writes, size cap, optional retention window), a per-engine data-flow
  disclosure, and language/formatting controls.
- **Optional text cleanup** — local filler-word removal and an AI auto-edit pass
  (Groq `llama-3.1-8b-instant`). **Both are OFF by default** — Demod ships your raw
  words; turn either on in Settings if you want them.
- **Single instance** — a Windows named mutex prevents a second copy from stealing
  the hotkey.
- **Per-user isolation** — all settings, history, and models live under the current
  Windows user's app-data directory.

## Tech stack

| Layer      | Choice                                             |
|------------|----------------------------------------------------|
| Shell      | Tauri v2 (Rust)                                    |
| Frontend   | Vanilla TypeScript + Vite (no framework)           |
| Local ASR  | whisper.cpp (bundled sidecar)                      |
| Cloud ASR  | Groq Whisper API                                   |
| Auto-edit  | Groq `llama-3.1-8b-instant` (opt-in)               |
| Platform   | Windows 10/11 (x64)                                |

## Getting started

```bash
npm install
npm run tauri dev      # develop
npm run tauri build    # produce the release exe + MSI/NSIS installers
```

The release binary is written to `src-tauri/target/release/demod.exe`, with
installers under `src-tauri/target/release/bundle/`.

> **Build note (this machine):** local AV HTTPS scanning blocks fetching new
> crates and can gut parts of the VS/Windows SDK. The repo carries the necessary
> offline/link workarounds in `.cargo/config.toml`; builds run with
> `CARGO_NET_OFFLINE=true`. Always build via `npm run tauri build` (never a plain
> `cargo build --release`, which would embed the dev-server URL and ship a dead
> WebView). See `docs/` for the full deploy runbook.

## Configuration

Set your Groq API key and pick an engine/model in **Settings**. Configuration is
stored locally at:

```
%APPDATA%\com.demod.app\config.json
```

That file holds your preferences and, if you use the cloud engine, your Groq API
key **in plaintext** — it is local-only and never transmitted anywhere except to
Groq for transcription. Do not commit or share it.

## Privacy

- **Local engine:** audio is transcribed on-device; nothing is uploaded.
- **Cloud engine:** audio is sent to Groq for transcription only.
- History is stored locally and can be disabled or auto-pruned in Settings.
- Auto-edit / filler-removal are off by default; your transcript is otherwise
  verbatim.

## Status

Personal project, `v0.1.0`. Windows-only for now.

## Recommended IDE setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
