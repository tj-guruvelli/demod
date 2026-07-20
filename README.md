# Demod

> **Formerly "Typr", briefly mis-shipped as "DMOD". The product name is `Demod`.**
> The checkout directory is still `tools/typr` and the bundle identifier is still
> `com.typr.app`. Both are deliberate holdovers, not stragglers: Tauri derives the
> settings path from the identifier, so changing it orphans an existing install's
> saved config (engine, API key, hotkey). Renaming either is a migration, not a
> find-and-replace. Everything user-visible says Demod.
> Naming rationale and the rejected alternatives: `tasks/research/naming.md`.

Demod (a clipping of "demodulator" — the circuit that recovers information from a
carrier wave) is a dictation app that transcribes speech **locally on your machine**
via whisper.cpp, with an optional cloud engine (Groq) for when you want it. Local
transcription means your audio never has to leave your device to be turned into text.

This template should help get you started developing with Tauri in vanilla HTML, CSS and Typescript.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
