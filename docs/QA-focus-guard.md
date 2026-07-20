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
3. Council scenario (First Principles): start dictation, and while transcribing,
   OPEN TYPR'S OWN SETTINGS WINDOW (tray/menu). EXPECTED: no paste into the settings
   window's controls; fallback path as in #2.
4. Focus change DURING recording (before stop): start in Notepad, Alt-Tab mid-speech,
   Alt-Tab back, stop. EXPECTED: paste lands in Notepad (focus at stop == focus at paste).
5. Push-to-talk variant of #2 (hold key, release after switching apps).
6. Tap-toggle variant of #2 (tap start, tap stop from the other app via hotkey).
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
