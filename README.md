# Demod

> **Formerly "Typr". Renamed to `Demod` on 2026-07-20** — directory, bundle
> identifier, and on-disk user data all moved together, so the rename is complete
> rather than cosmetic: checkout is `tools/demod`, the identifier is
> `com.demod.app`, and the pre-existing `com.typr.app` config + whisper model were
> migrated into it. Tauri derives the settings path from the identifier, so any
> FURTHER identifier change would orphan the saved config (engine, API key, hotkey)
> and the 487 MB model unless those directories are moved again in lockstep.
> Naming rationale and the rejected alternatives: `tasks/research/naming.md`.

Demod (a clipping of "demodulator" — the circuit that recovers information from a
carrier wave) is a dictation app that transcribes speech **locally on your machine**
via whisper.cpp, with an optional cloud engine (Groq) for when you want it. Local
transcription means your audio never has to leave your device to be turned into text.

This template should help get you started developing with Tauri in vanilla HTML, CSS and Typescript.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
