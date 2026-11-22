# Demod — project rules for Claude Code

Demod is a Windows dictation app (Tauri v2 + vanilla TS + whisper.cpp). These rules
are repo-specific and sit on top of the global `~/.claude/CLAUDE.md`.

## No hard-coded paths (non-negotiable)

Never write an absolute filesystem path (`C:\Users\...`, `C:/Users/...`, `/c/Users/...`,
`/home/...`) into code, build config, scripts, prompts, JSON, or docs in this repo.
The repo must build and run from any checkout location on any machine.

Resolve paths dynamically instead:

| Need                         | Use                                                          |
|------------------------------|--------------------------------------------------------------|
| Repo root in a script        | the script's own location or the cwd it is run from          |
| Repo root in `build.rs`      | `env!("CARGO_MANIFEST_DIR")`                                 |
| User data at runtime         | Tauri `app.path()` (`app_data_dir`, `app_config_dir`)        |
| Install location in docs     | `%LOCALAPPDATA%\Demod`                                       |
| Config location in docs      | `%APPDATA%\com.demod.app`                                    |
| Machine-only toolchain fixes | the user-level `~/.cargo/config.toml`, never the repo        |

`src-tauri/.cargo/config.toml` is committed and portable (`+crt-static` only). Do not
add machine-specific entries to it. Before every commit run
`git grep -n -i -E "C:[/\\\\]+Users|/c/Users|/home/"` and fix any hit outside
`tasks/lessons.md`.

## Other load-bearing rules

- Bundle identifier `com.demod.app` never changes (Tauri derives the user-data dir from it).
- Run `cargo check`/`cargo test` from inside `src-tauri/`, otherwise `.cargo/config.toml` is not read.
- Build deployable exes only with `npm run tauri build`; a plain `cargo build` exe embeds
  the dev-server URL and ships a dead WebView.
- Overlay animations are transform/opacity only; no polling loops (event-driven).
- Prefer `CARGO_BUILD_JOBS=6` for full rebuilds; heavy parallel rustc has crashed this
  machine's kernel before.
- Installing Demod on another PC never requires cloning this repo: ship the NSIS/MSI
  installer from `src-tauri/target/release/bundle/` via GitHub Releases. Keep the README
  **Install** section saying so.
