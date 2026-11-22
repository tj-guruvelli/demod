# Handy (MIT) — what Demod can borrow

Source: analysis pasted by TJ on 2026-11-22 (origin: a separate Claude chat that read the
Handy repo). Not yet verified against Handy's current source; file paths below are as
given in that analysis. Verify each path before porting code. Handy is MIT licensed:
copying code is fine as long as its copyright notice is kept.

## Ranked by payoff

1. **Live chunked transcription** — Demod records the whole clip, writes a WAV, then POSTs
   it to whisper-server after stop. Handy cuts audio at natural pauses while the user is
   still talking and decodes each chunk as it goes, so only the last few seconds remain
   when recording stops. `managers/transcription/chunked.rs`, `live.rs`.
2. **VAD silence trimming** — Silero VAD with smoothing and a pre-roll buffer so the first
   syllable is not clipped. Less audio = faster transcription and fewer Whisper
   hallucinations from silence ("Thank you for watching"). `audio_toolkit/vad/silero.rs`,
   `smoothed.rs`. VAD is also what finds the pauses for #1.
3. **Crash isolation with a supervisor** — Demod already runs whisper.cpp out of process
   and falls back to the CLI when the server is unreachable. Handy adds a supervisor that
   owns the worker, restarts it after a GPU driver fault or GGML assert, and handles
   hangs. `engine_supervisor/`.
4. **GPU acceleration on Windows** — Handy ships a Vulkan backend via its `transcribe-cpp`
   crate (NVIDIA, AMD, Intel). If Demod's sidecar is a CPU build, Medium and Large-v3
   are slow; a Vulkan whisper-server build would use the Legion's GPU.
5. **Parakeet V3 model option** — CPU-optimized, fast, auto language detection. Better
   default for machines without a strong GPU than a 1.5 GB Medium download; fits the
   plug-and-play-on-any-machine goal.
6. **Transcription hook** — if `<appdata>/hooks/transcription` exists, pipe the transcript
   to it on stdin and paste stdout, with a timeout. Gives users custom cleanup, slang
   fixes, routing. `transcription_hook.rs`, ~100 lines.
7. **CLI remote control** — `handy --toggle` / `--cancel` talk to the running instance.
   Demod already has a single-instance mutex; adding this enables Stream Deck,
   AutoHotkey, mouse-button bindings.
8. **Portable mode** — a `portable` marker file beside the exe makes all data live in a
   `Data/` folder next to it instead of `%APPDATA%`. `portable.rs`. Cheap; useful for
   USB use and locked-down machines.

## Skip

- Handy's Windows paste (`paste_tx/windows.rs`): delayed-render clipboard data plus
  clipboard restore. Demod's `SendInput` typing already avoids the clipboard, which is
  one of Demod's real edges. Borrow only for the clipboard fallback path, if at all.
- macOS secure input, Linux primary selection, glibc tuning: not applicable to a
  Windows-only app.

## Suggested order

VAD (#2) first as a contained change in `recorder.rs`, then chunking (#1) on top of it,
then the supervisor (#3). Together #1 and #2 are the difference between "wait for it"
and "it's just there".

## Status

Research only. Nothing ported yet. Next step when picked up: read Handy's actual source
for #2 and sketch how VAD plugs into `src-tauri/src/recorder.rs`.
