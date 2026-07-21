# Demod Architecture: Boundaries, Contracts and Language

> Produced 2026-07-20 from a module-seam audit, then spot-verified against the code.
> Its purpose is narrow and practical: **every serious break in this project so far has
> happened at a seam, not inside a module.** The loop died between `paste.rs` and its
> caller. A UI regression appeared between `index.html` semantics and `style.css`. A
> status indicator lied because a command existed with no caller. This file inventories
> the seams so the next change does not have to rediscover them.
>
> Line numbers drift. The *shape* of each contract is what matters.

## 1. Crate topology (not where you would guess)

There are two crates, and the split is uneven:

- `demod_lib` (`src-tauri/src/lib.rs`) exports settings, audio, transcribe_local,
  transcribe_groq, cleanup, paste, recorder, downloader.
- `pill_hittest` and `sounds` are **not** in the lib. They are `mod` declarations inside
  the binary (`main.rs`).

Two consequences that bite:

- `cargo test --lib` does **not** compile the binary crate. A signature change can break
  `main.rs` while `--lib` stays green. Always also run `cargo check --bin demod`.
- `pill_hittest` reaches *upward* into the binary root for `crate::AppState` and
  `crate::OVERLAY_W/OVERLAY_H`. Nothing else in the tree does this.

`lib.rs::run()` is dead stock-template residue: it builds a second `tauri::Builder` with
no `.setup`, no `.manage`, no `invoke_handler`, and `main()` never calls it. `lib.rs` also
omits `pill_hittest` and `sounds` from its module list. Safe to delete; left alone here
because deleting it is a change, not documentation.

## 2. The silent-failure surface: Rust to webview

This is the highest-risk boundary in the app because **it is stringly typed on both sides
and fails quietly**. Nothing here is checked by the compiler, by `cargo test`, or by
`npm run build`.

### 2.1 `eval()` — three sites, one selector

All three write `document.getElementById('mic').dataset.state = '<value>'`:
`recorder.rs` (the real helper), `main.rs` (a byte-identical copy), and a third inline
hardcoded copy in `main.rs` with `'listening'` baked into the literal.

Six values are reachable: `idle`, `listening`, `processing`, `done`, `confirm-discard`,
`paste-fallback`. All six have a matching CSS layer in `overlay.html`, so there are no
orphan states today.

The failure mode is not a missing state. It is `getElementById('mic')` returning null if
that id is ever renamed: the webview throws, and the Rust side discards the result with
`let _ =`, so **nothing anywhere reports the failure**. The pill would simply stop
responding.

### 2.2 Events — four, all correctly paired

| Event | Emitted by | Scope | Listener |
|---|---|---|---|
| `recording-state` | `recorder.rs` | global `emit` | `src/main.ts` |
| `status-dot` | `recorder.rs` | `emit_to("overlay")` | `src/overlay.html` |
| `audio-level` | `audio.rs` | `emit_to("overlay")` | `src/overlay.html` |
| `download-progress` | `downloader.rs` | global `emit` | `src/main.ts` |

No dead events in either direction. Two residual risks: `recording-state` carries an
untyped string (`"Ready"`/`"Recording"`/`"Transcribing"` from a fieldless enum) that the
frontend string-compares with an `else` fallback to "Ready", so **renaming a Rust variant
would make the UI silently report Ready forever**; and one lifecycle is broadcast globally
while the other is label-scoped, for no stated reason.

### 2.3 Commands — 11 registered, 10 reachable

Live and invoked: `get_settings`, `save_settings`, `list_microphones`,
`check_model_downloaded`, `download_model`, `get_recording_state`, plus the four
`menu_*` commands.

`toggle_recording` is **dead**: it wraps `do_toggle_recording`, which is live via the
hotkey path, but the command form has no caller. Its `Err("Currently transcribing…")`
string is unreachable.

The four `menu_*` commands are bound *indirectly*: `data-action="menu_quit"` in the HTML
becomes `invoke(btn.dataset.action)`. The command name never appears next to `invoke`, so
grepping for `invoke('menu_quit')` finds nothing. Worse, that call site ends in
`.catch(() => {})`, which makes a typo'd `data-action`, an unregistered command, and a
genuine `Err` (like "No transcript yet") **indistinguishable and all silent**. See the
open-issues list below.

### 2.4 Strings Rust depends on

`'mic'` (3 hardcoded copies), `dataset.state`, the window label `"overlay"` (about ten
literals, no constant), the window label `"main"` (which is *not* declared in
`tauri.conf.json` — it relies on Tauri's default), the `data-action` values doubling as
command names, and the pill geometry constants in `pill_hittest.rs` that mirror CSS values
in `overlay.html`. Both sides comment that coupling; nothing enforces it. **Change the
pill CSS and the hit rectangle silently drifts out of alignment.**

## 3. One lifecycle, three vocabularies

This is the app's main source of confusion. The same moment in time has up to three names.

| `RecordingState` | pill `data-state` | `status-dot` |
|---|---|---|
| `Ready` | `idle` | `off` |
| `Recording` | `listening` | (unchanged) |
| `Transcribing` (first 300ms) | `listening` | `starting` |
| `Transcribing` (past 300ms) | `processing` | `processing` |
| `Ready` (paste succeeded) | `done` | `done` |
| `Ready` (paste fell back) | `paste-fallback` | `off` |
| `Recording` (1st Esc past 30s) | `confirm-discard` | (unchanged) |

Read that table before changing any state code, because:

- **`RecordingState` does not determine the pill.** `Ready` maps to three different pill
  states (`idle`, `done`, `paste-fallback`). The pill carries *outcome* information the
  Rust enum has no variant for. `update_overlay` looks like the mapping function but only
  covers the idle/listening/processing subset; the other three bypass it.
- **`processing` means two different things** — the three-dot pill layer, and the blue
  corner dot. They are deliberately allowed to disagree (the dot goes `starting` while the
  pill is still `listening`), using one shared word.
- **`starting` points at the wrong edge**: it fires when recording *stops* and
  transcription begins, while `play_start` genuinely means recording start.
- **Three different "idle" words**: `RecordingState::Ready`, `data-state="idle"`, and
  `HotkeyGesture::Idle` — the last is true *during* recording in hands-free mode.

Other one-concept-many-words pairs: the floating window is **overlay / pill / mic /
Status / Dock** depending on the file (and "overlay" also names the OS window, while
"mic" also names the audio input device); **cancel vs discard** are mixed inside a single
enum (`CancelGuard::ConfirmDiscard`); **hotkey / shortcut / gesture**; **local/cloud** in
settings vs **whisper/groq** in module names vs **Models** in the UI; and the model value
is variously `model_size`, `whisperModel`, `modelSize`, and `m.id`.

## 4. Duplication worth collapsing

- The overlay-state eval helper exists **three times** (see 2.1). One copy's comment
  justifies itself with "recorder.rs is under separate review right now" — that review is
  over.
- The stop-and-transcribe block is copy-pasted **three times** in the hotkey closure
  (check state, play stop sound, clone settings, await, match).
- Paste orchestration exists twice: the recorder's flow and `menu_paste_last`. They share
  `paste.rs` correctly, but each maps outcome to overlay state on its own.
- Work-area clamping exists twice in `pill_hittest.rs`, taking the pill size from two
  different sources of truth.
- `config.json` has **two writers**: the `save_settings` command and the drag-end persist.
  The command defensively re-asserts `overlay_x/overlay_y` from live state over whatever
  the frontend sent — that defensive code is the tell that this boundary is genuinely split.

## 5. Comments that are now false

Verified stale, worth deleting or fixing when next in the file:

- `recorder.rs` refers to a **"Shift+Alt+Z re-paste shortcut"**. No such shortcut exists
  anywhere; the only registered shortcuts are the configured hotkey and `Escape`.
- `overlay.html` says the menu's invokes are "no-ops on this dev branch until Rust wires
  the commands in US-009". They are wired.
- `main.rs` justifies its duplicate eval helper by an in-flight review that has ended.
- A commit message claims a "testid contract"; there are **zero `data-testid` attributes**.
  The hooks that do exist are `data-section`, `data-hotkey` and `data-health`, and the E2E
  harness works only because it tries those as fallbacks.

## 6. Rules that follow from the above

1. Run **both** `cargo test --lib` and `cargo check --bin demod`. The lib gate does not
   compile `main.rs`.
2. Changing a pill state, event name, command name, or DOM id is a **two-sided** change.
   Grep the other side; the compiler will not help you.
3. Renaming a `RecordingState` variant silently degrades the settings UI to "Ready".
4. Never resize or reposition the overlay window to change the pill's appearance. The
   window is fixed size and the pill morphs inside the webview; resizing reintroduces the
   flicker this design removed.
5. Keep low-level hook callbacks (`WH_MOUSE_LL`) cheap and lock-free — they run on the
   system input path.
6. Do not add a second implementation of anything in `paste.rs`. It is the one place the
   wrong-window bug can regress.
