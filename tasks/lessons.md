
## 2026-07-21 — "Dictate anywhere" must TYPE the text, not clipboard+Ctrl+V

TJ, furious: "it's still not pasting it in here ... it was working perfectly fine
before ... what have you been fucking up?" He was dictating into Claude Code (a
terminal). `paste.log` said "confirmed foreground; Ctrl+V sent" - the injection
fired into the right window - yet nothing appeared. Root cause: **terminals treat
Ctrl+V as a control key, not paste** (paste is Ctrl+Shift+V / right-click there),
so a clipboard + synthetic-Ctrl+V paste silently does nothing in exactly the apps
a developer dictates into most. The fix Wispr uses: **type the transcript directly
as Unicode keystrokes** (`SendInput` with `KEYEVENTF_UNICODE`, wVk=0, wScan=code
unit), which lands in ANY focused text field regardless of its paste shortcut.
Belt-and-suspenders: also mirror to the clipboard (plain, no exclusion formats) so
Ctrl+V is a guaranteed manual fallback. See paste.rs `windows_impl::type_text`.

Corollary — you CANNOT verify synthetic input from these Bash/PowerShell tools:
they run **non-interactive** (no interactive window station), so `SendInput`
returns success (events accepted) but nothing renders. A self-contained WinForms
textbox test still showed empty because of the same non-interactive station. The
API works; only an interactive desktop (the running app, or the user) can confirm
rendering. Don't mistake a non-interactive-shell false-negative for an app bug.

## 2026-07-21 — A rename isn't done until the identity is pinned AND committed

TJ: "all my changes got wiped out ... models re-download ... why isn't this
persisting?" Nothing was wiped. The Typr→Demod rename left the bundle identifier
`com.typr.app` COMMITTED while the working tree had `com.demod.app`. Tauri derives
the app-data dir from that identity, so builds alternated identities: the 487MB
model got downloaded under `com.demod.app`, the installed build ran as
`com.typr.app`, looked in the wrong dir, found no model → "download a model" on
every launch, and read/wrote a different config.json → "settings reset". The UI
edits were never lost either — they were an uncommitted 244-line working-tree diff.

Rule: after a product rename, (1) pin the identity to ONE value in every place it
appears (tauri.conf.json identifier, get_app_dir, applog), (2) add a one-time
migration that copies old-identity user data forward (copy, never move/overwrite),
and (3) COMMIT immediately. An uncommitted rename that flip-flops per build is
indistinguishable from data loss to the user. "Is it committed?" is the first
question when a desktop app "forgets" state, not the last.

## 2026-07-21 — In an offline / AV-gutted build, don't reach for a new crate

Adding `tauri-plugin-single-instance` made `cargo check` try to update the crates.io
index, which died on the AV TLS interception (`CRYPT_E_NO_REVOCATION_CHECK`). The
repo builds ONLY from cached deps; any new crate needs a network fetch that this
machine's HTTPS scanning blocks. The fix was to implement the same feature with a
dependency already present — a Windows named mutex via the `windows` crate (just
had to enable its `Win32_Security` feature, which is a recompile, not a download).

Rule: before adding a dependency here, check whether an existing dep already does
it. New crates are a network operation and the network is hostile. Enabling a
feature on an already-vendored crate is free; pulling a new crate is a gamble.

## 2026-07-20 — Research the named product instead of inferring it

TJ said "make it as good as Wispr Flow" on day one. I built an entire settings
surface, dictionary feature and overlay against my *guess* at what Wispr Flow does,
and never once opened their site. When he finally asked directly, I had to admit I
had done no research at all. His response: any time I hit something I do not know, I
should already have an automation that goes and researches it, not fabricate.

Rule (now global, in ~/.claude/CLAUDE.md "Research Before Building"): a named
external product/API/spec I cannot verify triggers research BEFORE building -
WebSearch, then WebFetch the real docs, then Apify/scraping when the page resists,
then GitHub source. Findings get written to tasks/research/<product>.md so the build
has a citable source. Delegate it to a subagent; it is many pages of reading.

The failure mode is not "I lacked information". It is "I produced confident output
shaped like knowledge I did not have". An admitted unknown costs one sentence. A
fabricated one cost a week of building the wrong thing.

## 2026-07-20 — A repo edit is not a shipped change (Tauri bakes assets at compile time)

TJ, three times: "the logo hasn't been updated", "that still hasn't been updated",
"you're still not using the pulse icon". Every one of those was true, and none of
them was an icon-design problem.

The icon files were regenerated at 12:33. The installed binary was built at 12:22.
Tauri embeds icons and the frontend `dist/` into the executable at compile time, so
the app TJ was looking at physically could not contain any of it. The solid-colour
waveform fix had the same shape: `src/overlay.html` already set a single flat
`--wave-stroke`, with a comment citing TJ's own earlier "why is it gradient?"
correction - correct code, never compiled, so on screen it still looked wrong. I
would have "fixed" an already-fixed file a third time if I had not checked the
build timestamp against the asset timestamp.

Rule: for this app, a change is not done at file-write. It is done at
kill -> `npm run build` -> `cargo build --release` -> copy -> relaunch -> LOOK AT IT.
Before touching any asset TJ says is stale, first diff the artefact's mtime against
the installed exe's mtime. If the exe is older, the bug is the deploy, not the asset.
Verify by extracting what actually shipped (`[System.Drawing.Icon]::ExtractAssociatedIcon`
on the deployed exe) rather than by re-reading the source file - the source file is
the thing that has already lied twice.

Corollary on naming: the same session invented "DMOD" from a research doc that
recommended "Demod", then left `typr_lib`, `com.typr.app` and a `typr.exe` binary in
place. A half-applied rename reads to the user as no rename. Either rename the
user-visible surface completely and say plainly which internal identifiers you kept
and why, or do not start.

## 2026-07-20 — I broke the app by ignoring a memory note I already had

After renaming to Demod I built with `cargo build --release --bin demod` and copied
the exe. The stored project memory says verbatim: "Never build with plain `cargo build`
and hand-copy the exe: a Tauri exe built outside `tauri build` loads the dev server URL
(localhost:1420) forever." I did exactly that, twice, and shipped it to the daily
driver.

Symptom: the app launches, the window titles correctly, the process looks healthy at
26MB - and the WebView shows `chrome-error://chromewebdata/` because `devUrl` is
`http://localhost:1420` and nothing is serving it. Every DOM-level check fails while
every process-level check passes. Only `cdp-smoke` caught it (1/6). Extracting the
icon from the exe had "verified" the build minutes earlier, which proves an embedded
resource can be correct in a binary whose entire UI is dead.

Rules:
- `npm run tauri build` is THE build command. `cargo build` is for `cargo check`-grade
  feedback only, never for anything that gets deployed. `tauri build` runs
  `beforeBuildCommand` and embeds `frontendDist`; plain cargo does not.
- Run cdp-smoke against the exe BEFORE copying it over the daily driver, not after.
  DEPLOY.md section 1 already says this ("CDP smoke against the freshly built exe, not
  the installed one") and I skipped it.
- Verifying one embedded artefact is not verifying the build. Process-alive, correct
  window title and correct icon were all true of a completely broken binary.

## 2026-07-21 — Wave-family component map + fetch is blocked here (paste the source)

The recording/loading animations map to THREE distinct 21st.dev components. Only
ONE was ever pasted as real source; the rest are demo-only, so do not conflate them:

- Recording pill  = `Wave` from `@/components/ui/wave` -> https://21st.dev/@loading-ui/components/wave
  ("animated wave loading indicator built from bars that pulse vertically")
- Loading/processing = `WaveLoader` from `@/components/ui/wave-loader` -> https://21st.dev/@molecule-lab-rushil/components/wave-loader
  (deps: motion + class-variance-authority)
- Fallback = `SiriWave variant="fluid-dots"` from `@/components/ui/siri-wave`

SOURCE STATUS: only `waveform.tsx` (a different `Waveform` bars component) was ever
pasted (transcript line 2174). It is what the current pulsing-bars port is built from.
`Wave`, `WaveLoader`, `SiriWave` were given as DEMO usage only (`<Wave />`), never the
component body. `Wave` != `Waveform`.

FETCHING IS DEAD ON THIS MACHINE: 21st.dev `/r/<author>/<slug>` registry -> HTTP 403
(Cloudflare); component pages -> SPA shell, no source; loading-ui.com docs -> references
`<ComponentSource>` but does not inline it; curl -> HTTP 000 (AV TLS interception, same
wall that blocks new crates). So the ONLY way to get exact `Wave`/`WaveLoader` timing is
the user pasting the `.tsx` file, or `npx shadcn add @loading-ui/wave` in a throwaway
React project to materialize the file then read it. Do not guess the source again.

The overlay is vanilla `<canvas>` (no React/Tailwind): port the DRAW LOGIC into a canvas
loop, never drop the `.tsx` in.

## 2026-07-21 — Demod's own auto-edit is corrupting the user's dictations

During this session every dictated message arrived role-flipped and rambly ("You're
going to dictate some text and I'll clean it up...", "in the compacted version for
CloudMD..."). Root cause: the paste path runs `groq_auto_edit` (llama-3.1-8b-instant)
which is REWRITING the user's speech into assistant-style meta-text before it reaches
the chat. The app we are building is actively mangling the instructions used to build
it. Mitigation: default `autoEdits` to OFF (or tighten its system prompt to "fix
grammar/fillers only, never add or reinterpret content, never change voice/person").
When the user's messages read like an AI describing dictation, suspect this, not the user.
