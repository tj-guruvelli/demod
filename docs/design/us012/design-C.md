# US-012 Design C — Power-User Density

Angle: keyboard-first, compact rows, everything visible without scrolling at **900x640**, inline
status for model download / sidecar health / hotkey capture. Vanilla TS + CSS, inline SVG only,
dark palette matched to `overlay.html` (`rgba(20,20,20,.85)` surfaces, `rgba(255,255,255,.14)`
hairlines, `#eaeaea`/`#d8d8d8` text, `#6fe08c`/`#4a9ff5`/`#e0c341`/`#f2b8b8` status accents).

Window target: resize the Tauri window to `900x640, resizable:false` (or resizable with a
`min-size` clamp — main.ts change only, no `tauri.conf.json` edit needed if the window is already
declared there; if it is, that one JSON field is the only non-index/main.ts touch, called out in
§6). Everything below assumes exactly this canvas; no scroll containers anywhere except History's
row list in a later story.

---

## 1. Sidebar IA — icons + inline SVG

6 sections per the story, ordered by what a power user checks constantly (top) → set-once (bottom).
16x16 viewBox, `stroke="currentColor" stroke-width="1.3"`, matches the existing nav-icon style in
`index.html` exactly (no fill, round caps) so the new icons don't look like a different icon set
than the current General/Engine/Recording ones.

| # | Section | Icon (lucide name) | Why here |
|---|---|---|---|
| 1 | General | `sliders-horizontal` | mic device + language + hotkey display — the settings you touch daily |
| 2 | Recording | `mic` | mode / sounds / overlay-visible toggles (current "Recording" section, unchanged scope) |
| 3 | Overlay | `layout-panel-top` (pill-shaped rect) | dock edge, always-visible mini toggle — new section per research §5, mostly honest-gated in v1 |
| 4 | Models | `cpu` | engine local/cloud + model picker + **download state** + sidecar health row (this design's focus) |
| 5 | History | `history` (clock with backward arc) | retention setting only in v1 (no transcript list yet — honestly gated) |
| 6 | Privacy | `shield` | data-local statement + retention days (mostly static copy in v1) |

Inline SVG paths (drop-in replacements for the `<svg class="nav-icon">` block per nav item):

```html
<!-- General: sliders-horizontal -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <path d="M2 4.5h7M11.5 4.5H14M2 8h3M5.5 8H14M2 11.5h9M13 11.5h1"
        stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
  <circle cx="9" cy="4.5" r="1.6" stroke="currentColor" stroke-width="1.3"/>
  <circle cx="4.5" cy="8" r="1.6" stroke="currentColor" stroke-width="1.3"/>
  <circle cx="12" cy="11.5" r="1.6" stroke="currentColor" stroke-width="1.3"/>
</svg>

<!-- Recording: mic (kept from current markup, reused verbatim) -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <rect x="6" y="2.5" width="4" height="7" rx="2" stroke="currentColor" stroke-width="1.2"/>
  <path d="M4 8.5a4 4 0 008 0M8 12.5v1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
</svg>

<!-- Overlay: layout-panel-top -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <rect x="2" y="3" width="12" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
  <path d="M2 6.5h12" stroke="currentColor" stroke-width="1.2"/>
</svg>

<!-- Models: cpu -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <rect x="4.5" y="4.5" width="7" height="7" rx="1" stroke="currentColor" stroke-width="1.2"/>
  <rect x="6.5" y="6.5" width="3" height="3" stroke="currentColor" stroke-width="1.1"/>
  <path d="M6 2v2M10 2v2M6 12v2M10 12v2M2 6h2M2 10h2M12 6h2M12 10h2"
        stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
</svg>

<!-- History: history (clock w/ backward arc) -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <path d="M3 6.5A5.5 5.5 0 118.5 13.5M3 6.5V3M3 6.5H6.5"
        stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M8.5 6v3l2 1.3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>

<!-- Privacy: shield -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <path d="M8 2 2.5 3.8v4c0 3.3 2.2 5.6 5.5 6.4 3.3-.8 5.5-3.1 5.5-6.4v-4L8 2Z"
        stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
  <path d="M5.8 8l1.6 1.6 2.8-3.2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
```

Extra small icons used inline in row status (not nav): a 10px **download** arrow-into-tray (Models
row action), a 10px **check** (downloaded state, reuse the overlay's own tick path
`<polyline points="5 12.5 10 17.5 19 7"/>` rescaled to 0 0 16 16 as `points="3.5 8.5 6.5 11.5 12.5 4.5"`),
and a 10px **key** glyph for the hotkey-capture affordance:

```html
<!-- download-tray, 10x10 viewBox 0 0 10 10 -->
<svg width="10" height="10" viewBox="0 0 10 10" fill="none">
  <path d="M5 1v5.2M2.6 4.6 5 7l2.4-2.4" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M1.5 8.5h7" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
</svg>

<!-- key, 12x12 -->
<svg width="12" height="12" viewBox="0 0 12 12" fill="none">
  <circle cx="4" cy="8" r="2.3" stroke="currentColor" stroke-width="1.1"/>
  <path d="M5.7 6.3 10 2M8 4l1.2 1.2M9.6 2.4l1.2 1.2" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
</svg>
```

---

## 2. Layout spec (px values, 900x640)

```
┌ #titlebar  900 x 32 (drag region, unchanged) ─────────────────────────────┐
├ #app-body  900 x 608 ──────────────────────────────────────────────────────┤
│┌ #sidebar 208 x 608 ┐┌ #content 692 x 608 ─────────────────────────────────┐│
││ .sidebar-header      │ .section-header  692 x 56, padding 20 24          ││
││   56px               │   h2 18px/600, .section-desc 12.5px muted         ││
││ .sidebar-search       │ .settings-list  padding 0 24 24 24, gap 6         ││
││   36px  [/ Search]    │   .setting-row: 692-48=644 wide, min-height 48,   ││
││ nav (6 items)         │     padding 10 0, border-bottom hairline          ││
││   34px each, 6 items  │     row = [label col 320px | control col 300px,   ││
││   = 204px             │            right-aligned, gap 24]                ││
││ .sidebar-divider      │   dense rows: no card wrapper, no 16px card       ││
││ .sidecar-health       │   padding — that's the power-user density trade   ││
││   72px status block  │ Models section gets 2 EXTRA compact rows for      ││
││ .sidebar-footer       │   sidecar health + hotkey capture affordance      ││
││   28px (version+kbd)  │   (both fit inside the same 48px row rhythm)     ││
│└───────────────────────┘└────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

Exact box model:
- Window: `900 x 640`. Titlebar `32px` (unchanged, drag region). Body: `900 x 608`.
- Sidebar: fixed `208px` wide (was unspecified/flex before — now pinned so nav labels never
  reflow). `.sidebar-header` 56px (icon+name row 32px + status-indicator row, kept from current).
- **New**: `.sidebar-search` — a `Ctrl+K`-style command row, 36px tall, `8px` margin, not wired to
  real filtering in v1 (6 items don't need it) → **honestly omitted**, replaced by a plain keyboard
  hint row instead (see §5, "search" was cut, not gated-disabled, because a disabled search box
  would itself read as a dead control).
- Nav: 6 items × `34px` row height (dense: current is ~38-40px with more padding), `13px` label,
  `16px` icon, `10px` gap, `12px` left padding. Active state = left `2px` accent bar + subtle bg,
  no layout shift (border is `transparent` at rest, `#4a9ff5` at 40% opacity when active — reserved
  space, not inserted).
- `.sidebar-divider`: 1px hairline, 8px vertical margin.
- **New** `.sidecar-health` block: 72px, always visible (not a nav item — global status, power users
  want it pinned regardless of which section they're in). 3 stacked 20px rows: Sidecar, Model,
  Hotkey — each a 6px dot + 11px label + right-aligned 10px state text.
- `.sidebar-footer`: 28px, version text + a `?` kbd-hint ("Ctrl+, to reopen" style, static text).
- Content pane: `692px` wide. `.section-header` 56px. `.settings-list` rows are dense: `48px`
  min-height (current scaffold rows run ~56-64px with more breathing room), `1px` hairline divider
  between rows instead of the current spaced-card look, label column fixed `320px`, control column
  right-aligned filling the remainder (`~300px`), `24px` column gap. This is the literal "compact
  rows" density lever vs the other two design angles.
- Toggle-groups, selects, inputs: all `28px` tall (down from an implicit ~32-34px) to fit the 48px
  row with room for a hint line beneath when needed.

---

## 3. Complete CSS (shell + nav + rows + toggles)

Append/replace into `src/style.css` (file not read in full here since it's not in the given context
set, but every existing selector name below — `#sidebar`, `.nav-item`, `.toggle-btn`, `.setting-row`
etc. — is reused verbatim from `index.html`'s current classes so this is additive/override CSS, not
a rename). New selectors are marked `/* NEW */`.

```css
:root {
  --bg-0: #141414;
  --bg-1: rgba(20, 20, 20, 0.85);
  --bg-2: rgba(255, 255, 255, 0.04);
  --bg-2-hover: rgba(255, 255, 255, 0.07);
  --border: rgba(255, 255, 255, 0.14);
  --border-soft: rgba(255, 255, 255, 0.08);
  --text-hi: #eaeaea;
  --text-lo: #9a9a9a;
  --accent: #4a9ff5;
  --accent-dim: rgba(74, 159, 245, 0.4);
  --ok: #6fe08c;
  --warn: #e0c341;
  --err: #f2b8b8;
  --radius: 8px;
  --row-h: 48px;
}

html, body { background: var(--bg-0); color: var(--text-hi); }

#app-body { display: grid; grid-template-columns: 208px 1fr; height: 608px; }

/* ---------- sidebar shell ---------- */
#sidebar {
  width: 208px;
  display: flex;
  flex-direction: column;
  background: var(--bg-1);
  border-right: 1px solid var(--border-soft);
}

.sidebar-header { height: 56px; display: flex; flex-direction: column; justify-content: center; gap: 4px; padding: 0 14px; }
.app-identity { display: flex; align-items: center; gap: 8px; }
.app-name { font-size: 13px; font-weight: 600; }

#status-indicator { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-lo); }
#status-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--text-lo); transition: background-color 150ms ease-out; }
#status-dot.ready { background: var(--text-lo); }
#status-dot.recording { background: var(--warn); }
#status-dot.transcribing { background: var(--accent); }

/* NEW: quick keyboard hint row replacing a would-be dead search box */
.sidebar-hint {
  height: 30px;
  margin: 4px 10px 2px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px;
  font-size: 10.5px;
  color: var(--text-lo);
  background: var(--bg-2);
  border-radius: 6px;
}
.sidebar-hint kbd {
  font-size: 10px;
  padding: 1px 5px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.3);
}

/* ---------- nav ---------- */
#sidebar-nav { display: flex; flex-direction: column; padding: 4px 8px; gap: 1px; }

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 34px;
  padding: 0 10px 0 12px;
  border-radius: 6px;
  border-left: 2px solid transparent;
  font-size: 13px;
  color: var(--text-lo);
  text-decoration: none;
  cursor: pointer;
  transition: background-color 150ms ease-out, color 150ms ease-out, border-color 150ms ease-out;
  user-select: none;
}
.nav-item:hover { background: var(--bg-2-hover); color: var(--text-hi); }
.nav-item.active {
  background: var(--bg-2);
  color: var(--text-hi);
  border-left-color: var(--accent);
}
.nav-icon { flex: none; opacity: 0.85; }
.nav-item.active .nav-icon { opacity: 1; }

/* per-item inline status chip (e.g. "2 new" / a dot) reserved, empty by default */
.nav-item .nav-chip { margin-left: auto; font-size: 10px; color: var(--text-lo); }

.sidebar-divider { height: 1px; margin: 6px 12px; background: var(--border-soft); }

/* NEW: pinned sidecar/model/hotkey health block, always visible regardless of active section */
.sidecar-health { padding: 6px 14px 8px; display: flex; flex-direction: column; gap: 4px; }
.health-row { display: flex; align-items: center; gap: 6px; height: 18px; font-size: 10.5px; }
.health-dot { width: 6px; height: 6px; border-radius: 50%; flex: none; transition: background-color 150ms ease-out; }
.health-dot.ok { background: var(--ok); }
.health-dot.warn { background: var(--warn); }
.health-dot.err { background: var(--err); }
.health-dot.off { background: var(--text-lo); opacity: 0.5; }
.health-label { color: var(--text-lo); }
.health-value { margin-left: auto; color: var(--text-hi); font-variant-numeric: tabular-nums; }

.sidebar-footer {
  margin-top: auto;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 14px;
  font-size: 10.5px;
  color: var(--text-lo);
  border-top: 1px solid var(--border-soft);
}

/* ---------- content shell ---------- */
#content { overflow: hidden; }

.content-section { display: none; opacity: 0; transform: translateY(4px); }
.content-section.active {
  display: block;
  opacity: 1;
  transform: translateY(0);
  transition: opacity 150ms ease-out, transform 150ms ease-out;
}

.section-header { padding: 18px 24px 12px; border-bottom: 1px solid var(--border-soft); }
.section-title { font-size: 17px; font-weight: 600; margin: 0 0 2px; }
.section-desc { font-size: 12px; color: var(--text-lo); margin: 0; }

/* ---------- dense rows (the power-user-density lever) ---------- */
.settings-list { padding: 4px 24px 24px; display: flex; flex-direction: column; }

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  min-height: var(--row-h);
  padding: 8px 0;
  border-bottom: 1px solid var(--border-soft);
}
.setting-row:last-child { border-bottom: none; }
.setting-row.hidden { display: none; }

.setting-label { display: flex; flex-direction: column; gap: 1px; width: 320px; flex: none; }
.label-text { font-size: 12.5px; font-weight: 500; }
.label-hint { font-size: 11px; color: var(--text-lo); }

.setting-control { display: flex; align-items: center; justify-content: flex-end; gap: 8px; flex: 1; min-width: 0; }

/* NEW: honest-gated row modifier — dims the row and swaps the hint for a reason string */
.setting-row.gated { opacity: 0.6; }
.setting-row.gated .label-hint { color: var(--warn); }

/* ---------- toggle group (existing, restated at the new density) ---------- */
.toggle-group { display: flex; background: var(--bg-2); border: 1px solid var(--border-soft); border-radius: 6px; padding: 2px; }
.toggle-btn {
  appearance: none; border: none; background: transparent; color: var(--text-lo);
  font: inherit; font-size: 11.5px; padding: 5px 10px; border-radius: 4px; cursor: pointer;
  transition: background-color 150ms ease-out, color 150ms ease-out;
}
.toggle-btn:hover { color: var(--text-hi); }
.toggle-btn.active { background: var(--accent-dim); color: var(--text-hi); }
.toggle-btn:disabled { cursor: not-allowed; opacity: 0.45; }

/* selects / inputs at the new 28px row height */
select, input[type="password"], input[type="text"] {
  height: 28px; padding: 0 8px; font-size: 12px;
  background: var(--bg-2); border: 1px solid var(--border-soft); border-radius: 6px; color: var(--text-hi);
}
select:focus, input:focus { outline: none; border-color: var(--accent-dim); }

.btn-primary {
  height: 28px; padding: 0 12px; font-size: 12px; font-weight: 500;
  background: var(--accent); color: #0c1420; border: none; border-radius: 6px; cursor: pointer;
  transition: opacity 150ms ease-out, transform 150ms ease-out;
}
.btn-primary:hover { opacity: 0.9; }
.btn-primary:active { transform: scale(0.97); }
.btn-primary:disabled { background: var(--bg-2); color: var(--text-lo); cursor: not-allowed; }

/* NEW: model row — picker + inline download/health state, no dead spinner */
.model-row { display: flex; align-items: center; gap: 8px; }
.model-state { display: flex; align-items: center; gap: 5px; font-size: 11px; color: var(--text-lo); }
.model-state.ok { color: var(--ok); }
.model-state.downloading { color: var(--accent); }
.model-state svg { flex: none; }

#download-progress { height: 3px; border-radius: 2px; background: var(--bg-2); overflow: hidden; margin-top: 6px; }
#progress-fill { height: 100%; background: var(--accent); width: 0%; transition: width 150ms ease-out; }

/* NEW: hotkey capture affordance — real if a capture command exists, else honest read-only */
.hotkey-display { display: flex; align-items: center; gap: 8px; }
kbd#hotkey-text {
  font: 11.5px "IBM Plex Sans", monospace; padding: 4px 8px; border: 1px solid var(--border);
  border-radius: 5px; background: rgba(0, 0, 0, 0.3); color: var(--text-hi);
}
.hotkey-change-btn {
  display: flex; align-items: center; gap: 5px; height: 26px; padding: 0 10px; font-size: 11px;
  background: var(--bg-2); border: 1px solid var(--border-soft); border-radius: 6px; color: var(--text-lo);
  cursor: pointer; transition: background-color 150ms ease-out, color 150ms ease-out;
}
.hotkey-change-btn:hover:not(:disabled) { background: var(--bg-2-hover); color: var(--text-hi); }
.hotkey-change-btn:disabled { cursor: not-allowed; opacity: 0.5; }
.hotkey-change-btn.capturing { color: var(--warn); border-color: var(--warn); }

/* NEW: honest-gate badge, used anywhere a control exists but isn't backed yet */
.gate-badge {
  font-size: 9.5px; text-transform: uppercase; letter-spacing: 0.03em;
  padding: 2px 6px; border-radius: 4px; color: var(--warn);
  background: rgba(224, 195, 65, 0.12); border: 1px solid rgba(224, 195, 65, 0.3);
}
```

---

## 4. main.ts wiring plan

### 4.1 Existing settings — bind unchanged, respect the cached-object gotcha

Every current binding stays exactly as-is (`micSelect`, `engineLocal/Cloud`, `modelSelect`,
`downloadBtn`, `groqKey`, `modeAuto/Toggle/Ptt`, `hotkeyText`, `soundsOn/Off`, `overlayOn/Off`) —
same element IDs, same `set*` functions mutating `currentSettings` then calling `saveSettings()`.
The only structural change is which `<section>` each row lives in (General gets mic + hotkey
display moved in; Recording keeps mode/sounds/overlay-visible; Models gets engine+model+groq).

**Cached-object gotcha (progress.txt line 21-24)**: `saveSettings()` still sends the *whole*
`currentSettings` object, not a diff. Any Rust-owned field (currently none read by the frontend,
but `overlayX/overlayY` are the documented example) must never be blanked by a stale read. Rule
applied to every NEW control below: mutate `currentSettings` in memory first via a `set*` helper,
exactly like the existing ones, so the next `saveSettings()` call carries the update forward rather
than overwriting it with an unrelated field's snapshot. Do not introduce a second "read settings
fresh" path that could race the cached object.

### 4.2 New DOM bindings (all inside existing sections, no new files)

```ts
// Sidecar/model/hotkey health block (sidebar, always visible)
const healthSidecar = document.getElementById("health-sidecar")!;   // .health-value
const healthModel = document.getElementById("health-model")!;       // .health-value
const healthHotkey = document.getElementById("health-hotkey")!;     // .health-value
const healthDots = {
  sidecar: document.getElementById("health-dot-sidecar")!,
  model: document.getElementById("health-dot-model")!,
  hotkey: document.getElementById("health-dot-hotkey")!,
};

// Hotkey capture affordance (General section)
const hotkeyChangeBtn = document.getElementById("hotkey-change-btn") as HTMLButtonElement;
```

### 4.3 Sidecar health row — **honest gate, no Rust command exists yet**

`check_model_downloaded` tells us model presence, not whether the whisper-server sidecar process
is alive. No `get_sidecar_status` command exists in `main.rs` today (only the six commands
enumerated in the codebase pass: `get_settings`, `save_settings`, `list_microphones`,
`get_recording_state`, `check_model_downloaded`, `download_model`, `toggle_recording`). Options:

- **v1 (ship now, no NEW command)**: derive an honest proxy from `get_recording_state` (already
  polled/pushed via the `recording-state` event) — if the app is in `Transcribing` state the
  sidecar is provably alive; otherwise show `"Idle"` with the `off` dot, not a fabricated "Running"
  claim. Label: `Sidecar` / value `Idle` (grey dot) or `Active` (green dot, driven by the same
  `listen<string>("recording-state", ...)` handler that already updates `#status-dot`).
- **NEW (flagged, not built this story)**: `get_sidecar_health() -> { pid: Option<u32>, alive: bool }`
  in `main.rs`, backed by checking the child process handle already held wherever the sidecar is
  spawned (out of the read-only context set for this task, so not located/verified here — flag for
  the implementer to grep `Command::new` / `sidecar` in `src-tauri/src`). Until that command lands,
  the UI must use the v1 proxy above, not a static "Running" string.

```ts
listen<string>("recording-state", (event) => {
  // ...existing statusDot/statusText logic unchanged...
  const sidecarActive = event.payload === "Transcribing";
  healthDots.sidecar.className = "health-dot " + (sidecarActive ? "ok" : "off");
  healthSidecar.textContent = sidecarActive ? "Active" : "Idle";
});
```

### 4.4 Model download state — real, wired to existing commands

```ts
async function checkModelStatus() {
  const downloaded = await invoke<boolean>("check_model_downloaded", { modelSize: modelSelect.value });
  downloadBtn.textContent = downloaded ? "" : "Download";
  downloadBtn.disabled = downloaded;
  modelStateEl.className = "model-state" + (downloaded ? " ok" : "");
  modelStateEl.innerHTML = downloaded
    ? '<svg width="10" height="10" viewBox="0 0 10 10"><polyline points="1.5 5.2 3.8 7.5 8.5 2.5" stroke="currentColor" stroke-width="1.2" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg> Downloaded'
    : "Not downloaded";
  healthDots.model.className = "health-dot " + (downloaded ? "ok" : "off");
  healthModel.textContent = downloaded ? modelSelect.value : "none";
}
```

`download_model` invoke + `download-progress` listener stay exactly as they are today (real
progress payload, no fabricated percentages) — only the progress bar's container moves under the
new row layout; the event contract is untouched.

### 4.5 Hotkey capture affordance — **honestly gated, no NEW command in this story**

No `capture_hotkey`/`set_hotkey` command exists (`get_settings`/`save_settings` round-trip the
`hotkey` field, but nothing lets the frontend register a new global shortcut). The story text says
*"a 'change' affordance only if a real capture flow exists — otherwise read-only with explanation"*
— so ship the disabled path, clearly labeled, not a fake "click to record new hotkey" prompt:

```html
<div class="setting-row">
  <div class="setting-label">
    <span class="label-text">Hotkey</span>
    <span class="label-hint">Global keyboard shortcut to trigger recording</span>
  </div>
  <div class="setting-control hotkey-display">
    <kbd id="hotkey-text">Cmd+Shift+D</kbd>
    <button id="hotkey-change-btn" class="hotkey-change-btn" disabled title="Hotkey re-binding isn't wired to Rust yet">
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none"><circle cx="4" cy="8" r="2.3" stroke="currentColor" stroke-width="1.1"/><path d="M5.7 6.3 10 2M8 4l1.2 1.2M9.6 2.4l1.2 1.2" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/></svg>
      Change
    </button>
    <span class="gate-badge">read-only</span>
  </div>
</div>
```

```ts
// No click handler is attached while disabled — a disabled+labeled button is not a dead
// button per the story's own definition ("honestly disabled with a reason"). If/when a
// capture command ships (NEW: register_hotkey_capture / commit_hotkey — names TBD by
// whoever implements the Rust side), remove `disabled`, remove the badge, and wire:
//   hotkeyChangeBtn.addEventListener("click", startCapture) where startCapture puts the
//   button in `.capturing` state, listens for the next global-shortcut keydown combo via
//   a dedicated Tauri event, previews it in the kbd element, and calls a new
//   `invoke("save_settings", { settings: { ...currentSettings, hotkey: newCombo } })`
//   only on explicit confirm (Enter) — never live-writes on every keystroke.
```

### 4.6 Overlay / History / Privacy sections — mostly gated in v1

- **Overlay**: `overlayOn/overlayOff` toggle moves here from Recording (real, existing
  `overlayVisible` field). Dock-edge picker and "always-visible mini window" are **new UI with no
  backing field or command** → render as a disabled toggle-group row with `.gated` class and hint
  text `"Dock-edge picking isn't implemented yet — overlay is bottom-center only"`. Do not invoke
  anything on click.
- **History**: no transcript store, no Rust command. Render one static row: `label-text="Transcript
  history"`, control = `<span class="gate-badge">not yet stored</span>`, no button at all (a
  disabled button implies a future click target; a badge-only row implies "feature doesn't exist",
  which is the honest statement here).
- **Privacy**: static copy row, `"All processing happens on-device. Nothing leaves this machine."`
  — no control, no gate needed (it's a true statement about the current architecture, not a
  promise about a control).

### 4.7 Section nav — unchanged mechanism, six targets now

The existing `navItems.forEach(... data-section ...)` click handler and `.active` class toggling
is reused verbatim; only the HTML gains 3 more `<a class="nav-item" data-section="overlay|models|history-privacy">`
entries (History+Privacy can be two nav items or one combined "History & Privacy" section — spec
says 6 named sections, so keep them separate to match the acceptance criterion literally).

---

## 5. Honest-gating plan (summary table)

| Control | Backing today? | Treatment |
|---|---|---|
| Mic picker | Yes (`list_microphones`) | Fully real, unchanged |
| Engine local/cloud | Yes (`engine` field) | Fully real, unchanged |
| Model picker + download | Yes (`check_model_downloaded`, `download_model`) | Fully real, unchanged, richer state chip |
| Groq API key | Yes (`groqApiKey` field) | Fully real, unchanged (plaintext today — flag for a future Privacy-section note re: Credential Manager, not fixed in this story) |
| Recording mode / sounds / overlay-visible | Yes | Fully real, unchanged, relocated |
| Hotkey **display** | Yes (`hotkey` field, formatted) | Real, unchanged |
| Hotkey **change** | No command | Disabled button + `read-only` badge + tooltip reason. No click handler attached. |
| Sidecar health | No dedicated command | Proxy from existing `recording-state` event (Active/Idle), not fabricated uptime/PID |
| Dock-edge / always-visible mini (Overlay) | No field, no command | Disabled toggle-group row, `.gated`, hint explains why |
| Transcript history | No store | Badge-only row, no button |
| Privacy retention days | No field | Static statement only, no control offered (nothing to gate — don't invent a slider for a setting that does nothing) |

Rule applied throughout: a control is either (a) wired to a real command/field, or (b) rendered
disabled with a visible reason string, or (c) not rendered as an interactive control at all when
there is genuinely nothing behind it (badge/static text). Never (d) a clickable no-op.

---

## 6. Migration diff summary vs current `index.html`

- Window size: current scaffold has no explicit content-fit target; this design assumes/needs the
  Tauri window resized to `900x640` — a one-line change in `tauri.conf.json`'s window config if
  present, otherwise a `getCurrentWindow().setSize()` call in `main.ts` init. Flagged as the one
  cross-file touch beyond index.html/main.ts/style.css.
- Sidebar: `#sidebar` width goes from implicit/flex to a fixed `208px`; nav items shrink `~38px →
  34px`; **3 new nav items** (Overlay, Models, History, Privacy — 4 actually, since Engine's
  content is absorbed into Models and a new Overlay/History/Privacy trio is added) replacing the
  current 3 (General/Engine/Recording → General/Recording/Overlay/Models/History/Privacy).
- New sidebar block: `.sidebar-hint` (keyboard hint, static) and `.sidecar-health` (3-row live
  status) inserted between nav and footer — no such block exists today.
- Content rows: `.setting-row` padding/height tightened (`~56-64px` implicit → fixed `48px` +
  hairline dividers instead of the current spacing), column widths pinned (`320px` label / flexible
  control) instead of flex-implicit — this is the literal density change vs a spacier design.
- Engine section is folded into Models (engine toggle + model picker + Groq key all live under one
  nav item instead of their own "Engine" section) — matches the story's 6-section list, which has
  no separate "Engine" entry.
- Hotkey row moves from Recording → General, gains a disabled "Change" button + `read-only` badge
  (net-new markup, zero behavior since it's inert).
- Net-new, fully inert (by design) rows: Overlay dock-edge picker, History row, Privacy statement —
  zero new Rust invokes added by this story; all new interactivity is either a relabeled existing
  binding or explicitly disabled.
- CSS: net-new custom properties (`:root` palette vars) plus ~10 new selectors (`.sidebar-hint`,
  `.sidecar-health`/`.health-*`, `.gate-badge`, `.model-state`, `.hotkey-change-btn`), all additive;
  no existing selector is renamed, so no dangling references in `main.ts`'s current `getElementById`
  calls.
