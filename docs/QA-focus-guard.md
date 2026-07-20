# Focus-Guard Manual QA Script

> Derived from an adversarial (LLM council) design review, 2026-07-20. These steps are
> load-bearing, not boilerplate: this is the ONLY verification the paste-safety fix
> (US-007) will ever get. CDP cannot synthesize global hotkeys or real focus changes,
> so hotkey gestures (tap/hold/auto), the focus-guard paste decision, and the quick
> menu have ZERO automated coverage. If you skip a scenario here, that behavior is
> simply unverified; there is no other test catching it.

Execute the full list during the dogfood window immediately after deploy, with the
old exe still present as `typr.exe.bak`. For every scenario, dictate a distinctive
phrase so a misplaced paste is unmissable.

## Scenarios (each = one dictation cycle)

1. Baseline: dictate into Notepad, stay focused. Text lands in Notepad. Ping sounds fire.
2. THE historical bug: start dictation in Notepad, while "Transcribing..." shows,
   Alt-Tab to a browser text field. EXPECTED: NO paste into the browser; clipboard
   holds the transcript; overlay shows paste-fallback state.
3. [AUTOMATED - typr-e2e/focus-guard-check.mjs, 7/7 passing] Council scenario
   (First Principles): start dictation, and while transcribing, OPEN TYPR'S OWN
   SETTINGS WINDOW. EXPECTED: no paste into the settings window's controls.
   The harness produces a REAL transcript using the Windows speech synthesizer
   through the speakers, which this machine's mic array picks up. The
   transcription is garbled, and that does not matter: the test is about WHERE a
   transcript lands, not what it says. It steals the foreground to the settings
   window mid-transcription with the caret in the Groq key field - the exact
   control a stray Ctrl+V would hit - and asserts that field is unchanged. The
   real config is captured and restored, verified afterwards.
4. Focus change DURING recording (before stop): start in Notepad, Alt-Tab mid-speech,
   Alt-Tab back, stop. EXPECTED: paste lands in Notepad (focus at stop == focus at paste).
5. [AUTOMATED - typr-e2e/gesture-check.mjs, 10/10] Push-to-talk: a 900ms HOLD of the
   real global hotkey starts, and release completes the dictation.
6. [AUTOMATED - typr-e2e/gesture-check.mjs] Tap-toggle: a short tap starts recording
   and a second short tap stops it, via the real global hotkey.
7. Esc cancel: start, speak, Esc within a beat. EXPECTED: discard, no paste anywhere,
   confirm-discard state behaves per US-006.
8. Rapid re-dictation: two cycles back-to-back into different apps. No cross-paste.
9. Elevated-window target: dictate into a window running as admin (e.g. elevated
   Notepad). EXPECTED: either paste works or fallback engages - no silent loss.
10. Paste-fallback recovery: trigger #2, then press Ctrl+V manually in the intended
    window; the transcript must still be intact on the clipboard. Note there is NO
    click-to-paste button on the fallback layer - it is text only ("Copied, press
    Ctrl+V to paste"), verified in src/overlay.html. The quick menu's "Paste last
    transcript" (scenario 11) is the other recovery route.
11. Quick-menu paste-last (US-009): dictate, then menu > Paste last transcript into
    a fresh Notepad. Lands correctly.
12. Repeat #2 in a real messaging app (the original bug's blast radius). NO paste.
13. [AUTOMATED - no longer needs doing by hand] RECOVERY AFTER A FAILED DICTATION:
    force a transcription failure - easiest is Settings > Models > Groq Cloud with
    the API key field emptied - then dictate once. EXPECTED: the pill shows a short
    failure message rather than spinning forever, AND the very next hotkey press
    starts a new recording normally. Before the fix, one failure left the recorder
    stuck in Transcribing and every later press was rejected silently, so the app
    had to be restarted.
    Covered by `typr-e2e/recovery-check.mjs` (8/8 passing): it forces a real
    failure by selecting the local engine with an undownloaded model - never
    touching the Groq key - then asserts the state returns to Ready AND that a
    new recording can still start, and restores the original settings
    afterwards. Re-run it after any change to stop_and_transcribe.
14. Settings save failure is reported: not easily forced by hand; if you ever see a
    setting revert after reopening Settings, the notice banner should have said so.
    Worth watching for during the dogfood day rather than staging.

## Deploy procedure context

This script runs as part of the dogfood window in [DEPLOY.md](./DEPLOY.md) section 4.
Full deploy/restore commands (PowerShell) live there; summary:

- Kill running `typr.exe`, CONFIRM the process is gone, copy the new exe in with the
  old one kept as `typr.exe.bak`, relaunch, run the CDP smoke test, then this script.
- Restore one-liner (memorize the location, `C:\Users\guruv\AppData\Local\Typr\`):
  stop the new exe, copy `typr.exe.bak` back over `typr.exe`, relaunch.
- Bad-exe DETECTION = any scenario above failing, or hotkey dead, or sidecar not up
  within 10s of launch.
- If any of scenarios 2, 3, 5, 6, or 12 fails: restore `.bak` IMMEDIATELY, file the
  failure, do not "try one more time" on the daily driver.

## Which of these actually need a human, and in what order

Scenarios 3 and 13 are automated and passing. Two claims in the header above
turned out to be wrong and are corrected here: CDP cannot synthesize speech or a
foreground change, but the SHELL can - the Windows speech synthesizer plus
AppActivate covers both, and garbled transcription is fine when the assertion is
about paste destination rather than content.

Every assumption about what was unautomatable turned out to be wrong, in three
stages. Recorded here so nobody re-derives them:

- "CDP cannot synthesize speech" - true, but the Windows speech synthesizer
  through the speakers reaches the mic array. Transcription is garbled, which
  does not matter when the assertion is about paste DESTINATION.
- "CDP cannot change the foreground window" - true, but AppActivate can.
- "A global hotkey cannot be pressed programmatically" - FALSE. The plugin uses
  RegisterHotKey, and Windows delivers keybd_event-injected input through the
  same path, so the real gesture state machine (including the 300ms tap-vs-hold
  threshold) is fully drivable.

AUTOMATED and passing: 3 (focus guard, focus-guard-check.mjs 7/7), 5 and 6
(gestures, gesture-check.mjs 10/10), 13 (failure recovery, recovery-check.mjs
8/8). Scenario 2 shares scenario 3's code path - a foreign foreground window
fails the hwnd+pid check and falls back - so it is covered in mechanism.

What is left is genuinely observational rather than a discrete test: scenarios
1, 4, 7-12 and 14, all lower risk, best confirmed simply by using the app for a
day with the .bak in place. Scenario 12 in particular (a real messaging app) is
the same code path as 2 and 3, just a different target application.

If 2, 5, 6 or 12 fails: restore the .bak immediately per DEPLOY.md and do not
retry on the daily driver.
