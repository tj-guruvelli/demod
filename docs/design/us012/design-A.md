# US-012 Design A — Faithful Wispr Clone

Angle: mirror Wispr Flow's settings-window look and IA as closely as the research doc
(`docs/RESEARCH-wispr-superwhisper.md`, section 5) allows, while staying inside Demod's
existing dark palette (the same blacks/alphas as `src/overlay.html`) and binding to
exactly the Rust surface that exists today. Vanilla TS + CSS only. Inline SVG icons
only. Every control real or honestly disabled with a reason.

Icon note on provenance: these are **lucide-style** paths (same visual language: square
stroke caps/joins, 1.2px stroke, 16x16 grid, matching Demod's existing `.nav-icon`
convention in `index.html`/`style.css` today) hand-drawn to spec, not byte-extracted
from the lucide npm package (no CDN/package fetch is allowed under the hard constraints).
They are visually equivalent to the named lucide icon and license-free to inline.

---

## 1. Sidebar IA

Six sections, exactly per the US-012 acceptance criteria and description order:

| # | Section | data-section | lucide-style icon name | Binds to |
|---|---------|---------------|------------------------|----------|
| 1 | General | `general` | `sliders-horizontal` | microphone, language (future), account-free notice |
| 2 | Recording | `recording` | `mic` | recordingMode, hotkey (read-only), interactionSounds |
| 3 | Overlay | `overlay` | `app-window` | overlayVisible, reset pill position (NEW, optional) |
| 4 | Models | `models` | `cpu` | engine (local/cloud), whisperModel, groqApiKey, download |
| 5 | History | `history` | `history` | honest empty-state (no backend yet) |
| 6 | Privacy | `privacy` | `shield` | static local-only statement + gated retention control |

This supersedes the current 3-tab scaffold (General/Engine/Recording) by splitting
Engine into "Models" (Wispr's own naming for its model/voice picker surface) and
adding Overlay/History/Privacy as their own tabs — matching Wispr's Settings tab row
(General / System / Audio / Data & Privacy) and SuperWhisper's sidebar (Models /
History / Advanced) collapsed into Demod's six.

### Icon SVGs (16x16 viewBox, `stroke="currentColor"`, `stroke-width="1.2"`, `fill="none"`, matching the existing `.nav-icon` class)

```html
<!-- 1. General — sliders-horizontal -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <path d="M2 4h6M11 4h3M2 8h3M8 8h6M2 12h8M13 12h1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
  <circle cx="8.5" cy="4" r="1.4" stroke="currentColor" stroke-width="1.2"/>
  <circle cx="5.5" cy="8" r="1.4" stroke="currentColor" stroke-width="1.2"/>
  <circle cx="10.5" cy="12" r="1.4" stroke="currentColor" stroke-width="1.2"/>
</svg>

<!-- 2. Recording — mic (same family as the existing recording nav icon, kept) -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <rect x="6" y="2.5" width="4" height="7" rx="2" stroke="currentColor" stroke-width="1.2"/>
  <path d="M4 8.5a4 4 0 008 0M8 12.5v1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
</svg>

<!-- 3. Overlay — app-window (outer window chrome + filled bottom pill strip,
     a literal picture of what this tab controls) -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <rect x="2" y="2.5" width="12" height="11" rx="1.6" stroke="currentColor" stroke-width="1.2"/>
  <path d="M2 5.5h12" stroke="currentColor" stroke-width="1.2"/>
  <rect x="5.5" y="10.2" width="5" height="1.6" rx="0.8" fill="currentColor"/>
</svg>

<!-- 4. Models — cpu -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <rect x="4.5" y="4.5" width="7" height="7" rx="1" stroke="currentColor" stroke-width="1.2"/>
  <rect x="6.5" y="6.5" width="3" height="3" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
  <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3 3l1.4 1.4M13 3l-1.4 1.4M3 13l1.4-1.4M13 13l-1.4-1.4"
        stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
</svg>

<!-- 5. History — clock with a counter-clockwise reload arrow -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <path d="M3.5 4.5A6 6 0 112.2 8.6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
  <path d="M2 2.5v2.4h2.4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M8 5v3.2l2.2 1.3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>

<!-- 6. Privacy — shield -->
<svg class="nav-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
  <path d="M8 1.8l5 1.8v3.8c0 3.3-2.1 5.7-5 6.8-2.9-1.1-5-3.5-5-6.8V3.6l5-1.8z"
        stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
  <path d="M5.8 8.1l1.6 1.6 2.8-3.2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
```

Sidebar footer keeps `v0.1.0`; add a second small line under it, `--text-tertiary`,
11px: `Local-only · no account` — a one-line honesty anchor visible from every section
(mirrors Wispr's persistent account/billing footer, inverted to state what Demod does
NOT have, since it has no account system).

---

## 2. Layout spec (px values)

Unchanged shell geometry from the current scaffold — do not re-litigate what already
works:

- `#titlebar`: 38px tall, full width, drag region, `--sidebar-bg`.
- `#app-body`: flex row, fills remaining height.
- `#sidebar`: **224px** fixed width (was 220px; +4px to give the 6-item nav column
  breathing room now that labels are slightly longer, e.g. "Recording"), `16px 0`
  padding, right border `1px solid var(--border)`.
- `.sidebar-header`: `0 16px 16px`, bottom border, `margin-bottom: 8px`. Unchanged.
- `#status-indicator`: unchanged (6px dot + label chip).
- `#sidebar-nav`: `4px 8px` padding, column, **6** `.nav-item` rows, `1px` gap.
- `.nav-item`: `7px 10px` padding, 13px label, 16px icon, `6px` icon-label gap. Row
  height ends up ~30px — six of them + header + footer fit comfortably inside a
  640px-tall window with room to spare (current window height unspecified in
  `tauri.conf.json`; if it is below ~560px, bump the min window height to 600px so the
  sidebar never scrolls — Wispr's settings window never scrolls its own nav).
- `.sidebar-footer`: unchanged position (`margin-top: auto`), now carries the two-line
  version + "Local-only" stamp described above.
- `#content`: flex:1, scrolls vertically (its own scrollbar, sidebar never scrolls).
- `.content-section`: **28px 36px** padding (was 32px 40px; slightly tighter to match
  Wispr's denser settings cards) — apply uniformly, this is a global content-section
  rule, no per-section override.
- `.section-header`: `margin-bottom: 24px` (was 28px).
- **NEW: `.card`** wraps each logical group of `.setting-row`s (Wispr groups related
  toggles inside a bordered card with a small header, rather than one flat list per
  tab — e.g. Recording tab = one card "Trigger" (mode + hotkey) + one card "Feedback"
  (sounds)). Card: `background: var(--surface)`, `border: 1px solid var(--border)`,
  `border-radius: var(--radius-lg)` (8px), `padding: 4px 20px`, `margin-bottom: 20px`.
  `.setting-row` inside a `.card` loses its own `padding: 16px 0` bottom border look
  only for the *last* row (card border takes over); first/last row rules from the
  existing CSS keep working unmodified since `.card` is just a wrapper.
- **NEW: `.card-header`**: `padding: 14px 0 10px`, 12px, uppercase, `letter-spacing:
  0.04em`, `color: var(--text-tertiary)`, `border-bottom: 1px solid var(--border)`,
  `margin-bottom: 4px` — this is the literal Wispr "small caps section label inside a
  card" pattern (compare Wispr's Settings → System card headers "Appearance",
  "Behavior").
- **NEW: `.switch`** (iOS-style toggle, Wispr's actual control for every on/off pref —
  replaces `.toggle-group` for *binary* settings only; segmented `.toggle-group` stays
  for 3-way choices like Recording Mode and 2-way-but-branching choices like Engine
  where the off-state reveals a different card, matching current behavior). Track
  36x20px, thumb 16x16px, 2px inset. See CSS below.
- Model picker card row height grows slightly (44px) to fit a size chip + status chip
  per model — see section 3 CSS `.model-card`.

---

## 3. Complete CSS (shell + nav + cards + toggles)

This is an **additive diff** against the existing `src/style.css` — append these rules,
and change the three numeric values called out above (`#sidebar` width 220→224,
`.content-section` padding, `.section-header` margin). Nothing in the existing file is
deleted; `.toggle-group`/`.toggle-btn`/`select`/`kbd`/`.btn-primary` all keep their
current definitions verbatim and are reused as-is.

```css
/* ── Diff: sidebar width (was 220px) ─────────────────── */
#sidebar {
  width: 224px;
  min-width: 224px;
}

/* ── Diff: tighter content padding to match Wispr density ─ */
.content-section {
  padding: 28px 36px;
}
.section-header {
  margin-bottom: 24px;
}

/* ── Sidebar footer: second honesty line ─────────────── */
.footer-stamp {
  font-size: 11px;
  color: var(--text-tertiary);
  font-weight: 400;
  margin-top: 2px;
}

/* ── Cards (Wispr's bordered settings groups) ─────────── */
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 4px 20px;
  margin-bottom: 20px;
}

.card:last-child {
  margin-bottom: 0;
}

.card-header {
  padding: 14px 0 10px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-tertiary);
  border-bottom: 1px solid var(--border);
  margin-bottom: 4px;
}

/* A card's setting-rows reuse .setting-row/.setting-label/.setting-control
   verbatim from the existing sheet — only the outer wrapper changes. */

/* ── iOS-style switch (Wispr's real control for binary prefs) ─ */
.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  flex-shrink: 0;
}

.switch input {
  /* Real, focusable, keyboard-operable checkbox — not a div faking a control.
     Visually hidden, not display:none, so Tab focus and Space/Enter still work. */
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  margin: 0;
  opacity: 0;
  cursor: pointer;
  z-index: 1;
}

.switch-track {
  position: absolute;
  inset: 0;
  background: var(--surface-active);
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  transition: background-color var(--transition), border-color var(--transition);
}

.switch-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--text-secondary);
  transition: transform var(--transition), background-color var(--transition);
}

.switch input:checked ~ .switch-track {
  background: var(--accent);
  border-color: var(--accent);
}

.switch input:checked ~ .switch-thumb {
  transform: translateX(16px);
  background: #fff;
}

.switch input:focus-visible ~ .switch-track {
  box-shadow: 0 0 0 2px var(--accent-subtle);
}

.switch input:disabled ~ .switch-track {
  opacity: 0.4;
  cursor: not-allowed;
}

.switch input:disabled {
  cursor: not-allowed;
}

/* ── Honest-gate badge (for anything not backed by Rust yet) ─ */
.gate-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-tertiary);
  background: var(--surface-active);
  border: 1px solid var(--border);
  border-radius: 999px;
  white-space: nowrap;
}

.gate-badge svg {
  width: 11px;
  height: 11px;
  opacity: 0.7;
}

/* ── Empty state (History tab honesty) ────────────────── */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 8px;
  padding: 48px 24px;
  color: var(--text-secondary);
}

.empty-state svg {
  width: 32px;
  height: 32px;
  opacity: 0.35;
  margin-bottom: 4px;
}

.empty-state .empty-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.empty-state .empty-detail {
  font-size: 12px;
  max-width: 340px;
  line-height: 1.5;
}

/* ── Model picker card row ────────────────────────────── */
.model-card-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 0;
  border-bottom: 1px solid var(--border);
}

.model-card-row:last-child {
  border-bottom: none;
}

.model-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.model-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.model-meta {
  font-size: 11px;
  color: var(--text-secondary);
  display: flex;
  gap: 8px;
}

.model-status {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.model-status .gate-badge.downloaded {
  color: var(--green);
  border-color: rgba(62, 207, 110, 0.3);
  background: rgba(62, 207, 110, 0.08);
}
```

---

## 4. `main.ts` wiring plan

### 4.1 Nav: 3 sections → 6 sections

Current `navItems`/`sections` querySelector loop is generic and needs **zero changes**
— it already toggles `.active` off `data-section` matches. Only `index.html` grows
three more `<a class="nav-item" data-section="...">` entries (overlay, models,
history, privacy — `models` replaces `engine` 1:1, keep the same DOM subtree that was
under `#section-engine`, just retitle/re-id it `#section-models`) and three more
`<section id="section-X">` blocks. No new JS needed for nav itself.

### 4.2 Existing settings — bind exactly as today, respecting the cached-object gotcha

Per `progress.txt`: `main.ts` caches the **whole** `Settings` object at `loadSettings()`
and every `saveSettings()` call sends that entire cached object back, mutated
in-place per field before the call. This is the load-bearing gotcha: **any field the
Rust side owns (`overlayX`/`overlayY` today) must be re-asserted server-side in
`save_settings`, because the frontend's cached object is a stale snapshot for
Rust-owned fields.** This design adds no new Rust-owned fields, so nothing new needs
that re-assertion — call it out explicitly in the PR description so the next story
doesn't reintroduce the bug.

All current bindings move as-is, just relabeled into their new section/card:

- **General** card "Input": `micSelect` ← `list_microphones` / `currentSettings.microphone`. Unchanged logic, moved out of the old flat General section into a `.card`.
- **Models** card "Engine": `engineLocal`/`engineCloud` toggle-group, `localSettings`/`cloudSettings` show/hide. Unchanged (`setEngine`, `save_settings`).
- **Models** card "Voice model": `modelSelect`, `download-btn`, `download-progress`/`progress-fill`, `check_model_downloaded`, `download_model`. Unchanged logic — see 4.3 for the honest multi-model list upgrade.
- **Models** card "Cloud": `groqKey` (password input). Unchanged.
- **Recording** card "Trigger": `modeAuto`/`modeToggle`/`modePtt` toggle-group (unchanged `setRecordingMode`), hotkey `<kbd>` display fed by `formatHotkey(currentSettings.hotkey)` (unchanged, US-010 already made this live — do not hardcode it again).
- **Recording** card "Feedback": interactionSounds — **migrate from `.toggle-group` (sounds-on/sounds-off two buttons) to the new `.switch`.** Same `setInteractionSounds(enabled)` function, same `saveSettings()` call; only the DOM control and its one event listener change shape (see 4.4).
- **Overlay** card "Visibility": overlayVisible — same migration to `.switch` as interactionSounds.

### 4.3 Models tab: honest downloaded-vs-available state

Replace the single `<select>` + one `check_model_downloaded` call with a static
client-side catalog (matches SuperWhisper's honest size/speed labeling, research
§6) rendered as `.model-card-row`s, each independently checked:

```ts
const MODEL_CATALOG = [
  { id: "small",  label: "Small",  sizeLabel: "~466 MB", speedLabel: "Fast" },
  { id: "medium", label: "Medium", sizeLabel: "~1.5 GB", speedLabel: "Balanced" },
] as const; // exactly the two sizes settings.rs / download_model already support —
            // do NOT list a model id the Rust side can't download.

async function renderModelCards() {
  const rows = await Promise.all(MODEL_CATALOG.map(async (m) => ({
    ...m,
    downloaded: await invoke<boolean>("check_model_downloaded", { modelSize: m.id }),
  })));
  modelList.innerHTML = rows.map((m) => `
    <div class="model-card-row" data-model="${m.id}">
      <div class="model-info">
        <span class="model-name">${m.label}${m.id === currentSettings.whisperModel ? " (active)" : ""}</span>
        <span class="model-meta"><span>${m.sizeLabel}</span><span>${m.speedLabel}</span></span>
      </div>
      <div class="model-status">
        ${m.downloaded
          ? `<span class="gate-badge downloaded">Downloaded</span><button class="btn-primary" data-use="${m.id}">${m.id === currentSettings.whisperModel ? "Active" : "Use"}</button>`
          : `<button class="btn-primary" data-download="${m.id}">Download</button>`}
      </div>
    </div>`).join("");
}
```

Event delegation on `modelList` (one listener, `click` target `data-download`/`data-use`)
replaces the single `downloadBtn`/`modelSelect` pair. `data-download` reuses the exact
existing `download_model` invoke + `download-progress` event listener (already wired to
the global `download-progress` Tauri event — no change there, just point the progress
bar at the row being downloaded via a `data-active-download` attribute instead of the
global `#download-progress` div, OR keep one shared progress bar under the list since
only one download can run at a time anyway — simpler, keep it shared). `data-use` sets
`currentSettings.whisperModel = id; saveSettings(); renderModelCards()` (to refresh the
"(active)"/"Active" labels) — this is the same `saveSettings()` cached-object path,
nothing new on the Rust side.

**No new Rust command required for this.** `check_model_downloaded` already takes a
`modelSize` param; calling it twice (once per catalog entry) is the entire "upgrade."

### 4.4 Switch component wiring (replaces two-button toggle-group for binary prefs)

```html
<label class="switch">
  <input type="checkbox" id="sounds-switch" />
  <span class="switch-track"></span>
  <span class="switch-thumb"></span>
</label>
```

```ts
const soundsSwitch = document.getElementById("sounds-switch") as HTMLInputElement;
function setInteractionSounds(enabled: boolean) {
  currentSettings.interactionSounds = enabled;
  soundsSwitch.checked = enabled; // keep in sync when set programmatically (loadSettings)
}
soundsSwitch.addEventListener("change", () => {
  setInteractionSounds(soundsSwitch.checked);
  saveSettings();
});
```

Identical pattern for `overlaySwitch` / `overlayVisible`. This removes
`soundsOn`/`soundsOff`/`overlayOn`/`overlayOff` and their four listeners, replaced by
two `<input type="checkbox">` elements and two listeners — net simpler, and it is a
**real native control** (keyboard/screen-reader operable), which the old two-button
toggle-group was not.

### 4.5 Overlay tab: reset-position affordance (marked NEW)

Per research §1 (Wispr persists pill position; SuperWhisper clamps on every show),
Demod's pill position already persists via `pill_hittest.rs` (`overlayX`/`overlayY` in
config, Rust-owned). Users who drag the pill off a monitor that later disappears fall
back to default placement automatically per `restore_position()` — but there's
currently no way to recenter it deliberately without unplugging a monitor. Offer:

```html
<button class="btn-primary" id="reset-overlay-pos">Reset pill position</button>
```

```ts
resetOverlayBtn.addEventListener("click", async () => {
  await invoke("reset_overlay_position"); // NEW — see below
});
```

**NEW Rust command: `reset_overlay_position`** — clears `overlay_x`/`overlay_y` to
`None` in the settings store, persists, and re-runs the same bottom-center placement
`pill_hittest::restore_position()` already computes for "no saved position." This is
additive, low-risk (mirrors existing logic, no new geometry math), and optional for
this story — if the implementing agent is time-boxed, cut this button entirely rather
than ship it disabled-with-no-reason; a missing feature is honest, a fake button is not.

### 4.6 History tab: no wiring — honest empty state only

No Rust history/persistence backend exists anywhere in `progress.txt` or the command
surface used by `main.ts` today. Ship **zero JS** for this tab beyond the nav toggle —
static HTML empty-state (icon + title + detail), see §5 gating plan. Do not invoke any
command that doesn't exist; do not stub a fake list.

### 4.7 Privacy tab: static copy + one gated control

No wiring beyond nav toggle. Static paragraph stating the local-only guarantee (real,
verifiable: no account system exists, confirmed by `progress.txt`/`main.ts` having no
network calls except the user-supplied Groq key path, which is opt-in and disclosed
in the Models tab already). One `.switch` for "Delete recordings automatically" is
**disabled** with a `.gate-badge` reading "Needs History logging" — see §5.

---

## 5. Honest-gating plan

Three surfaces in this design touch functionality the Rust side does not (yet) power.
Each gets the same treatment: visible, explained, never clickable-but-inert.

| Feature | Rust backing today | Gate treatment |
|---|---|---|
| Hotkey "change" affordance | None — no runtime re-register-shortcut command exists; `hotkey` is read-only from config, set only via first-run/manual config edit per `progress.txt` | Keep the existing read-only `<kbd>` display (already live via US-010). Add one `.gate-badge` next to it: "Fixed for this build" with a `title` tooltip "Editing the hotkey isn't wired up yet — edit config.json and restart." **No button rendered at all** — a missing affordance is honest; a disabled button implies "coming soon" which we cannot promise on a timeline. |
| History list | No persistence layer for past transcripts anywhere in the codebase | `.empty-state` block: history icon (opacity 0.35) + "No history yet" + "Demod doesn't save past transcripts in this build — nothing to show here." Zero list markup, zero invoke calls. |
| Privacy → "Delete recordings automatically" | Depends on the same missing History persistence | Render the `.switch` with `disabled` on the `<input>` (grey track per the CSS `:disabled` rule above) plus a `.gate-badge`: "Needs History logging (not implemented)". The control is visibly present (so users know the *shape* of what's planned) but structurally inert — `disabled` attribute, not a fake click handler that no-ops. |
| Overlay → reset position | Real, additive, described in §4.5 | Not gated — ship it, or cut it entirely (binary call, not a middle ground). |

This satisfies the AC "every control visibly does something real or is honestly
disabled with a reason — zero dead buttons": every interactive element in this design
either calls a real, currently-existing `invoke()` (or the one small additive
`reset_overlay_position`), or is a non-interactive text explanation, or is a `disabled`
input with a stated reason. Nothing renders a clickable element with an empty/no-op
handler.

---

## 6. Migration diff summary vs current `index.html`

| Current | New | Why |
|---|---|---|
| 3 nav items: General, Engine, Recording | 6 nav items: General, Recording, Overlay, Models, History, Privacy | Matches US-012 AC "6 sections" and the Wispr/SuperWhisper tab union in research §5 |
| `#section-general` — flat `.settings-list` with 1 row (mic) | `#section-general` — 1 `.card` "Input" with the same mic row | Card wrapper only; binding untouched |
| `#section-engine` — flat list, engine toggle + model select + groq key inline | `#section-models` — 3 `.card`s: "Engine", "Voice model" (new per-model rows, §4.3), "Cloud" | Splits one crowded section into scannable cards, matches Wispr's Model picker density |
| `#section-recording` — flat list: mode, hotkey, sounds (2-btn toggle), overlay (2-btn toggle) | `#section-recording` — 2 `.card`s: "Trigger" (mode + hotkey), "Feedback" (sounds as `.switch`); overlay toggle **moves out** to its own tab | interactionSounds/recordingMode/hotkey are recording-specific; overlay visibility isn't — it belongs with the pill itself |
| (none) | `#section-overlay` — 1 `.card` "Visibility" (overlayVisible as `.switch`) + reset-position button (§4.5, optional) | New tab; consolidates everything about the pill window in one place, mirroring Wispr's "Show Flow Bar" living under System |
| (none) | `#section-history` — `.empty-state` only, no controls | Honest placeholder per §5, not a fabricated feature |
| (none) | `#section-privacy` — static copy + 1 disabled `.switch` | Honest placeholder per §5 |
| `sounds-on`/`sounds-off`/`overlay-on`/`overlay-off` 2-button toggles | `.switch` checkboxes | Real native control, closer to Wispr's actual toggle style, fewer DOM nodes/listeners |
| `#sidebar` 220px | 224px | Room for longer 6-item labels, no wrapping |
| `.content-section` 32px/40px padding | 28px/36px | Denser, closer to Wispr's settings card spacing |
| IBM Plex Sans + Google Fonts `<link>` | **Unchanged** — out of scope for this story, and removing an external font fetch is a separate hardening task, not part of a "faithful Wispr clone" IA/layout pass | Flagging only: this is the one remaining network dependency in `index.html` (`fonts.googleapis.com`/`fonts.gstatic.com` preconnects) and is worth its own follow-up story since the hard constraints for *this* story are about vanilla TS/CSS and inline SVG, not network calls in general |

No existing `invoke()` call, event name, or Settings field is renamed, removed, or
changed in shape. `main.ts`'s `Settings` interface gains zero new fields (the one
optional addition, `reset_overlay_position`, is a fire-and-forget command with no
settings-field round-trip — it just clears two already-existing fields server-side).
