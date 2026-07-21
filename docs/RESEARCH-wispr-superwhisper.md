# RESEARCH: Wispr Flow + SuperWhisper UX teardown (for Demod)

Date: 2026-07-19. Sources: official docs (docs.wisprflow.ai, superwhisper.com/docs), changelogs
(wisprflow.featurebase.app, superwhisper.com/changelog), engineering blog (wisprflow.ai/post/technical-challenges),
the PillFloat reverse-engineering repo (github.com/OrangeAKA/pillfloat), and independent reviews
(spokenly.app, tldv.io, getvoibe.com, zackproser.com). Full source list at the bottom.

Scope: observable product behavior only, mapped to how Demod replicates it in Tauri v2 with a
vanilla TS + CSS transparent always-on-top overlay webview. Hard constraints honored throughout:
transform/opacity-only animation, no polling loops, event-driven everything.

---

## 1. The floating pill / recording window

### Wispr Flow "Flow Bar" (Mac + Windows)

| Feature/Behavior | Exact observable detail | Replicate in Tauri v2 |
|---|---|---|
| Position | Bottom-center of screen, floating above everything. Draggable to bottom, left, or right edges; when docked to a side it reorients vertically. Position persists across launches. | Overlay window positioned via `currentMonitor()` work area: `x = (workW - winW)/2`, `y = workH - winH - 8`. Persist dock edge + offset in store. Reposition on `monitor` change / scale-factor events, never on a timer. |
| Window construction | Reverse-engineered by PillFloat: the visible pill (~70px wide) lives inside a ~440x300px fully transparent host window titled "Status". The oversized host gives animation headroom so the pill can grow into waveform/buttons without resizing the OS window. | Copy this exactly: one transparent window ~440x160 (bottom strip is enough), `decorations:false, transparent:true, alwaysOnTop:true, skipTaskbar:true, shadow:false, focus:false, resizable:false`. All state morphing happens INSIDE the webview via CSS; the OS window never resizes during a state change (resizing = jank + flicker). |
| Idle state | Small dark lozenge, roughly 60-70px x 10-14px, near-invisible. Visible at all times by default; hidden by full-screen apps. Optional settings toggle to hide entirely. | `.pill[data-state=idle]`: width ~64px, height ~12px, `border-radius:999px`, background `rgba(20,20,20,.85)`, subtle 1px light inner border. Hide on fullscreen: listen for foreground-window fullscreen via Rust (Win32 `SHQueryUserNotificationState` or window-rect == monitor-rect check on foreground-change event) and `window.hide()`. |
| Hover state | Pill brightens/expands slightly, becomes clearly clickable (center bubble). | CSS `:hover` -> `transform: scaleX(1.15) scaleY(1.2)` + opacity 1.0, 120ms ease-out. Requires cursor events ON only over the pill: keep window click-through by default (`setIgnoreCursorEvents(true)`) and flip to false when Rust-side hit-test (cursor within pill rect, checked on the OS mouse-move hook, not a JS poll) says cursor entered the pill zone. Simplest robust alternative: make the window small enough that it never occludes content and leave cursor events on. |
| Listening state | Live white waveform bars replace the idle lozenge; Cancel and Stop buttons appear beside it. A ping sound plays on start ("when you hear a ping or see the white bars moving, start speaking"). Waveform goes FLAT after a short period of silence (changelog fix: it used to keep wiggling with no input, users read that as fake). | Pill morphs to ~180x36px: container `transform: scale()` + crossfade children with opacity (no width/height animation; pre-lay both layouts absolutely and toggle opacity + `transform: translateY(2px)->0`). Waveform: 12-16 pre-rendered bar divs animated with `transform: scaleY(v)`; drive from real mic RMS. Rust (cpal) computes RMS per ~33ms chunk, emits a batched `levels` event at 30Hz; JS applies via one rAF writing a CSS var per bar. If RMS < threshold for >600ms, ease all bars to scaleY(0.08) (the honest flatline). |
| Processing state | After hotkey release: brief "processing" look (spinner/shimmer on the pill) then text lands. Changelog: "Dictation bubble flickering when switching between recording, processing, and idle states" was fixed with "smooth fade-in state transitions", so state changes crossfade, never hard-swap. | `data-state=processing`: bars collapse to a single pulsing dot row or shimmer sweep (`transform: translateX` on a gradient overlay, opacity loop). Only show it if processing exceeds ~250-300ms; under that, jump straight idle<-inserting (matches the "instant" feel on short utterances). Every state change = 150ms opacity crossfade + 150ms transform; never display:none swap mid-frame. |
| Inserting/confirm | Text pastes at cursor; pill flashes a brief success tick, then relaxes to idle. If paste fails after retries, pill shows a persistent "Paste" button instead (fallback). | `data-state=done`: tick icon fades in 100ms, holds 400ms, fades out. `data-state=paste-fallback`: pill stays expanded with a Paste button until clicked or Esc. |
| Click actions | Click center bubble = start/stop dictation. Right-click = quick menu: Hide for 1 hour, Settings, Microphone picker, Language picker, Transcript history, Paste last transcript. Language picker was later promoted INTO the bar itself (one click). | Left-click toggles hands-free session. `contextmenu` event opens a native-feeling menu rendered in the same webview (grows upward from pill, transform-origin bottom center, `scale(0.95)->1` + opacity 120ms). Menu items exactly: Hide for 1 hour, Microphone submenu, Language submenu, History, Paste last transcript, Settings, Quit. |
| Dragging | Drag pill anywhere along bottom/left/right edge; Esc cancels an in-progress drag; app snaps it to the nearest allowed edge. Known bug they fixed: bar rendered behind macOS Dock / stopped short of edges, i.e. clamp to work area, not full monitor. | Pointer-down on pill grip -> call `window.startDragging()` (Tauri built-in, native drag, no JS mousemove math). On drop, Rust snaps to nearest edge of `monitor.work_area` and persists. Do NOT copy Wispr's enforcement timer (they re-assert position every ~400ms; users hated it enough to build PillFloat). Set position on events only. |
| Hide behavior | "Hide for 1 hour" from context menu; full-screen apps hide it; Settings toggle removes it permanently (hotkey still works with the bar hidden). | Timer-based unhide via a single Rust `tokio::time::sleep` task, not JS polling. Bar hidden must never disable the hotkey path. |

### SuperWhisper recording window (Mac)

| Feature/Behavior | Exact observable detail | Replicate in Tauri v2 |
|---|---|---|
| Two sizes | Main window and Mini window; a resize toggle appears on hover (top-right icon) to switch between them. Both remember position AND minification state across app restarts (v1.40.0). | One overlay window, two CSS layouts (`data-view=main|mini`) crossfaded in place. Persist `{x, y, view}` in the store. |
| Appears on demand | Window shows on hotkey press, and auto-closes when paste succeeds (v1.40.2 "Automatically close recording window if paste was successful"). Optional Advanced setting keeps the mini window always visible as an idle indicator. | Default Demod behavior: pill always present but near-invisible at idle (Wispr model) OR SuperWhisper model behind a setting `overlay.alwaysVisible`. Auto-relax to idle on successful paste. |
| Position logic | Ensured on-screen when presented, otherwise centered; v2.16.1 "recording window now stays clamped to the screen edge instead of jumping off-screen"; can appear over full-screen apps (v1.45.4). | Clamp persisted coords to current work area on every show (coords can go stale when monitors change). |
| Status indicator | Color-coded dot on the window: Yellow = model loading, Blue = processing dictation, Green = processing complete. | 8px dot in pill corner, colored via CSS var, opacity-crossfaded between colors. Yellow maps to "model warming" (relevant when Demod runs local Whisper). |
| Mode display | Window shows active mode name + shortcut for switching; in Super Mode it shows the current app/website context it captured. | Text label in expanded pill: current mode + detected target app name. |
| Context capture indicator | Lights up when clipboard/selected-text context was captured, within 3 seconds of dictation start. | Small sparkle icon fading in when context grab succeeds. |
| Hover controls | Idle mini window hover reveals: Change Mode, Start Recording, Expand. Recording hover reveals Stop. Right-click: Expand, Settings, History. | Same pattern: controls hidden at rest, revealed on hover via opacity (pointer-events gated by opacity state). |
| Cancel guard | Cancel is instant under 30 seconds of recording; over 30s it asks for confirmation (protects long dictations from a stray Esc). | Track recording duration in Rust; >30s cancel -> pill shows inline "Discard? [Y/N]" instead of dropping audio. |
| Live transcription | v2.10.0 added a realtime streaming UI with live transcription text displayed while you speak. | Phase 2 for Demod: streaming partials from local Whisper into an expanding text line above the waveform (opacity-fade each partial in; never reflow the pill itself, give text a fixed-height clipped strip). |
| Waveform perf | Two separate changelog entries about waveform performance fixes (v1.40.6 "Massive performance improvements for Recording Window waveform", v2.15.0). This is a known hot spot. | Vindication of the constraint: bars via `transform: scaleY` only, one rAF loop, no canvas repaints of shadows/gradients per frame, no layout reads in the frame loop. |

---

## 2. Hotkey model

| Behavior | Wispr Flow | SuperWhisper | Demod replication |
|---|---|---|---|
| Default key | Mac: `Fn` (falls back to `Ctrl+Opt` if no Apple Fn). Windows: `Ctrl+Win`. | `Option+Space` | Windows default `Ctrl+Win` feels native to Wispr users; safer conflict-free default: `Ctrl+Space` or `F9`, configurable on first run. |
| Push-to-talk | Hold key = record; release = stop + process. | Same, "press and hold to record, release to stop". | Global shortcut with down/up: `tauri-plugin-global-shortcut` v2 reports `ShortcutState::Pressed/Released`. Start capture on Pressed, finalize on Released. |
| Toggle / hands-free | Separate shortcut: `Fn+Space` (Mac) / `Ctrl+Win+Space` (Win). ALSO double-tap of the main key = hands-free toggle. | Same shortcut can serve both: quick tap = toggle, hold = PTT (explicitly offered for mouse buttons). | One key, three gestures: keydown starts capture immediately; if keyup < ~300ms treat as toggle-on (keep recording); if held past 300ms it is PTT (stop on keyup). Second tap while recording = stop. This gives instant feedback in both gestures with zero extra latency. |
| Cancel | `Esc` cancels dictation. | Dedicated cancel shortcut; 30s confirmation guard. | Register `Esc` only while recording (dynamic register/unregister on state change) so we never swallow the user's Esc globally. |
| Paste last transcript | `Cmd+Ctrl+V` (Mac) / `Shift+Alt+Z` (Win). | Not exposed as shortcut. | Implement: re-run insertion of last result. Cheap, saves failed pastes. |
| Constraints | Max 3 keys; must include a modifier or mouse button 4-10/middle; plain letter keys and CapsLock rejected; OS-reserved combos blocked. | Mouse button dual function (click=toggle, hold=PTT), hardware-dependent. | Validate recorder UI with the same rules. Mouse side buttons via `rdev`/raw-input hook in Rust if the plugin lacks them. |
| Wake word | "Hey Flow" hands-free invocation (paid). | None. | Skip for v1. |

Key premium detail: Wispr's guidance is "when you hear a ping or see the white bars moving, start
speaking." Feedback (sound + waveform) must begin on KEYDOWN, before any model/network work.
Demod: on Pressed event, in order: (1) emit `state:listening` to overlay, (2) play start ping
(pre-decoded buffer, ~5ms), (3) open cpal stream (keep the input device warm/pre-opened to avoid
100-300ms device-open cost). Target: pill visibly reacts < 50ms after keydown.

---

## 3. Text insertion

| Behavior | Observable detail | Demod replication |
|---|---|---|
| Mechanism (both apps) | Simulated paste, not char-typing: save user clipboard -> put transcript on clipboard -> synthesize Cmd/Ctrl+V -> restore previous clipboard after success. Wispr documents this exactly, including that a cancelled paste does NOT restore, and that file/RTFD/audio clipboard types are not preserved. | Rust: read clipboard (text + best-effort formats), write transcript, `SendInput` Ctrl+V (enigo or windows-rs), wait ~150-300ms, restore. SuperWhisper shaved "time to paste latency" by 300ms in v2.16.2, so keep the settle delay as short as reliably possible; detect paste completion rather than sleeping long. |
| Fallback | Wispr: if insertion fails after retries, text stays on clipboard and the pill shows a Paste button. | Same: `paste-fallback` pill state + toast "Copied. Press Ctrl+V". |
| Clipboard hygiene | Wispr changelog: "Cleaner clipboard experience" so dictations do not pollute third-party clipboard-manager histories. | Windows: set the `CF_CLIPBOARD_VIEWER_IGNORE` / `ExcludeClipboardContentFromMonitorProcessing` formats when writing the transient transcript. |
| Accessibility path | Wispr states "Flow uses accessibility access to insert spoken words into other apps" (Mac AX API used for focus/context detection; insertion is still paste-based; iOS is a real keyboard). | Windows has no AX permission gate; use UIA only for reading focused-control context, keep insertion as paste. |
| AI cleanup | Wispr always post-processes: filler-word removal ("um/uh"), self-correction handling ("no wait, make that Tuesday" applies the correction), auto punctuation from pauses/tone, list/format detection. Whole pipeline budget: 700ms after speech ends (ASR <200ms, LLM <200ms, network <200ms). | Demod v1: local Whisper transcript + light rule-based cleanup (fillers, punctuation spacing) to stay in budget; optional LLM polish pass behind a per-mode toggle. Show raw text fast rather than perfect text slow. |
| Per-app tone | Wispr "Styles": 4 app categories (Email, Work messaging, Personal messaging, Other), each with a tone (Formal default; Casual/Very Casual/Excited variants), style cards with live preview text. App category detected automatically (Slack -> casual, Gmail -> professional) via Context Awareness. | Map foreground process/exe (+ browser tab host later) -> category -> prompt/cleanup preset. Ship the same 4 categories with preview cards. |
| Context awareness | Wispr reads: active app, text around the cursor, selected text, on-screen names, conversation history in chat apps, filenames in IDEs. Password fields excluded. Global toggle only (Settings -> Data & Privacy), not per-app. | Later phase: UIA read of focused element text for casing/punctuation continuation. Always excluded for password controls. |
| Per-app modes | SuperWhisper: modes auto-activate by app/website; once auto-activated, manual override is disabled for that context. Mode carries its own voice model + AI prompt + auto-paste setting. | Demod modes = {name, activation apps, STT model, cleanup prompt, paste on/off}. Auto-activation on foreground-window change event (Win32 `EVENT_SYSTEM_FOREGROUND` hook via `SetWinEventHook`, event-driven, no polling). |

---

## 4. Onboarding / setup

| Step | Wispr Flow | SuperWhisper | Demod |
|---|---|---|---|
| Account | Required (Google/Apple/MS/SSO/email), browser-based sign-in, email verification. Cloud product. | None required for free local use; license unlock for Pro. | None. Local-first, no account. |
| Permissions | Mic + Accessibility, with a re-verify flow after app updates (dedicated doc). | Onboarding wizard: Accessibility ("required for pasting text into other apps"), Microphone, input device selection. | Windows: mic permission via first `cpal` open (OS prompt). No AX equivalent needed. Onboarding still shows a mic test. |
| Mic test | Live audio visualization during onboarding (bug report: "microphone test bars stayed grey on Windows", i.e. the test IS live bars). | Onboarding verifies recording works. | Reuse the pill's own waveform component inside onboarding: proves the exact runtime path works. |
| Hotkey setup | Choose gesture: hold vs double-tap, pick the key, then PRACTICE dictation in a demo environment before being released into the OS. | Told the default (Option+Space) and prompted to try it. | Interactive practice field: "Hold Ctrl+Win and say something", show live pill + result inserted into an in-app textbox. Do not finish onboarding until one successful dictation. |
| Model download | None (cloud only). | Downloads first model during onboarding; Nano (150MB) or Fast (75MB) recommended for quick start; big models flagged as needing disk/RAM. | Bundle or auto-download a tiny model (whisper tiny/base, 75-150MB) with progress UI; larger models are explicit opt-in downloads from settings with size labels. |
| Language | Pick languages or auto-detect. | Per-model language selection. | Auto-detect default, per-mode override. |
| Total time | ~5 minutes claimed. | Minutes. | Target under 2 minutes to first successful dictation. |

---

## 5. Settings surface

Wispr Flow Hub (main window) left sidebar: **Home** (history + stats live on the home page since
Flow Home 2.0, stats widget on top), **Dictionary** (custom words + replacements, auto-add option),
**Snippets** (voice-triggered text blocks), **Style** (the 4 category tone cards), **Scratchpad**
(voice notes, cloud sync), **Transforms** (highlight text + wand icon to rewrite, Command Mode
family). Bottom: Settings, Help, Refer a Friend.

Wispr Settings tabs: **General** (Shortcuts, Microphone, Languages), **System** (Launch at login,
Show Flow Bar, Show in dock, sound toggles, Mute music while dictating, notification categories,
Scratchpad behavior, Auto-add to dictionary, Reset & restart), **Audio** (Interaction Sounds,
Use Built-In Mic), **Data & Privacy** (Privacy Mode, Context Awareness, Local storage, HIPAA),
**Vibe Coding** (variable recognition, IDE file tagging), **Experimental**, **Account**, **Billing**.

SuperWhisper settings sidebar: **General/Settings**, **Modes** (dedicated view since v2.13.0 with
activation flow), **Models** (voice models with per-model size labels + download buttons; language
models separately), **History** (recordings list; expand/collapse card animations), **Advanced**
(always-visible mini window, audio muting, media pause during dictation, system audio capture,
speaker ID). Note: it stores API keys in plaintext JSON under Application Support (called out
negatively in reviews; Demod should use Windows Credential Manager / DPAPI).

Demod v1 settings map (steal the union, ship the minimum): General (hotkey, mic, language),
Overlay (show/hide, dock edge, sounds), Dictionary, Modes (with per-app activation), Models
(download manager with sizes), History (local only, with per-item audio delete; SuperWhisper is
criticized for keeping audio on disk with NO toggle to disable, so Demod makes retention a
first-class setting), Privacy (everything local, retention days).

---

## 6. SuperWhisper model catalog (for Demod's model picker copy)

Local STT (name -> size): Fast 75MB, Nano 150MB, Standard 500MB (free tier ceiling), Pro 1.5GB,
Ultra V3 Turbo 1.6GB, Ultra V3 3.0GB, NVIDIA Parakeet V2 476MB / V3 494MB (near-realtime, custom
keyword list up to 1,000 terms). Benchmarks from reviews: Large-V3 ~1x realtime on M1 but 17-27x
realtime on M4; Turbo ~8x faster than Large-V3 at ~0.4pp WER cost. Lesson: default to the
turbo/small end, let enthusiasts opt into big models, and show speed-vs-accuracy honestly in the
picker (SuperWhisper labels each model with speed/accuracy/size columns).

---

## 7. Performance/animation patterns worth copying (and one to avoid)

1. Oversized transparent host window, small visible pill. The OS window NEVER resizes between
   states; all morphing is CSS inside the fixed canvas (Wispr: ~440x300 host, ~70px pill).
   Resizing a transparent always-on-top window per state = flicker + WM churn.
2. Crossfade state machine. Wispr shipped a specific fix replacing hard state swaps with
   "smooth fade-in state transitions" because flicker between recording/processing/idle was a
   filed bug. All states pre-mounted in DOM, toggled with opacity + small translate/scale,
   120-180ms, `ease-out` in, `ease-in` out.
3. Honest waveform. Bars driven by real RMS, flatline after ~0.5-1s of silence. SuperWhisper
   needed two rounds of "waveform performance" fixes; transform-scaleY bars + single rAF + zero
   layout reads avoids that class of bug entirely.
4. Feedback before work. Ping + bars on keydown; capture starts immediately; device kept warm.
   Perceived latency lives here, not in the model.
5. Latency budget as a product spec. Wispr: 700ms speech-end -> text-inserted (ASR 200 / LLM 200 /
   network 200). Demod local target: <=700ms for a 10s utterance with the small model; show the
   processing state only when the budget is blown (>300ms).
6. Auto-close/relax on success (SuperWhisper v1.40.2) and a paste-latency obsession (v2.16.2
   shaved 300ms off time-to-paste). Detect paste completion; do not sleep generously.
7. Clamp to work area on every show; persist position + view size; handle fullscreen (hide) and
   Dock/taskbar overlap (Wispr had bugs on both).
8. ANTI-PATTERN, do not copy: Wispr re-asserts the pill position on a ~400ms timer (PillFloat
   fights it at 150ms). Event-driven positioning only, and let the user drag the pill and keep it
   where they put it.
9. No polling anywhere: global-shortcut down/up events, `SetWinEventHook` for foreground-app
   change, monitor-change events for repositioning, one rAF loop that only runs while
   `state == listening`, audio levels pushed from Rust at a throttled 30Hz.
10. Sound design is a setting: interaction sounds toggle, "mute music while dictating"
    (media pause during dictation in SuperWhisper's Advanced settings), both apps ship it.

---

## 8. Top 10 premium behaviors, prioritized for Demod

1. **<50ms keydown feedback**: pill lights + ping + capture start on key press, everything else after.
2. **700ms speech-end-to-text-inserted budget**, processing UI shown only past 300ms.
3. **One key, three gestures**: hold = push-to-talk, tap = hands-free toggle, Esc = cancel
   (with the 30s discard-confirmation guard).
4. **Honest live waveform**: real RMS, scaleY-only bars, flatlines on silence.
5. **Flicker-free state morphing**: fixed-size transparent host window, pre-mounted states,
   opacity/transform crossfades (this exact polish is a shipped bugfix at Wispr).
6. **Clipboard-respecting paste**: save -> paste -> restore, clipboard-manager exclusion flags,
   Paste-button fallback, paste-last-transcript shortcut.
7. **Per-app tone/modes**: 4 style categories with preview cards (Wispr) + per-app auto-activating
   modes carrying model + prompt (SuperWhisper).
8. **Draggable, dockable, persistent pill** with work-area clamping, edge snap, fullscreen hiding,
   "Hide for 1 hour".
9. **Right-click quick menu on the pill**: mic, language, history, paste last, hide, settings.
10. **Model picker with honest tradeoffs**: size/speed/accuracy columns, tiny default downloaded
    during onboarding with progress, big models opt-in; audio retention is a user setting.

---

## 9. Suggested Demod overlay state machine

`hidden -> idle -> listening -> processing -> inserting -> done -> idle`
plus `paste-fallback` (from inserting) and `error` (from any). Transitions are events from Rust
(`demod://state` emit); the webview is a pure renderer of `data-state` on the pill root. All state
visuals pre-mounted; CSS handles the crossfade; JS only sets the attribute and feeds waveform vars.

---

## Sources

- https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android
- https://docs.wisprflow.ai/articles/3152211871-setup-guide
- https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts
- https://docs.wisprflow.ai/articles/7971211038-fix-text-not-pasting-after-dictation
- https://docs.wisprflow.ai/articles/4678293671-feature-context-awareness
- https://docs.wisprflow.ai/articles/2368263928-how-to-setup-flow-styles
- https://docs.wisprflow.ai/articles/5002934560-why-is-the-wispr-bar-is-not-appearing-or-disappearing
- https://wisprflow.ai/post/technical-challenges (latency budgets)
- https://wisprflow.featurebase.app/changelog and https://roadmap.wisprflow.ai/changelog
- https://github.com/OrangeAKA/pillfloat (pill window reverse engineering)
- https://superwhisper.com/docs/get-started/interface-rec-window
- https://superwhisper.com/docs/modes/modes
- https://superwhisper.com/docs/get-started/settings-shortcuts
- https://superwhisper.com/models
- https://superwhisper.com/changelog
- https://spokenly.app/blog/superwhisper-review and https://spokenly.app/blog/wispr-flow-review
- https://tldv.io/blog/wisprflow/
- https://www.getvoibe.com/resources/ (model benchmarks, safety review)
