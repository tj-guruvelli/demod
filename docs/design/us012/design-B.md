# US-012 Design B — "Typr-Native Minimal"

**Angle:** don't import a design system (Linear/indigo-accent look the current scaffold half-copied) — derive every value from `src/overlay.html`, the one surface TJ already approved. Same near-black layers (`rgba(20,20,20,.85/.95)`), same hairline border (`rgba(255,255,255,.14)`), same lozenge/pill radii (`999px`), same 150ms opacity+transform grammar, same system font stack. No indigo. The accent *is* white-on-black, with the overlay's status colors (`#e0c341` / `#4a9ff5` / `#6fe08c`) reserved for state, not decoration. Fewest sections that honestly cover every existing setting: **6** (General, Recording, Overlay, Models, History, Privacy) — matches the acceptance criteria and the research doc's planned sidebar exactly, dropping nothing, inventing nothing.

Read for this design: `index.html`, `src/main.ts`, `src/overlay.html`, `src/style.css`, `docs/RESEARCH-wispr-superwhisper.md`, `progress.txt` (Codebase Patterns), `src-tauri/src/{main,settings,audio}.rs` (confirmed real commands: `get_settings`, `save_settings`, `list_microphones`, `check_model_downloaded`, `download_model`, `toggle_recording`, `get_recording_state`; confirmed wire fields incl. `overlayX`/`overlayY` already in `Settings` but never surfaced in UI; confirmed **no** hotkey-capture, history, or retention commands exist).

---

## 1. Sidebar IA — 6 sections, icon choices, real SVG path data

All icons: `viewBox="0 0 16 16"`, `stroke="currentColor"`, `stroke-width="1.2"`, `stroke-linecap="round"`, `stroke-linejoin="round"`, `fill="none"` — matching the hand-style already set by the existing General/Engine icons in `index.html` (not a literal lucide asset drop-in; redrawn at the app's existing 16px/1.2 grid, each named after the lucide icon it's modeled on so a future dependency swap is a 1:1 lookup).

| # | Section | Covers (existing settings) | lucide model | 
|---|---|---|---|
| 1 | General | microphone, hotkey (read-only display) | `mic` |
| 2 | Recording | recordingMode, interactionSounds | `audio-waveform` |
| 3 | Overlay | overlayVisible, overlayX/overlayY (dock position) | `app-window` |
| 4 | Models | engine, whisperModel, groqApiKey, download | `cpu` |
| 5 | History | *(none yet — honest placeholder)* | `history` |
| 6 | Privacy | *(none yet — honest placeholder)* | `shield` |

```html
<!-- 1. General — mic -->
<svg class="nav-icon" viewBox="0 0 16 16" fill="none">
  <rect x="6" y="1.5" width="4" height="7.5" rx="2" stroke="currentColor" stroke-width="1.2"/>
  <path d="M3.5 7.5a4.5 4.5 0 009 0M8 12v2M6 14h4" stroke="currentColor" stroke-width="1.2"/>
</svg>

<!-- 2. Recording — audio-waveform (echoes the overlay's own listening bars) -->
<svg class="nav-icon" viewBox="0 0 16 16" fill="none">
  <path d="M1.5 8h1.5M4.5 4.5v7M7.5 2v12M10.5 5v6M13 6.5v3M14.5 8H14"
        stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
</svg>

<!-- 3. Overlay — app-window (a floating pill/panel, the overlay itself) -->
<svg class="nav-icon" viewBox="0 0 16 16" fill="none">
  <rect x="1.5" y="3" width="13" height="10" rx="2" stroke="currentColor" stroke-width="1.2"/>
  <path d="M1.5 6h13" stroke="currentColor" stroke-width="1.2"/>
  <circle cx="3.7" cy="4.5" r=".55" fill="currentColor"/>
</svg>

<!-- 4. Models — cpu -->
<svg class="nav-icon" viewBox="0 0 16 16" fill="none">
  <rect x="4" y="4" width="8" height="8" rx="1.2" stroke="currentColor" stroke-width="1.2"/>
  <rect x="6.3" y="6.3" width="3.4" height="3.4" rx=".6" stroke="currentColor" stroke-width="1.1"/>
  <path d="M6 1.5v2M10 1.5v2M6 12.5v2M10 12.5v2M1.5 6h2M1.5 10h2M12.5 6h2M12.5 10h2"
        stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
</svg>

<!-- 5. History — history (clock + rewind sweep) -->
<svg class="nav-icon" viewBox="0 0 16 16" fill="none">
  <path d="M2.5 5.5A6 6 0 118.5 14" stroke="currentColor" stroke-width="1.2"/>
  <path d="M2.5 2.5v3h3" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
  <path d="M8.5 6v3l2 1.3" stroke="currentColor" stroke-width="1.2"/>
</svg>

<!-- 6. Privacy — shield-check -->
<svg class="nav-icon" viewBox="0 0 16 16" fill="none">
  <path d="M8 1.5l5 2v3.8c0 3.4-2.2 5.7-5 7.2-2.8-1.5-5-3.8-5-7.2V3.5l5-2z"
        stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
  <path d="M5.7 8l1.6 1.6 3-3.4" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
</svg>
```

Badge glyph (honest-gate pill, used in §5), one shared inline SVG:

```html
<svg class="badge-icon" viewBox="0 0 16 16" fill="none">
  <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.2"/>
  <path d="M8 5.2v3.3M8 10.8h.01" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
</svg>
```

---

## 2. Layout spec (px values)

```
#app                 100vw x 100vh, column flex
  #titlebar          height: 38px  (unchanged — drag region)
  #app-body          flex: 1, row flex
    #sidebar         width: 216px, fixed
      .sidebar-header  padding: 18px 16px 14px
        .app-identity  20px icon-glyph box + 13px name, gap 8px
        #status-indicator  pill, padding 5px 10px, radius 999px (overlay lozenge radius)
      #sidebar-nav   padding: 6px 8px, gap 2px
        .nav-item    height 32px, padding 0 10px, radius 8px
        .nav-icon    16x16, opacity .55 → 1 active
      .sidebar-footer  padding 12px 16px, border-top hairline
    #content         flex: 1, overflow-y auto
      .content-section  padding: 36px 44px, max-width 640px
        .section-header  margin-bottom 24px
          .section-title  15px / 600 / -0.01em
          .section-desc   12.5px / var(--text-secondary)
        .card            radius 12px, padding 4px 0, border hairline, background var(--surface)
          .setting-row   padding 14px 18px, gap 24px, border-bottom hairline (last-child none)
            .setting-label  min-width 176px
            .setting-control  flex 1, justify-content flex-end
```

Section max-width caps at 640px (not full-bleed) — cards read as a focused column against the black field, same restraint as the overlay's own tight lozenge silhouette rather than a sprawling settings-panel look.

Section switch: outgoing section `opacity:0, translateY(-4px)`, incoming `opacity:1, translateY(0)`, both 150ms, matching overlay's `150ms ease-out`/`ease-in` split exactly (in fast, out fast, no bounce).

---

## 3. Complete CSS

Replaces `src/style.css` wholesale (same file, same selectors where names carry over — see §6 diff for exactly what changes vs. stays).

```css
:root {
  /* Pulled 1:1 from overlay.html's own values — this IS the app's palette now. */
  --bg:             #0a0a0a;
  --sidebar-bg:     #0e0e0e;
  --surface:        rgba(20, 20, 20, 0.85);
  --surface-solid:  #141414;
  --surface-hover:  rgba(255, 255, 255, 0.06);
  --surface-active: rgba(255, 255, 255, 0.1);
  --text:           #eaeaea;
  --text-secondary: #9a9a9a;
  --text-tertiary:  #5a5a5a;
  --white-accent:   #ffffff;
  --green:          #6fe08c;
  --blue:           #4a9ff5;
  --yellow:         #e0c341;
  --red:            #f2b8b8;
  --border:         rgba(255, 255, 255, 0.10);
  --border-strong:  rgba(255, 255, 255, 0.14);
  --radius:         8px;
  --radius-lg:      12px;
  --radius-pill:    999px;
  --font:           system-ui, -apple-system, "Segoe UI", sans-serif;
  --ease-in:        150ms ease-in;
  --ease-out:       150ms ease-out;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  font-family: var(--font);
  background: var(--bg);
  color: var(--text);
  user-select: none;
  -webkit-user-select: none;
  font-size: 13px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
  overflow: hidden;
}

/* ── App Shell ──────────────────────────────────────── */

#app { display: flex; flex-direction: column; height: 100vh; width: 100vw; }

#titlebar { height: 38px; width: 100%; flex-shrink: 0; background: var(--sidebar-bg); }

#app-body { display: flex; flex: 1; min-height: 0; }

/* ── Sidebar ────────────────────────────────────────── */

#sidebar {
  width: 216px;
  min-width: 216px;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 0;
}

.sidebar-header {
  padding: 18px 16px 14px;
  border-bottom: 1px solid var(--border);
  margin-bottom: 8px;
}

.app-identity { display: flex; align-items: center; gap: 8px; margin-bottom: 14px; }

.app-icon { color: var(--text); opacity: 0.9; flex-shrink: 0; }

.app-name { font-size: 13px; font-weight: 600; color: var(--text); letter-spacing: -0.01em; }

#status-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-pill); /* overlay lozenge radius */
}

#status-dot {
  width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0;
  background: var(--green);
  box-shadow: 0 0 6px rgba(111, 224, 140, 0.45);
  transition: background-color var(--ease-out), box-shadow var(--ease-out);
}
#status-dot.recording  { background: var(--red);    box-shadow: 0 0 6px rgba(242, 184, 184, 0.5); animation: statusPulse 1s ease-in-out infinite; }
#status-dot.transcribing { background: var(--yellow); box-shadow: 0 0 6px rgba(224, 195, 65, 0.45); animation: statusPulse 1.5s ease-in-out infinite; }
@keyframes statusPulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }

#status-text { font-size: 12px; font-weight: 500; color: var(--text-secondary); }

/* ── Navigation ─────────────────────────────────────── */

#sidebar-nav { flex: 1; padding: 6px 8px; display: flex; flex-direction: column; gap: 2px; }

.nav-item {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 32px;
  padding: 0 10px;
  border-radius: var(--radius);
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 13px;
  font-weight: 400;
  cursor: pointer;
  transition: background-color var(--ease-out), color var(--ease-out);
}

.nav-item:hover { color: var(--text); background: var(--surface-hover); }

.nav-item.active { color: var(--text); background: var(--surface-active); font-weight: 500; }

.nav-icon { width: 16px; height: 16px; flex-shrink: 0; opacity: 0.55; transition: opacity var(--ease-out); }
.nav-item:hover .nav-icon, .nav-item.active .nav-icon { opacity: 1; }

/* ── Sidebar Footer ─────────────────────────────────── */

.sidebar-footer { padding: 12px 16px; border-top: 1px solid var(--border); margin-top: auto; }
.version-text { font-size: 11px; color: var(--text-tertiary); }

/* ── Content Area ───────────────────────────────────── */

#content { flex: 1; overflow-y: auto; background: var(--bg); }

.content-section {
  display: none;
  padding: 36px 44px;
  max-width: 640px;
  opacity: 0;
  transform: translateY(-4px);
}
.content-section.active {
  display: block;
  opacity: 1;
  transform: translateY(0);
  transition: opacity var(--ease-out), transform var(--ease-out);
}
.content-section.leaving {
  opacity: 0;
  transform: translateY(-4px);
  transition: opacity var(--ease-in), transform var(--ease-in);
}

.section-header { margin-bottom: 24px; }
.section-title { font-size: 15px; font-weight: 600; color: var(--text); letter-spacing: -0.01em; margin-bottom: 4px; }
.section-desc { font-size: 12.5px; color: var(--text-secondary); }

/* ── Card + Settings Rows ───────────────────────────── */

.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
}
.card + .card { margin-top: 16px; }

.setting-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 24px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}
.setting-row:last-child { border-bottom: none; }

.setting-label { flex-shrink: 0; min-width: 176px; }
.label-text { display: block; font-size: 13px; font-weight: 500; color: var(--text); margin-bottom: 2px; }
.label-hint { display: block; font-size: 12px; color: var(--text-secondary); line-height: 1.4; }

.setting-control { flex: 1; display: flex; justify-content: flex-end; min-width: 0; }

/* ── Form Controls ──────────────────────────────────── */

select, input[type="password"] {
  width: 100%;
  max-width: 260px;
  padding: 7px 12px;
  background: var(--surface-solid);
  color: var(--text);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  font-family: var(--font);
  font-size: 13px;
  outline: none;
  appearance: none;
  -webkit-appearance: none;
  transition: border-color var(--ease-out);
}
select {
  background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' viewBox='0 0 10 6' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%239a9a9a' stroke-width='1.3' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 32px;
  cursor: pointer;
}
select:focus, input[type="password"]:focus {
  border-color: rgba(255, 255, 255, 0.4);
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.08);
}
input[type="password"]::placeholder { color: var(--text-tertiary); }

/* ── Toggle Group (pill, overlay-lozenge radius) ────── */

.toggle-group {
  display: inline-flex;
  gap: 2px;
  background: var(--surface-solid);
  border-radius: var(--radius-pill);
  padding: 3px;
  border: 1px solid var(--border);
}
.toggle-btn {
  padding: 6px 14px;
  border: none;
  border-radius: var(--radius-pill);
  background: transparent;
  color: var(--text-secondary);
  font-family: var(--font);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: background-color var(--ease-out), color var(--ease-out);
}
.toggle-btn:hover { color: var(--text); }
.toggle-btn.active { background: rgba(255, 255, 255, 0.14); color: #fff; }

/* ── Buttons ────────────────────────────────────────── */

.btn-primary {
  padding: 7px 16px;
  background: #fff;
  color: #0a0a0a;
  border: none;
  border-radius: var(--radius-pill);
  font-family: var(--font);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: opacity var(--ease-out);
}
.btn-primary:hover { opacity: 0.85; }
.btn-primary:disabled { opacity: 0.35; cursor: default; }

/* ── Model Row / Progress ───────────────────────────── */

.model-row { display: flex; gap: 8px; width: 100%; max-width: 260px; }
.model-row select { flex: 1; max-width: none; }

#download-progress { padding: 0 18px 14px; border-bottom: 1px solid var(--border); }
#progress-bar { height: 3px; background: var(--surface-solid); border-radius: var(--radius-pill); overflow: hidden; margin-top: 8px; }
#progress-fill { height: 100%; background: #fff; border-radius: var(--radius-pill); width: 0%;
  transition: width 200ms ease-out; }

/* ── Kbd / read-only display ─────────────────────────── */

kbd {
  display: inline-block;
  padding: 5px 12px;
  background: var(--surface-solid);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 12px;
  color: var(--text-secondary);
  letter-spacing: 0.02em;
}

/* ── Honest-gate badge (§5) ──────────────────────────── */

.gate-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 9px 4px 7px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 500;
  cursor: default;
}
.badge-icon { width: 12px; height: 12px; flex-shrink: 0; }

.empty-state {
  padding: 34px 18px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.6;
}

/* ── Utility / Scrollbar / Selection ─────────────────── */

.hidden { display: none !important; }

#content::-webkit-scrollbar { width: 6px; }
#content::-webkit-scrollbar-track { background: transparent; }
#content::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 3px; }
#content::-webkit-scrollbar-thumb:hover { background: var(--text-tertiary); }

::selection { background: rgba(255, 255, 255, 0.14); color: var(--text); }
```

---

## 4. `main.ts` wiring plan

**Governing rule (the cached-object gotcha, from `progress.txt`):** `currentSettings` is one mutable object; `saveSettings()` re-reads only `microphone`/`whisperModel`/`groqApiKey` straight off their DOM elements, and trusts every *other* field to already be sitting correctly on `currentSettings` because each setter (`setEngine`, `setRecordingMode`, `setInteractionSounds`, `setOverlayVisible`) writes into `currentSettings.<field>` **before** `saveSettings()` is called. Any new bound control must follow that exact shape — mutate `currentSettings` in its setter, then call `saveSettings()` — or the write silently no-ops and the previous cached value gets re-serialized. This is the #1 way a redesign quietly regresses a toggle.

Existing bindings carry over untouched (same element IDs, same setter functions, same `Settings` interface) — this is a re-skin of the shell, not a rewrite of the data flow:

- `micSelect` → `saveSettings()` (`change`)
- `engineLocal`/`engineCloud` → `setEngine()` → `saveSettings()`
- `modelSelect` → `checkModelStatus()` + `saveSettings()`
- `downloadBtn` → `download_model` invoke, progress via `download-progress` event
- `groqKey` → `saveSettings()` (`change`)
- `modeAuto`/`modeToggle`/`modePtt` → `setRecordingMode()` → `saveSettings()`
- `soundsOn`/`soundsOff` → `setInteractionSounds()` → `saveSettings()`
- `overlayOn`/`overlayOff` → `setOverlayVisible()` → `saveSettings()`
- `recording-state` listener → `statusDot`/`statusText` (sidebar chip, unchanged)

**New markup, same real backend (no new Rust commands):**

- Hotkey moves from Recording section into **General**, same `hotkeyText` element + `formatHotkey()`, still a `<kbd>` — **read-only**, no click handler added (see §5, no capture flow exists).
- Overlay section gets a **Dock position** row reading `overlayX`/`overlayY` off `currentSettings` (already deserialized today, just never rendered) as plain text, e.g. `"Bottom-center · offset (0, -8)"`. Read-only for the same reason as hotkey: no UI-driven reposition command exists, only the drag-to-move behavior already in `overlay.html`/`pill_hittest.rs`, which persists on its own. No control, no promise it does something it can't.

**New markup needing a NEW Rust command (clearly gated, §5):**

- **History** section: needs `list_recordings() -> Vec<HistoryEntry>` and `delete_recording(id: String)`. Neither exists in `main.rs` today. **NEW.**
- **Privacy** section: needs a retention-days setting (`retentionDays: u32` on `Settings`, camelCase-renamed per the existing convention) plus a `clear_all_history()` command. **NEW.**
- General's microphone list already has a real command (`list_microphones`) — no new command needed there, contrary to the story text's "else add list_input_devices" hedge: it already exists under a different name, use it as-is.

Section-switch animation wiring (replaces the current no-op CSS `.active` toggle with a proper leave/enter to get the two-phase 150ms in `.content-section.leaving`):

```ts
navItems.forEach((item) => {
  item.addEventListener("click", () => {
    const target = item.getAttribute("data-section");
    const current = document.querySelector(".content-section.active") as HTMLElement | null;
    const next = document.getElementById(`section-${target}`) as HTMLElement | null;
    if (!next || next === current) return;

    navItems.forEach((n) => n.classList.remove("active"));
    item.classList.add("active");

    if (current) {
      current.classList.add("leaving");
      current.classList.remove("active");
      current.addEventListener(
        "transitionend",
        () => { current.classList.remove("leaving"); current.style.display = "none"; },
        { once: true }
      );
    }
    next.style.display = "block";
    // rAF so the browser commits `display:block` before the class flips —
    // otherwise the opacity/transform transition can't run (no paint to animate from).
    requestAnimationFrame(() => next.classList.add("active"));
  });
});
```

---

## 5. Honest-gating plan

Every control that isn't backed by a real Rust path today gets a `.gate-badge` next to its label — never a disabled-looking-but-silently-clickable control, never a hidden section (the section itself stays reachable so the user isn't confused about where a feature "should" be).

| Control | Gate | Rendered state |
|---|---|---|
| Hotkey (General) | no capture-flow command exists | `<kbd>` shown read-only + `.gate-badge`: "Fixed — no rebind flow yet" |
| Dock position (Overlay) | no reposition command; position is drag-persisted only | plain text row + `.gate-badge`: "Drag the pill to move it" (informational, not an apology) |
| History section | `list_recordings`/`delete_recording` don't exist | whole section renders one `.empty-state` card: "History isn't wired up yet — recordings aren't being saved to a log. Coming in a later story." No fake list, no fake delete buttons. |
| Privacy section | no retention/local-storage commands exist | one `.empty-state` card: "Everything Typr does already stays on this device — nothing is uploaded except audio you explicitly send to Groq Cloud when that engine is selected. Retention controls aren't built yet." (This is a true statement given the current engine toggle, not a placeholder promise.) |
| Groq API key field | real (`groqApiKey` round-trips today) | no badge — it works |
| Download button | real (`download_model` + `download-progress` event) | no badge — it works |

No section is ever fully empty-and-silent: the empty-state card always says *why*, matching the standing "zero dead buttons" rule — an honest badge is not a dead button, a button that does nothing with no explanation is.

---

## 6. Migration diff summary vs. current `index.html`

- **Sidebar nav**: 3 items (General/Engine/Recording) → 6 items (General/Recording/Overlay/Models/History/Privacy). `data-section` values change (`engine` → `models`); icons redrawn per §1 (Engine's old equalizer icon retired, its markup moves under Models).
- **Content restructure**:
  - `section-general` keeps the mic picker, **gains** the hotkey `<kbd>` row (moved out of `section-recording`).
  - `section-recording` keeps mode toggle + sounds toggle, **loses** hotkey (moved to General) and **loses** "Show overlay" (moved to new `section-overlay`).
  - `section-engine` renamed `section-models`, otherwise unchanged (engine toggle, model picker, download button, progress bar, Groq key input all identical markup/IDs).
  - **New** `section-overlay`: "Show overlay" toggle (moved, same `overlayOn`/`overlayOff` IDs) + new read-only dock-position row (§4/§5).
  - **New** `section-history`, `section-privacy`: honest empty-state cards only (§5), no functional controls yet.
- **Settings rows wrap in `.card`** instead of a flat `.settings-list` — same row markup inside, just bounded in a bordered container per §2/§3.
- **CSS**: full palette swap from the Linear-style indigo scaffold (`--accent:#5e6ad2`) to the overlay-derived black/alpha/white system (§3) — every `--accent` usage replaced with white-on-alpha states; every hard `border-radius: var(--radius)` on pills/toggles/buttons becomes `var(--radius-pill)` to match the overlay's lozenge; font stack drops the Google-Fonts IBM Plex import (`<link>` tags removed from `<head>`) in favor of `system-ui` — matching `overlay.html`'s stack exactly and cutting an external network dependency.
- **`main.ts`**: no changes to `Settings` interface, no changes to existing setter functions or invoke calls; only addition is the leave/enter transition handler in §4 (replaces the current instant `classList.toggle("active")` swap) and, if History/Privacy ship in this story, two new elements wired to two new commands flagged **NEW** in §4 — otherwise those two sections ship as static empty-state cards with zero `main.ts` wiring (valid, honest, and keeps this story's Rust surface at zero net-new commands if scope is held tight).
