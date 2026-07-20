# US-012 Final Design Synthesis (binding for implementation)

Judged 2026-07-20 by a 3-lens panel (Wispr fidelity + honesty / integration risk / visual
craft) over designs A, B, C. Aggregate: A=24, B=21.5, C=17. Verdict: **A's structure
wearing B's skin**, plus grafts. The implementer reads design-A.md as the base document
and applies the overrides below, in this priority order.

## Base: design-A.md (Faithful Wispr Clone)
Adopt wholesale: 6-section iconized sidebar (General/Recording/Overlay/Models/History/
Privacy), card layout with `.card-header` small-caps sub-group labels, iOS-style switches
replacing the 2-button toggles, honest-gating tables (no dead buttons; a missing
affordance beats a disabled-implying-coming-soon one), main.ts wiring plan (every existing
invoke()/Settings binding moves verbatim; cached-object saveSettings gotcha respected),
migration diff table.

## Override 1 - Skin comes from design-B.md (visual-craft winner, 8.5)
Replace A's borrowed indigo/Linear palette with B's overlay-derived language: the exact
rgba blacks, 0.14-alpha hairline borders, lozenge radii, and 150ms opacity/transform
motion taken from src/overlay.html. The settings window and the pill must read as one
product. Keep A's card STRUCTURE (judge 3's own graft was A's card-headers into B - this
synthesis is that graft applied in reverse).

## Override 2 - Grafts (all three judges' picks)
1. **From C (fidelity judge's pick):** compact health status row pinned in the sidebar
   footer - sidecar/model/hotkey state - visible in every section. Sourced ONLY from the
   existing recording-state event and check_model_downloaded; never fabricated. Replaces
   A's static "Local-only" footer line.
2. **From B (integration judge's pick):** read-only "Dock position" row in the Overlay
   tab rendering the already-deserialized overlayX/overlayY as plain text. No control, no
   invoke, zero regression surface.
3. **From A (craft judge's pick):** already covered by keeping A's `.card-header`
   sub-group labels - do not flatten cards into undifferentiated lists.

## Scope guards (binding)
- NEW Rust commands: at most `reset_overlay_position` (A's optional). If timeboxed, CUT
  it entirely rather than shipping a disabled button. B's proposed history/retention
  commands are OUT of this story.
- Files touched: index.html, src/main.ts, src/style.css only. No tauri.conf.json resize
  (C's 900x640 requirement is dropped), no new Settings fields.
- Gates: npm run build + cargo check; every existing setting must still round-trip
  (manual check of interactionSounds/overlayVisible/recordingMode after the rewrite).
- Honesty: History and Privacy sections ship as A specifies - honest badges/empty states,
  no fabricated retention or history features.

---

## SUPERSEDED 2026-07-20 by the user, after seeing the built app

TJ, looking at the shipped settings window: *"it's not centered the content and
also the colors are lost"* and *"did you use the template from the repo?"*

Two decisions in this document were wrong, and both came from treating in-session
design notes as authority while never reading the `template` remote
(`github.com/albertshiney/typr.git`) that this project was forked from.

1. **Override 1 was wrong.** It said to "replace A's borrowed indigo/Linear palette
   with B's overlay-derived language". That palette was not borrowed - it is the
   template's own product identity: `--accent: #5e6ad2`, `--accent-hover: #6872db`,
   `--accent-subtle: rgba(94,106,210,0.12)`, and saturated status colours
   (`#3ecf6e` / `#e5484d` / `#f5a623`). Removing it is what "the colors are lost"
   means. The accent system is restored.

2. **A's 640px left-aligned `max-width` was wrong for this window.** The template
   uses `padding: 32px 40px` with no cap, so content fills the pane. At 900x600 the
   capped, left-aligned column left a large dead area to the right. Content is now
   centred.

**Rule going forward: the `template` remote is the design authority for anything it
already answers.** Read `git show template/main:src/style.css` before deciding
palette or layout. In-session design docs may extend it; they may not overrule it
without the user saying so.

The accessibility corrections made after this document (AA contrast on small text,
focus-visible, reduced-motion, keyboard-reachable nav) are NOT superseded - the
template fails AA on `--text-tertiary: #3e3e44` and must not be copied there.
