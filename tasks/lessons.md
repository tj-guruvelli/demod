
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
