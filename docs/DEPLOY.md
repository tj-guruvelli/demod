# Typr Deploy Runbook

> Derived from an adversarial (LLM council) design review, 2026-07-20. Every step here
> exists because the council found the original plan hand-waved it. Do not skip a step
> because the build "looks fine" - the build looking fine is what the steps are checking.
> All shell blocks below are **PowerShell** (Windows 11, PowerShell 5.1) unless a block
> is explicitly marked cmd. Do not mix syntaxes within one block.

## 0. Preconditions (all must be true before starting)

- [ ] Both tracks finished and merged; `git status` clean on `ralph/wispr-parity`.
- [ ] Tripwire clean: `git diff <branchpoint>..ui/us012 --stat -- src-tauri/ src/overlay.html`
      is EMPTY (proves the UI track never crossed into Rust-owned files).
- [ ] `invoke()` command-name set on the UI branch is identical to the branch point.
- [ ] `cargo test --lib` EXIT=0. Run it UNPIPED and read the exit code:
      ```powershell
      cargo test --lib *> gate.txt; if ($?) { "EXIT=0" } else { "EXIT=$LASTEXITCODE" }
      ```
      A gate piped into a pager/`Select-Object` reports that command's status, not
      cargo's, and will show green on code that does not compile.

## 1. Merged-binary integration gate (the step nobody had)

Both tracks passing in isolation proves nothing about them coexisting. The first time
focus-guard paste, the quick menu, and the new settings UI live in one binary must NOT
be the moment the daily driver is overwritten.

- [ ] Merge `ui/us012` into `ralph/wispr-parity` locally. Resolve nothing blindly.
- [ ] `npm run build` EXIT=0 and `cargo test --lib` EXIT=0 on the MERGED tree.
      (`npm run build` runs `tsc && vite build` per `package.json` - confirmed against
      this repo's actual scripts, not assumed.)
- [ ] `npm run tauri build` EXIT=0. This invokes the Tauri CLI, which runs
      `beforeBuildCommand` (`npm run build`, per `src-tauri/tauri.conf.json`) and then
      bundles - this is also US-011 and the first end-to-end exercise of the patched
      VS/SDK workarounds on a real release.
- [ ] CDP smoke (`typr-e2e`) against the freshly built exe, not the installed one.

## 2. Replace the exe (Windows locks a running binary)

The old plan said "replace the exe." Windows will refuse: the running process holds
the file handle. There is no quit-and-replace logic in the build script.

Installed path confirmed to exist on this machine: `C:\Users\guruv\AppData\Local\Typr\typr.exe`.

```powershell
# PowerShell
taskkill /IM typr.exe /F
```

- [ ] CONFIRM the process is actually gone before copying:
      ```powershell
      Get-Process typr -ErrorAction SilentlyContinue
      ```
      This must return nothing.
- [ ] Back up first, in this order (never overwrite the only good copy):
      ```powershell
      Copy-Item "$env:LOCALAPPDATA\Typr\typr.exe" "$env:LOCALAPPDATA\Typr\typr.exe.bak" -Force
      ```
- [ ] Copy the new binary in, then relaunch:
      ```powershell
      Copy-Item "<path-to-freshly-built-typr.exe>" "$env:LOCALAPPDATA\Typr\typr.exe" -Force
      & "$env:LOCALAPPDATA\Typr\typr.exe"
      ```

## 3. Detect a bad deploy (".bak is not a rollback plan, it's a hope")

A rollback plan answers "how do I know it's bad" and "how fast can I be working again,"
not just "can I get the old bytes back." Bad-deploy signals:

- hotkey does nothing within 10s of launch, or
- the sidecar/model never comes up, or
- ANY scenario in the focus-guard QA script below fails, or
- settings fail to load (settings-format regression from the UI track).

### Restore (memorize this)

```powershell
# PowerShell
taskkill /IM typr.exe /F
Copy-Item "$env:LOCALAPPDATA\Typr\typr.exe.bak" "$env:LOCALAPPDATA\Typr\typr.exe" -Force
& "$env:LOCALAPPDATA\Typr\typr.exe"
```

Do NOT delete the `.bak` until the dogfood window below has passed.

## 4. Dogfood window (the canary, since there is no second machine)

There is no spare machine to canary on and one developer is simultaneously the only
reviewer, QA, and rollback operator. That single point of failure is accepted, so the
dogfood window has to carry the weight a staged rollout normally would.

- [ ] Run the FULL focus-guard QA script (see [QA-focus-guard.md](./QA-focus-guard.md))
      immediately after swapping, before doing any real work in the app.
- [ ] Keep the `.bak` in place for a full day of normal dictation use.
- [ ] If scenarios 2, 3, 5, 6, or 12 fail: restore immediately. Do not "try it once
      more" on the tool you depend on to work.

## 5. What is NOT covered by any automated test (state this honestly)

CDP cannot synthesize global hotkeys or real focus changes. So there is ZERO automated
coverage for: hotkey gestures (tap/hold/auto), the focus-guard paste decision, and the
quick menu. The manual QA script ([QA-focus-guard.md](./QA-focus-guard.md)) is the only
verification these three ever get. That is a known, accepted gap - not an oversight -
and it is why the script is mandatory rather than advisory.
