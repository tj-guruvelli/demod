# Demod — overlay + persistence + Wispr-like build-out (2026-07-21)

## Scope (user-confirmed)
Fix persistence; redo overlay pill; build out History + Privacy + Transcription
tabs "very similar to Wispr Flow"; **+ AI auto-edits** (user picked "3 tabs +
auto-edits"). History logging ON by default. Model default = Groq cloud
whisper-large-v3-turbo (real-time). Deploy swarm QA/QC (done: 18 findings fixed).

## Root cause (persistence)
Bundle-identifier churn com.typr.app(committed) vs com.demod.app(worktree). Fixed:
pinned com.demod.app + startup migration (now ATOMIC tmp+rename) + must COMMIT.

## DONE — Rust (cargo test 53 green, cargo check clean)
- migrate_legacy_app_dir: atomic copy (tmp+rename) [QA#1].
- Single-instance guard (Windows named mutex, no new crate).
- cancel_recording command + X-circle wired; on_click gated on idle state [QA#2].
- history.rs store (atomic, 500 cap, retention prune) + with_lock serialization [QA#6].
- recorder append: with_lock, retention on write, "on by default" wording [QA#11].
- Settings: voiceSensitivity, whisperThreshold, historyEnabled(true),
  historyRetentionDays, removeFillers(true), autoEdits(true). engine=cloud,
  cloudModel=whisper-large-v3-turbo defaults.
- AI auto-edits: cleanup::remove_fillers (local, tested) + transcribe_groq::
  groq_auto_edit (llama-3.1-8b-instant, best-effort) wired into stop_and_transcribe.
- History commands list/delete/clear/copy (+ with_lock on mutating).

## DONE — overlay.html
- Idle: flat deep-purple bar (116x4), NO glow/pulse; logo on hover; click opens app.
- Recording pill 180x36 fixed; 128px waveform + X cancel button.
- Whisper(wave) vs talking(bars) switch (voiceSensitivity + whisperThreshold).
- reduced-motion now covers processing dots [QA#10].

## IN PROGRESS — frontend worker (index.html, main.ts, style.css)
- 12 frontend QA fixes (#3,4,5,7,8,12,13,14,15,16,17,18).
- History build-out: search, stats strip (dictations/words/~time saved), styled
  rows (had ZERO css [QA#4]), Copy feedback, Delete confirm.
- Privacy build-out: carded intro, per-engine data-flow disclosure, label/aria fixes.
- NEW Transcription tab: move Language/Dictionary/Formatting/VoiceDetection from
  Models; add remove-fillers + auto-edits toggles.

## KNOWN TRADEOFF (flagged to user, not changed)
- QA#9: idle bar contrast on very dark backdrops is low; respecting the explicit
  "flat bar, no outline/glow at rest" directive. Offer a hairline safety outline.

## REMAINING
- [ ] Review frontend worker diff; npx tsc + npm run build green.
- [ ] Visual validation (screenshots) of overlay states + all 7 tabs.
- [ ] Commit everything (STOPS the identifier churn) — gated on user OK.
- [ ] `npm run tauri build` + replace AppData\Local\Demod\demod.exe (.bak rollback) — gated.

## Deploy runbook (docs/DEPLOY.md)
cargo test --lib -> npm run build -> cargo test --lib -> npm run tauri build ->
kill/backup/replace exe -> manual QA. Build workaround auto-applied via .cargo/config.toml.
Model already on disk at Roaming\com.demod.app\ggml-small.bin (no re-download after rebuild).
