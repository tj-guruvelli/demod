# UI Constraints for Typr

> Every rule here exists because it was violated in this codebase and caught. The
> incident is named so the rule is arguable rather than dogma. Agents building or
> changing this UI should read this first; it is shorter than the diff it prevents.

## Honesty

**1. Never display a status you did not read.**
The settings header shipped a hardcoded `Ready` and only updated on the next state
event, so opening Settings mid-dictation asserted "Ready" while a transcription was
running. A registered `get_recording_state` command existed with no caller. If a value
cannot be read, say `Unknown` or `Checking…` — never guess the happy case.

**2. "Unknown" is a third state, not a synonym for "no".**
A failed model-status check rendered as "(not downloaded)", which claims something we
never learned. Distinguish true / false / could-not-determine, and show the third
honestly.

**3. A failed action must say why.**
The overlay menu ended in `.catch(() => {})`, so a typo'd action, an unregistered
command, and a real error ("No transcript yet") were indistinguishable: the item just
appeared dead. Surface the reason.

**4. A missing affordance beats a disabled one.**
The History retention row shipped a disabled On/Off toggle implying "coming soon".
Removed; the "Not available" badge and its explanation stayed. Don't render a control
you cannot back.

**5. Local data must render even when a remote call fails.**
Model names, sizes and speeds come from a hardcoded catalog, yet one rejected status
check blanked the whole section because the render came after `await Promise.all`.
Never let an optional enrichment take the mandatory content down with it.

## Accessibility (non-negotiable, and measured)

**6. Every interactive element must be reachable by keyboard.**
The sidebar was built from non-focusable elements: `.focus()` left `activeElement` on
`BODY`, making the entire settings window keyboard-unusable. Use real controls with a
real ARIA pattern (this app uses tablist/tab/tabpanel with roving tabindex).

**7. Selection state must live in ARIA, not only in a CSS class.**
Toggle groups conveyed the chosen option purely through `.active`, invisible to a
screen reader. Use `role="radiogroup"`/`role="radio"` with `aria-checked`, set by the
*same code path* that sets the visual state so the two cannot drift.

**8. Contrast is measured, not judged.**
Three elements failed AA at 2.67–2.80:1 while looking fine. Compute the ratio against
the real composited background. Small text gets no large-text exemption. Fix at the
token level, not per selector. The "Not available" badge matters most: an unreadable
explanation makes the honesty decorative.

**9. Ship `:focus-visible` and `prefers-reduced-motion`.**
Both were entirely absent from a UI built on 150ms transitions. Focus rings must be
visible against the dark skin and inset on rounded shapes so they are not clipped.

## Rendering and process

**10. Changing an element's tag changes which UA styles apply.**
Converting nav items to real `<button>`s (correct for a11y) pulled in the browser's
default background, border, centred text and system font — every inactive item became
a pale pill against the dark skin. Reset UA styles whenever you change a tag.

**11. Never resize or reposition the overlay window to change the pill.**
The window is fixed size and the pill morphs inside the webview. Resizing reintroduces
the flicker the design removed. Widening a hit area means changing hit-test geometry or
`pointer-events`, not window geometry.

**12. A green build is not a verified UI.**
The regression in rule 10 passed the build, passed the ARIA audit, and passed the
contrast audit. Only a screenshot caught it. Look at the pixels before claiming done —
and note that a plain browser cannot run this UI at all (`__TAURI_INTERNALS__` is
absent and the module aborts at load), so behavioral checks need the real app.

**13. Settings controls must write to the cached object AND call `saveSettings()`.**
A control that updates the DOM but not the cached settings object silently stops
persisting. This is the known footgun in `src/main.ts`.

**14. Do not add a Tauri command without a caller, or an invoke without a command.**
Two commands shipped dead. The four `menu_*` commands are bound indirectly through
`data-action`, so grepping for `invoke('menu_quit')` finds nothing — see
[ARCHITECTURE.md](./ARCHITECTURE.md) for the full contract inventory.
