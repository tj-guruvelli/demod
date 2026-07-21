# Dictation-app market research: Wispr Flow (primary), SuperWhisper, and the field

Date: 2026-07-20
Purpose: replace inference with sourced fact. A previous feature set was built by guessing what
"like Wispr Flow" means. Everything below carries a URL. Anything unsourced is in the
UNVERIFIED section at the bottom, not asserted in the body.

## Method and confidence

- Wispr Flow claims are from their own marketing site (`wisprflow.ai`) and their public help
  centre (`docs.wisprflow.ai`), fetched directly.
- SuperWhisper and the secondary apps were researched by delegated agents using the same
  fetch-the-primary-source rule. Where those agents relied on a search engine's summary of a doc
  page rather than a direct fetch, that is flagged inline as `[search-summarised]`.
- Pricing figures move. All prices captured 2026-07-20.
- Demod's current feature set was verified against source at `src-tauri/src/settings.rs` and the
  `#[tauri::command]` surface in `src-tauri/src/main.rs`, not taken on trust from the brief.

---

# 2026-07-21 UPDATE — deep dive for Demod's History / Privacy / Notes tabs

Added this pass to spec the three tabs Demod is about to build. Every claim here was pulled
**2026-07-21 via the Apify `rag-web-browser` actor** against the live Wispr help-centre pages
(HTTP 200 verified, full page markdown read), not a search summary. Two things changed
materially since the 2026-07-20 body below, and I have corrected the stale facts inline further
down as well:

1. **Privacy Mode was split into TWO independent controls** — *Privacy Mode* (training) and
   *Private Cloud Sync* (server-side storage). The old single-toggle model in section D is legacy.
2. **The Context-Awareness accessibility-vs-OCR default split is now CONFIRMED** from a real page
   (it was UNVERIFIED #2 before): accessibility-text **on** by default, Screen OCR **off**/opt-in.

## TAB 1 — HISTORY (Wispr calls it "Home")

**(a) What Wispr does** — verified from
https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android

- The desktop **Home** page is the landing page and doubles as history. Left column: a welcome
  header showing **total word count + your dictation shortcut**, then the **full transcript
  history grouped by day** — literal buckets **Today**, **Yesterday**, then by date.
- **Search** sits at the top of the page and filters the whole history.
- **Per-entry actions on hover (desktop):** **Copy**, **Flag**, and **Take action**. (That's the
  entire desktop hover set — it is deliberately small.)
- **Right column is a stats card:** **streak**, **average WPM**, **total words**.
- **Mobile is where the richer per-item action set lives** (useful because Demod is a desktop app
  but these are the affordances users now expect):
  - iOS: **tap** a transcript to copy; **tap a failed one to retry**; **swipe left → Flag / Retry
    / Copy**; **long-press → Copy, Retry, Report, Delete**.
  - Android: each transcript row offers **Copy, Report, Delete, Retry**.
- **Stats gating (iOS):** stats stay hidden until you have dictated **500 words total**, then show
  **Words Spoken, Days Used, Words Per Minute**. (A nice "earn your dashboard" pattern.)
- History is also reachable **without opening the Hub**: the Flow Bar right-click menu has a
  **Transcript history** item and a **Paste last transcript** item.
- Retry semantics (from the errors page, section A7/errors below): a failed transcript keeps its
  **saved audio**, and retry **reprocesses without creating a duplicate history entry**.

**(b) Sources:** app-nav page above; errors page
https://docs.wisprflow.ai/articles/4984532368-fix-taking-longer-than-usual-and-transcription-errors

**(c) Concrete build for Demod's History tab (to read as "very similar to Wispr"):**
- Persisted, searchable transcript list **grouped by day** with `Today` / `Yesterday` / date
  headers. Search box pinned at top, filters title+content.
- Per-row hover actions, minimum viable set = **Copy**, **Re-insert/Paste** (Demod's promoted
  `menu_paste_last`), **Delete**; add **Retry** on rows whose transcription failed, and **Report/
  Flag** if/when Demod grows a feedback path. Keep the row itself click-to-copy like iOS.
- A small **stats strip**: total words, a streak, and average WPM. These are all derivable from
  data Demod already produces (word count per transcript + timestamps); WPM needs recording
  duration, which Demod has at capture time. Consider Wispr's 500-word unlock as a tasteful touch,
  or just show zeros — either reads fine.
- Each row needs: timestamp, the text, source app (if Demod adds context-awareness later),
  engine used (local/Groq) as a subtle badge, and a "failed → saved audio → retry" state.
- This tab is the keystone: it is also Demod's **paste-failure safety net** (a lost paste is
  recoverable because the transcript is in history) and the host for **retry**.

## TAB 2 — PRIVACY (Wispr calls it "Data & Privacy")

**(a) What Wispr does — the CURRENT two-control model** — verified from
https://docs.wisprflow.ai/articles/4709791908-understanding-privacy-mode-and-cloud-sync
and the security FAQ https://docs.wisprflow.ai/articles/3467817258-security-and-compliance-faq

Wispr now governs dictation data with **two independent toggles** in Settings → Data & Privacy:

| Control | Governs | ON | OFF |
|---|---|---|---|
| **Privacy Mode** | Whether your audio/transcripts/edits are used to **train / improve AI models** | Never used for training, by Wispr or any third party | "Share Data" — may be used to improve models (default for trial/standard) |
| **Private Cloud Sync** | Whether transcription + notes data is **stored on Wispr's servers** | Stored server-side; enables cross-device Scratchpad sync, meeting sharing, Notetaker, todos sync | Not stored server-side; those sync features stop. Dictation still works; raw transcripts may still be uploaded transiently to power core dictation |

The four combinations (verbatim truth table from the page):

| Privacy Mode | Cloud Sync | Meaning |
|---|---|---|
| OFF | ON | May train on data + stored server-side. All features. |
| ON | ON | No training, but stored server-side so sync features still work. |
| ON | OFF | **Zero Data Retention** — identical to the old single Privacy Mode. No training, no server storage. Sync features off. |
| OFF | OFF | No server storage of sync-gated data, but data may still be used for training while transmitted in real time. |

Other verified privacy mechanics that matter for a Demod parity build:

- **"Zero Data Retention (ZDR)"** is Wispr's shorthand for *Privacy Mode ON + Cloud Sync OFF*.
  Enterprises can enforce it org-wide (locks both toggles). A **HIPAA BAA permanently locks**
  Privacy Mode ON + Cloud Sync OFF and is **irreversible**.
- **What always syncs regardless of Cloud Sync:** **Dictionary entries, Snippets, and
  subscription/account settings** — Wispr classifies these as "user-authored productivity assets,"
  not dictation content, and stores them server-side (encrypted, TLS 1.2+, access-controlled)
  even under ZDR.
- **Local data storage is a SEPARATE, third control** (desktop), independent of Cloud Sync:
  **Store data locally (default) / Auto-delete every 24 hours / Never store data locally.** The
  24h and "Never" options also clear locally stored **Polish history and Instruct/command history**
  and hide the voice-profile UI. (This was `[search-summarised]` before; now confirmed.)
- **Context Awareness = two separately-toggled sub-features** (this resolves old UNVERIFIED #2):
  - **Accessibility-text context — DEFAULT ON.** Reads text from the active app's accessibility
    tree to improve AI formatting.
  - **Screen OCR — DEFAULT OFF, opt-in.** Captures a full-display screenshot (the display under
    the mouse cursor) to extract proper nouns. Screenshots only flow when **both** toggles are on;
    if accessibility-text is off, Screen OCR is force-disabled. **"Password field contents are
    never read or included in context data."**
- **Data & Privacy page also holds:** Context Awareness, Local storage, **Default note visibility**,
  **Sync Notes / "Hard refresh notes"** button (disabled when Cloud Sync is off), and HIPAA BAA
  controls (**View signed BAA / Revoke BAA** on desktop).
- **Cloud-only transcription remains true** and remains Demod's edge: "Wispr's backend must decrypt
  audio to perform transcription," so there is no local/on-device mode and no true E2E encryption
  at any tier. Demod's local whisper.cpp is a capability Wispr cannot match.
- **Deletion + export (for a "delete all my data" affordance):** account deletion runs an
  async erasure workflow across account DB, usage/history, stored audio, stored content, retrying
  until done (minutes to ~1h). GDPR export bundles account info, subscriptions, **notes,
  dictionaries, usage stats**, and full dictation history *when Cloud Sync is on*.

**(b) Sources:** the two URLs above.

**(c) Concrete build for Demod's Privacy tab.** Demod is local-first, so most of Wispr's
cloud-storage machinery does not apply — which is exactly the story to tell. Recommended contents:

- A headline that states Demod's actual posture plainly: **"Local-first. Your audio and transcripts
  stay on this machine unless you choose the cloud engine."** This is the honest, differentiated
  version of Wispr's privacy page and it is *true* for the whisper.cpp path.
- **Engine/data-flow disclosure:** show, per engine, where audio goes. Local (whisper.cpp) →
  "never leaves your device." Groq (cloud) → "audio is sent to Groq for transcription" with a link.
  Mirror Wispr's candor about the cloud round-trip rather than hiding it.
- **A local-storage control mirroring Wispr's three-way:** *Keep history / Auto-delete after 24h /
  Never store transcripts.* Demod owns the whole store locally so this is straightforward and it is
  the single most recognisable Wispr privacy affordance.
- **"Delete all history / data" button** with confirm — the local analogue of Wispr's erasure
  workflow. Cheap for Demod (drop the local store).
- **"Don't store audio" toggle** — Wispr streams audio and does not persist it locally; Demod
  should let users opt out of retaining the raw WAV after transcription.
- If/when Demod adds context-awareness or an AI-cleanup stage, gate each behind its own **default-
  sensible toggle** here (accessibility-text style reading default-off for a privacy-first app,
  unlike Wispr), and copy the verbatim guardrail **"password field contents are never read."**
- Keep dictionary/snippets explicitly OUT of any "delete my data" wipe, matching Wispr's "these are
  your productivity assets, not dictation content" framing.

## TAB 3 — NOTES / "Scratchpad" (Wispr's transcription scratch surface)

**(a) What Wispr does** — verified from
https://docs.wisprflow.ai/articles/9618237082-using-the-scratchpad-to-save-and-edit-notes
(desktop is the relevant target; iOS differs)

- **Scratchpad (Beta)** is a **floating notes window** with a **multi-tab rich-text editor**, a
  **sidebar**, and **image attachments**. It is a first-class sidebar page ("Scratchpad") *and* an
  optional Flow Bar button.
- **Core behaviors:** notes **auto-save as you type**, **sync across signed-in devices**, and
  **stay available offline** (save locally, sync on reconnect). Notes list is **newest-first**;
  custom sorting is not offered.
- **Voice dictation into a note:** tap the mic → **live waveform** while speaking → **green
  checkmark inserts** the transcribed text at the cursor (or replaces the selection), **X cancels**;
  a spinner shows during transcription and X stays available so you can still discard.
- **Multi-tab:** open several notes side-by-side as tabs; **drag to reorder**, **drag a tab out to
  detach into its own window**.
- **Version history:** a **Clock button** shows past versions of a note; each version is icon-tagged
  by origin — **typed, dictated, or transformed** (AI-rewritten). This is the standout feature —
  provenance per revision.
- **Formatting toolbar:** Bold, Italic, Underline, Inline code, Bullet list, Numbered list,
  **Checklist**, Blockquote, **Toggle**, Link, **Table**.
- **Organizing:** **Pin** (pinned notes stick to top with a pin icon), **Search by title + content
  with matches highlighted**.
- **Images (desktop only):** paste/drop **PNG, JPEG, WebP, GIF**; each image **≤ 5 MB**, **≤ 10
  images per note**; oversized auto-resized or rejected. **No image support on iOS.**
- **Open behavior (Settings → System → Scratchpad):** **Resume last note (default) / Open in new
  tab / Open last active pinned note.**
- **Access:** a bindable **Open Scratchpad** hotkey (Settings → General → Shortcuts; the shortcut
  pill is always shown on the Notes page even when unbound) and an **"Add to Flow Bar"** toggle
  (the Flow Bar button is **hidden by default**).
- **Sync-status UI:** progress indicator during sync; **cloud-checkmark = synced, slashed-cloud =
  offline**; pull-to-sync or a manual circular-arrow refresh; icons only appear when Cloud Sync is
  on. (These are the exact affordances Demod would fake/hide since it is local-only.)

**(b) Sources:** Scratchpad article above; app-nav page (sidebar "Scratchpad: voice notes with
cloud sync"); iOS Notes https://docs.wisprflow.ai/articles/3529886556-using-notes-in-wispr-flow-for-ios

**(c) Concrete build for Demod's Notes/Transcription tab.** The brief calls this Demod's
transcription/"Notes" tab; Wispr's Scratchpad is the exact reference. Recommended contents:

- A **local notes surface** where dictation lands and can be edited before/instead of pasting into
  another app. This is *also* Demod's paste-failure fallback — Wispr literally routes failed pastes
  into the Scratchpad ("your transcript is safe"). Wire Demod's focus-guard miss to "dropped into
  Notes" the same way.
- **Dictate-into-note** with the same interaction grammar: mic → **live waveform** (Demod already
  has the waveform overlay) → **checkmark inserts at cursor / X discards**, insert-at-cursor or
  replace-selection.
- **Auto-save**, **newest-first list**, **pin**, and **search over title + body with highlight**.
  All local, all cheap.
- **Version history tagged by origin (typed vs dictated vs AI-cleaned)** is the single most
  differentiating idea worth stealing — and it's especially apt for Demod because it makes the
  local pipeline's stages visible.
- A **formatting toolbar** if Demod wants rich text (Checklist + Table are the two that users miss
  most), or keep it **markdown + preview** like Wispr's iOS Notes (lighter to build in a WebView).
- **Bindable "Open Notes" hotkey** so users capture a thought without leaving their active app —
  Wispr treats this as a best practice.
- **Skip the cloud/sync UI entirely.** Demod is local; do not fake sync icons. The honest version is
  "saved locally" with no cloud-checkmark theater. This is a place Demod is *simpler and better*.
- Images are a nice-to-have (paste screenshot inline, ≤5 MB, cap per note) but lower priority than
  the dictate → edit → history loop.

---

## A) Wispr Flow feature set

### Positioning, in their own words

> "Flow lets you dictate anywhere you can type — about 4x faster than typing. Speak naturally and
> Flow transcribes in real time, runs AI commands to edit your text, and learns your vocabulary
> over time."
> — https://docs.wisprflow.ai/articles/2772472373-what-is-flow

### A1. Custom dictionary / vocabulary — it does BOTH bias and post-correct

This is the question that matters most for Demod, and the answer is that Wispr runs two distinct
mechanisms under one "Dictionary" UI:

| Mechanism | What it does | Stage |
|---|---|---|
| **Vocabulary words** | "Vocabulary words improve recognition accuracy for uncommon or specialized terms during transcription." | Biases the recogniser |
| **Replacement rules** | "Replacement rules automatically swap one spelling for another", and they "apply as soon as you finish dictating." | Post-transcript find/replace |

Source: https://docs.wisprflow.ai/articles/4052411709-teach-flow-your-words-with-the-dictionary

Other verified dictionary mechanics from the same page:

- Entry length 1–60 characters (59 on iOS).
- "Each word can only have one replacement rule."
- **Starring**: starred words get priority "when Flow encounters conflicting words or has many
  dictionary items" — i.e. an explicit conflict-resolution/priority model, not a flat list.
- **Auto-add**: when you correct a transcription, Flow can add the word automatically.
  "Auto-add to dictionary filters out common everyday words, so Flow only learns distinctive or
  specialized words from your corrections." Toggle lives in Settings → System.
- **Bulk import**: CSV, Mac/Windows only, max 1,000 entries per file, 3MB. One-column file =
  vocabulary; two-column file = misspelling/correction pairs.
  https://docs.wisprflow.ai/articles/8955301725-how-do-i-bulk-import-for-dictionary-and-snippets
- Syncs across Mac, Windows, iOS, Android; changes take effect without restart.
- Dictionary UI has tabs (All / Personal / Shared with team), search, and sort
  (Starred First, Newest, Oldest, Alphabetical).
  https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android

### A2. Text formatting — "Smart Formatting" and "Backtrack"

Source: https://docs.wisprflow.ai/articles/5373093536-how-do-i-use-smart-formatting-and-backtrack

- **Auto-punctuation and capitalisation**: context-aware — it lowercases mid-sentence text "to
  flow naturally" while preserving capitals at sentence starts, and "adds spaces before and after
  your dictation as needed". Note this is smarter than a blanket capitalise-first-letter rule: it
  looks at what is already around the cursor.
- **Filler removal**: removes disfluencies and false starts.
- **Lists**: "use numbers or sequence words ('one… two…' or 'first… second…') to create a list
  automatically."
- **Per-app tone nuance**: in iMessage and WhatsApp, "trailing periods are removed for a more
  casual feel" on short dictations.
- **Spoken punctuation still works**: you can say "period", "exclamation point", "em dash".
- **Backtrack (self-correction)**: say "actually" or restate, and Flow edits mid-dictation. Flow
  "uses your full dictation as context to decide what to change." Documented example: "Let's do
  coffee at 2 actually 3" → "Let's do coffee at 3."
- **Toggleability**: on by default everywhere. Desktop toggle is Settings → Auto Cleanup. iOS has
  a plain on/off. **Android cannot disable it.** Desktop offers "Undo AI edit" to recover the raw
  dictation.
- **Auto Cleanup has four levels** — None / Light / Medium / High — which replaced the older
  binary "Smart Formatting" toggle in desktop v1.5.55 (April 24, 2026).
  https://wisprflow.ai/whats-new

### A3. AI post-processing, tone and style

- **Flow Styles**: "Pick a tone for each kind of app — formal for email, casual for personal
  messages — and Flow formats your dictation to match."
  Four app categories: Personal messages (WhatsApp, Telegram, Signal, Messenger, Discord),
  Work messages (Slack, Teams, Zoom, LinkedIn, Notion, Jira, Google Docs), Email (Gmail, Outlook,
  ProtonMail), Other. Four tones: Formal, Casual, Very Casual (Personal only), Excited (Work/Email/
  Other). Selection is **automatic by app category**; iOS allows a per-session manual override via
  a "style pill" on the keyboard. Desktop defaults all categories to Formal.
  https://docs.wisprflow.ai/articles/2368263928-how-to-setup-flow-styles
- **Writing samples**: you can add 50–500 word writing samples, up to 5, to teach Flow your voice
  (via the Polish page settings). Same source.
- **No custom style rules** — only the four presets. Same source.
- **Transforms (Beta)**: separate AI-rewrite feature with custom prompts, shipped desktop v1.5.751
  (June 10, 2026). Ships with two default bound transforms: "Polish" (`Opt+1` / `Win+Alt+1`) and
  "Prompt Engineer" (`Opt+2` / `Win+Alt+2`), plus a **View Diff** shortcut (`Opt+O` / `Win+Alt+O`)
  so you can see what the AI changed.
  https://wisprflow.ai/whats-new ,
  https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts

### A4. Context awareness — this is a real, documented, default-on feature

Source: https://docs.wisprflow.ai/articles/4678293671-feature-context-awareness

What it reads, locally on device:
- "Flow reads a limited amount of text near your cursor and identifies your active app."
- Textbox contents before / selected / after the cursor.
- On-screen text, and screenshots.
- Variable and file names in coding apps.
- Conversation participant IDs and message roles/content in messaging apps.
- **"Password field contents are never read or included in context data."**

What it improves: transcription accuracy (proper nouns visible on screen), writing style (detects
app category), and smart formatting (context-aware punctuation and capitalisation).

Defaults: **on by default on both Mac and Windows.** Disable at Settings → Data and Privacy.
Enterprise admins can force it off org-wide, and then "the user toggle is forced off and locked."

There are two context sources with different defaults: accessibility-tree text is **default on**,
Screen OCR is **default off** [search-summarised from
https://docs.wisprflow.ai/articles/5020906721-how-does-context-awareness-work — that URL 404s on
direct fetch, so treat the on/off split as lower confidence than the rest of this section].

Developer-specific context features:
- **Variable Recognition** and **IDE File Tagging** are their own Settings pane ("Vibe Coding").
  https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android
- Dedicated docs for Cursor/VS Code: https://docs.wisprflow.ai/articles/6434410694-use-flow-with-cursor-vs-code-and-other-ides
- Dedicated docs for Linux/WSL/terminal: https://docs.wisprflow.ai/articles/6478598909-using-flow-with-linux-wsl-and-terminal-applications
- Dedicated docs for RDP/Citrix/VDI: https://docs.wisprflow.ai/articles/7336156466-use-flow-with-remote-desktops-citrix-rdp-vdi

### A5. Command Mode — a separate hotkey, works on selection

Source: https://docs.wisprflow.ai/articles/4816967992-how-to-use-command-mode

> "Command Mode lets you transform text with your voice — rewrite a paragraph, translate content,
> or ask a question by speaking."

- Trigger: its own shortcut. `Fn+Ctrl` (Mac), `Ctrl+Win+Alt` (Windows).
- **Operates on highlighted text**: "Highlight text in any app", hold the shortcut, speak, release,
  and "Flow replaces the highlighted text with the result."
- Also works with no selection, generating content inline at the cursor.
- Selection cap: "Selections must be under 1,000 words."
- Can even change app settings by voice, e.g. "Add a rule to never use exclamation marks."
- **Paywalled**: "Command Mode requires a paid subscription or active free trial."
- Listed under Settings → Experimental alongside "Press Enter Command" and "Bulk Import".

### A6. Snippets — voice-triggered text expansion

Source: https://docs.wisprflow.ai/articles/5784437944-create-and-use-snippets [search-summarised]

Trigger phrase → expansion text. Say "my address" mid-sentence and Flow types the full address in
place. Desktop UI has Add new, search (`Ctrl+F`), new (`Ctrl+N`). Syncs across devices. Distinct
from Dictionary: dictionary fixes recognition, snippets expand shorthand.

### A7. History, search, re-paste

Source: https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android

- Full transcript history "grouped by day" (Today, Yesterday, then by date), with **search**.
- Hover actions per entry: copy, flag, take action.
- Stats card: streak, average WPM, total words.
- **Inline retry from history** (shipped v1.4.661, March 31, 2026) and **transcript undo**
  (v1.5.55, April 24, 2026). https://wisprflow.ai/whats-new
- **Insights tab** (v1.5.55): usage stats, communication profile, team leaderboard.
- Dedicated hotkeys for **Paste last transcript** (`Shift+Alt+Z` on Windows) and
  **Copy last transcript** (`Shift+Alt+X` on Windows).
- **Local storage policy is a user setting** (desktop, separate from Cloud Sync): Store data
  locally (default) / Auto-delete local data every 24 hours / Never store data locally. Confirmed
  2026-07-21 (no longer search-summarised); the two stricter options also clear local Polish and
  Instruct/command history. See the rewritten two-control privacy model in the 2026-07-21 UPDATE
  (Tab 2) above and the corrected section D below.
  https://docs.wisprflow.ai/articles/4709791908-understanding-privacy-mode-and-cloud-sync

### A8. Languages, code-switching, accents

Source: https://docs.wisprflow.ai/articles/3191899797-use-flow-with-multiple-languages

- 100+ languages, including regional variants (British English, Swiss German, Canadian English
  added July 9 2026, Simplified/Traditional Chinese).
- **Auto-detect exists** but "picks a language at the start of each dictation session" from all
  100+ rather than a narrowed subset.
- **Code-switching is a documented weakness**: "If you switch languages mid-sentence, Flow
  transcribes the entire segment in one language." Chinese-English mixing is called out as
  particularly bad.
- Key guidance worth stealing: "Selecting only the languages you actually use narrows the set Flow
  chooses from, which improves accuracy." — i.e. a **multi-select** language list, not a single
  picker.
- **No on-the-fly translation** in dictation (translation is available through Command Mode).
- A **Language Picker was added to the Flow Bar itself** for quick switching (v1.4.661).
  https://wisprflow.ai/whats-new

### A9. Hotkey / trigger model

Source: https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts

| Action | Mac default | Windows default |
|---|---|---|
| Push to talk | `Fn` | `Ctrl+Win` |
| Hands-free mode | `Fn+Space` | `Ctrl+Win+Space` |
| Command Mode | `Fn+Ctrl` | `Ctrl+Win+Alt` |
| Cancel | `Esc` | `Esc` |
| Paste last transcript | `Cmd+Ctrl+V` | `Shift+Alt+Z` |
| Copy last transcript | `Cmd+Ctrl+C` | `Shift+Alt+X` |
| Polish (Transform) | `Opt+1` | `Win+Alt+1` |
| Prompt Engineer (Transform) | `Opt+2` | `Win+Alt+2` |
| View Diff | `Opt+O` | `Win+Alt+O` |
| Open Scratchpad | none (opt-in) | none (opt-in) |

Gesture model: hold = push-to-talk; **double-press the push-to-talk key to lock into hands-free**;
press hands-free shortcut again or click the ✓ in the Flow Bar to stop.
https://docs.wisprflow.ai/articles/6391241694-use-flow-hands-free

Binding validation rules (directly relevant to Demod's rebinder):
- 3 keys or fewer.
- At least one modifier, or a valid mouse button.
- Cannot mix left/right versions of the same modifier.
- No duplicate bindings across actions.
- "Caps Lock cannot be used in shortcuts."
- Left/right mouse click unsupported; bare letters/numbers unsupported.

**Mouse Flow**: non-primary mouse buttons can be bound for push-to-talk (v1.4.661).
**The cancel hotkey is itself rebindable** (Escape is only the default). https://wisprflow.ai/whats-new

Session limits: desktop dictation sessions cap at **20 minutes with a warning at 19 minutes**;
Android caps at 5 minutes. https://docs.wisprflow.ai/articles/6391241694-use-flow-hands-free

---

## B) UX specifics — the Flow Bar

Source: https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android

**Idle**: a floating bubble at the bottom of the screen. Right-click menu contains: Hide for 1 hour,
Settings, Microphone, Languages, Transcript history, Paste last transcript.

**Recording**: the bubble expands to centre and shows a **live waveform** plus **Cancel and Stop
buttons**. Hands-free stop is a **checkmark (✓)**; **X discards without pasting**.
https://docs.wisprflow.ai/articles/6391241694-use-flow-hands-free

**Positioning**: draggable to left / right / bottom edges with **visual drop zones**; position
persists across launches; **orientation goes vertical when docked to a side edge** (shipped
July 9, 2026). https://wisprflow.ai/whats-new

**Audible feedback**: a "ping" on start. The docs describe the listening confirmation as: "Wait for
the ping, or watch for the white bars moving on the Flow Bar — that confirms Flow is listening."

**No timer on desktop.** A timer is documented only for iOS ("always-on timer", iOS v1.62).
https://wisprflow.ai/whats-new

### Paste target and focus handling — the honest version

Wispr has the same fundamental problem Demod has, and they solve it with *recovery*, not prevention.
Source: https://docs.wisprflow.ai/articles/7971211038-fix-text-not-pasting-after-dictation

- Their first instruction is the same guard Demod uses: "Before dictating, click inside a text
  field — Flow needs an active text field to receive text."
- **Paste-failure is a first-class state**: "If Flow shows a paste-failed notification or opens the
  Scratchpad, your transcript is safe." You then edit in the Scratchpad and copy, or hit
  `Alt+Shift+Z` to paste the last transcript. **The transcript is never lost to a focus failure.**
- Documented real bug classes they hit and fixed: dictation shortcut colliding with `Ctrl+V` so the
  output was just the letter "v"; **old clipboard contents pasting instead of the transcript due to
  a timing issue**; Auto Polish silently dropping text.
- Non-QWERTY layouts break text-field detection and insert incorrectly — they have a whole article
  on it. https://docs.wisprflow.ai/articles/1621472516-flow-fails-to-detect-text-fields-or-inserts-incorrectly-on-non-qwerty-keyboard-layouts

### Error states, verbatim

Source: https://docs.wisprflow.ai/articles/4984532368-fix-taking-longer-than-usual-and-transcription-errors

- "Taking longer than usual" — with body text **"Your audio is saved for retrying."**
- "Flow was processing your last transcript" — when you start a new dictation too soon.
- Microphone states, each distinct: "No microphone detected", "Selected microphone is unavailable",
  "Microphone unavailable", "Microphone error", "Microphone Permission Required" (macOS),
  "Microphone access is disabled" (Windows).
- Storage states with a "Restart Flow" button: "Local database corruption", "Storage Full",
  "There was an error".
- Retry semantics: audio is retained locally for a limited period; retry reprocesses the saved
  audio **without creating a duplicate history entry**. Failure message: "Retry failed. Audio file
  missing".
- Notification categories are individually mutable (v1.4.661). https://wisprflow.ai/whats-new

---

## C) Onboarding and settings surface

Desktop settings panes, as documented:
https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android

| Pane | Contents |
|---|---|
| **General** | Shortcuts, Microphone, Languages |
| **System** | Launch at login, Show Flow Bar, Show in dock (Mac), sound toggles, Mute music while dictating, notification categories, Scratchpad opening behaviour, Auto-add to dictionary, Reset & restart |
| **Vibe Coding** | Variable Recognition, IDE File Tagging |
| **Experimental** | Command Mode, Press Enter Command, Bulk Import |
| **Account** | Name/profile picture, Sign Out, Delete Account |
| **Plans & Billing** | Current plan, upgrade, billing management |
| **Data & Privacy** | Privacy Mode, Context Awareness, Local storage, Default note visibility, Sync Notes, HIPAA BAA controls |

Left sidebar pages (not settings): Home/History, Dictionary, Snippets, Style, Scratchpad/Notes.

Two settings worth noting because they are non-obvious and cheap:
- **"Mute music while dictating"** — mutes system audio during capture.
- **"Auto-select microphone"** (v1.5.751) — ranks preferred devices rather than pinning one, plus
  **clamshell mode support** for docking a laptop. https://wisprflow.ai/whats-new

---

## D) Pricing and processing model

Source: https://wisprflow.ai/pricing

| Plan | Price | Includes |
|---|---|---|
| **Flow Basic** (free) | $0 | 2,000 words/week Mac+Windows; 1,000/week iPhone; unlimited Android (limited time). Custom dictionary and snippets, 100+ languages, Privacy Mode, HIPAA-ready |
| **Flow Pro** | $15/mo monthly, or $12/mo billed annually ($144/yr) | Unlimited words all platforms, **Command Mode**, prioritised support, early access, team features, shared dictionary/snippets |
| **Flow Enterprise** | custom | + SOC 2 Type II, ISO 27001, enforced HIPAA, enforced Privacy Mode, SSO/SAML, admin dashboards, IT admin seats |

14-day Pro trial, no card. Students: 3 months free + 50% off Pro.

### Cloud-only. There is no local option.

This is the single most important strategic fact in this document.

- "Wispr Flow runs entirely in the cloud."
- "Dictation content is processed server-side, and under Privacy Mode no transcripts are stored
  locally."
- "Audio recordings (audio is streamed to the backend and not persisted locally)."
- And, unusually candid: Wispr Flow "does not provide end-to-end encryption in the strict
  cryptographic sense (where the service provider cannot decrypt content)" because "Wispr's backend
  must decrypt audio to perform transcription."

Source: https://docs.wisprflow.ai/articles/3467817258-security-and-compliance-faq

Their privacy story is therefore a *policy* story, not an *architecture* story. **UPDATED
2026-07-21:** the single Privacy Mode toggle described here is now legacy — Wispr split it into two
independent controls, **Privacy Mode** (governs *training*) and **Private Cloud Sync** (governs
*server-side storage*). "Zero Data Retention" is now the specific combination *Privacy Mode ON +
Cloud Sync OFF*. Dictionary + snippets sync server-side even under ZDR. Full current model, truth
table, and the confirmed Context-Awareness accessibility/OCR default split are in the 2026-07-21
UPDATE → Tab 2 near the top of this file.
https://docs.wisprflow.ai/articles/4709791908-understanding-privacy-mode-and-cloud-sync

**Demod's local whisper.cpp engine is a capability Wispr Flow does not have at any price.**

---

## E) Naming and branding observation (observation only, no legal advice)

- "Wispr" is a deliberate misspelling of "whisper" — it evokes both quiet speech (their "whisper
  mode" for dictating in shared spaces, per https://wisprflow.ai/features) and, for a technical
  audience, OpenAI's Whisper model. The company is Wispr AI; "Flow" is the product.
- "Flow" carries the productivity/flow-state meaning, and they lean on it in-product: the overlay
  is the **Flow Bar**, the Android overlay is the **Flow Bubble**, the iOS keyboard is the **Flow
  Keyboard**, tones are **Flow Styles**. The noun is reused as a system prefix.
- The wider market has converged on water/voice metaphors: **Aqua Voice** ("We've typed for 150
  years. It's time to speak.", https://aquavoice.com/), **Willow Voice** ("Speak naturally, Willow
  writes it.", https://willowvoice.com/), **superwhisper**, **MacWhisper**, **FluidVoice**,
  **VoiceInk**, **VoiceDash**, **Spokenly**.
- Practical observation for a rename discussion: "whisper"-derived names are extremely crowded and
  additionally read as "wraps OpenAI Whisper", which undersells anything with its own pipeline.
  "Demod" sits in the *typing* metaphor rather than the *voice* metaphor, which is differentiated in
  this field — every competitor named above is in the voice/water cluster. No trademark opinion is
  offered here; that needs a lawyer, not a research doc.

---

## Feature matrix

Legend: ✅ yes · ❌ no · ⚠️ partial · ❓ unverified

| Feature | Wispr Flow | SuperWhisper | Demod (today) | Source |
|---|---|---|---|---|
| **Windows support** | ✅ | ✅ (with documented gaps) | ✅ (Windows-first) | [WF pricing](https://wisprflow.ai/pricing) · [SW Windows](https://superwhisper.com/docs/get-started/windows) |
| **Local/offline transcription** | ❌ cloud only | ✅ Parakeet V2/V3, Whisper Ultra V3, on-device by default | ✅ whisper.cpp small/medium/large-v3 | [WF security FAQ](https://docs.wisprflow.ai/articles/3467817258-security-and-compliance-faq) · [SW models](https://superwhisper.com/models) |
| **Cloud transcription** | ✅ only mode | ✅ S1-Voice, ElevenLabs Scribe V2, Deepgram Nova | ✅ Groq | same |
| **Vocabulary that biases recognition** | ✅ | ✅ sent alongside audio | ✅ via initial prompt | [WF dictionary](https://docs.wisprflow.ai/articles/4052411709-teach-flow-your-words-with-the-dictionary) · [SW vocabulary](https://superwhisper.com/docs/get-started/interface-vocabulary) |
| **Post-transcript replacement rules** | ✅ | ✅ "Replacements", case-insensitive | ❌ | same |
| **Dictionary priority / starring** | ✅ starred entries win conflicts | ❌ (warns too many words confuses the model) | ❌ | same |
| **Auto-add corrected words** | ✅ with common-word filter | ❓ | ❌ | [WF dictionary](https://docs.wisprflow.ai/articles/4052411709-teach-flow-your-words-with-the-dictionary) |
| **CSV bulk import of vocabulary** | ✅ 1,000 rows / 3MB | ❓ | ❌ | [WF bulk import](https://docs.wisprflow.ai/articles/8955301725-how-do-i-bulk-import-for-dictionary-and-snippets) |
| **Auto-punctuation** | ✅ context-aware | ✅ via AI mode | ⚠️ terminal full stop only | [WF smart formatting](https://docs.wisprflow.ai/articles/5373093536-how-do-i-use-smart-formatting-and-backtrack) |
| **Auto-capitalisation** | ✅ context-aware, lowercases mid-sentence | ✅ via AI mode | ⚠️ first letter of sentence | same |
| **Filler-word removal** | ✅ | ✅ via AI mode | ❌ | same |
| **Reformat into lists** | ✅ from "one… two…" | ✅ via prompt | ❌ | same |
| **Cleanup intensity control** | ✅ None/Light/Medium/High | ✅ via custom prompt | ❌ | [WF changelog](https://wisprflow.ai/whats-new) |
| **Mid-dictation self-correction** | ✅ "Backtrack" | ❓ | ❌ | [WF smart formatting](https://docs.wisprflow.ai/articles/5373093536-how-do-i-use-smart-formatting-and-backtrack) |
| **Undo the AI edit / see raw** | ✅ "Undo AI edit" + View Diff | ✅ History toggles Voice vs AI text | n/a (no AI stage) | same · [SW history](https://superwhisper.com/docs/get-started/interface-history) |
| **Tone/style presets** | ✅ 4 tones × 4 app categories, auto-selected | ✅ per-mode prompts | ❌ | [WF styles](https://docs.wisprflow.ai/articles/2368263928-how-to-setup-flow-styles) · [SW modes](https://superwhisper.com/docs/modes/modes) |
| **Custom AI prompt** | ⚠️ Transforms (Beta) only; no custom style rules | ✅ full custom prompt per mode | ❌ | same |
| **Voice command mode on selected text** | ✅ paid, <1,000 words | ✅ Super Mode reads selection | ❌ | [WF command mode](https://docs.wisprflow.ai/articles/4816967992-how-to-use-command-mode) · [SW modes](https://superwhisper.com/docs/modes/modes) |
| **App-context awareness** | ✅ default on: cursor text, screen text, app id, screenshots | ✅ Super Mode + per-app auto-activation | ❌ | [WF context](https://docs.wisprflow.ai/articles/4678293671-feature-context-awareness) · [SW modes](https://superwhisper.com/docs/modes/modes) |
| **Per-app config/profiles** | ⚠️ style categories only | ✅ explicit per-app/per-site mode rules | ❌ | same |
| **Transcript history + search** | ✅ grouped by day, searchable | ✅ searchable (raw transcript only) | ❌ last transcript only | [WF app nav](https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android) · [SW history](https://superwhisper.com/docs/get-started/interface-history) |
| **Retry a failed transcript** | ✅ audio saved, no dup entry | ✅ reprocess with a different mode | ❌ | [WF errors](https://docs.wisprflow.ai/articles/4984532368-fix-taking-longer-than-usual-and-transcription-errors) |
| **Paste last transcript hotkey** | ✅ `Shift+Alt+Z` (Win) | ✅ Copy button | ⚠️ tray menu item, no hotkey | [WF hotkeys](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts) |
| **Snippets / text expansion by voice** | ✅ | ❓ | ❌ | [WF snippets](https://docs.wisprflow.ai/articles/5784437944-create-and-use-snippets) |
| **Push-to-talk** | ✅ | ✅ | ✅ | [WF hotkeys](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts) · [SW shortcuts](https://superwhisper.com/docs/get-started/settings-shortcuts) |
| **Toggle / hands-free** | ✅ double-press to lock | ✅ | ✅ tap/hold/auto | same |
| **Mouse-button binding** | ✅ non-primary buttons | ✅ click=toggle, hold=PTT | ❌ | same |
| **Rebindable cancel key** | ✅ | ❓ | ❌ | [WF changelog](https://wisprflow.ai/whats-new) |
| **Waveform in overlay** | ✅ | ✅ | ✅ | [WF app nav](https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android) · [SW rec window](https://superwhisper.com/docs/get-started/interface-rec-window) |
| **Cancel/Stop buttons in overlay** | ✅ ✓ and ✗ | ✅ stop on hover | ❌ hotkey only | same |
| **Draggable overlay w/ edge docking** | ✅ + vertical when side-docked | ❓ | ⚠️ draggable, no docking | [WF changelog](https://wisprflow.ai/whats-new) |
| **Recording timer** | ❌ desktop (iOS only) | ❓ | ❌ | [WF changelog](https://wisprflow.ai/whats-new) |
| **Multi-language select + auto-detect** | ✅ 100+, multi-select narrows | ✅ 100+, translation to English | ⚠️ single language or auto | [WF languages](https://docs.wisprflow.ai/articles/3191899797-use-flow-with-multiple-languages) · [SW language](https://superwhisper.com/docs/models/language) |
| **Mid-sentence code-switching** | ❌ documented limitation | ⚠️ model-dependent, not on Nova/Parakeet | ❌ | same |
| **Language picker in the overlay** | ✅ | ❓ | ❌ | [WF changelog](https://wisprflow.ai/whats-new) |
| **Scratchpad / notes** | ✅ Beta, tabs + version history | ❌ | ❌ | [WF changelog](https://wisprflow.ai/whats-new) |
| **Usage stats / insights** | ✅ WPM, streak, word count, leaderboard | ❓ | ❌ | [WF app nav](https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android) |
| **Launch at login** | ✅ | ❓ | ✅ | same |
| **Mic auto-select / ranked preference** | ✅ | ❓ | ⚠️ single pick | [WF changelog](https://wisprflow.ai/whats-new) |
| **Mute music while dictating** | ✅ | ✅ Sound settings pane | ❌ | [WF app nav](https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android) |
| **Start/stop sounds** | ✅ ping | ✅ | ✅ with preview | same |
| **Free tier** | 2,000 words/wk | unlimited local on Apple Silicon | free (BYO Groq key) | [WF pricing](https://wisprflow.ai/pricing) · [SW pricing, 3rd-party](https://spokenly.app/blog/superwhisper-pricing) |
| **Paid** | $15/mo, $12/mo annual | $8.49/mo, $84.99/yr | n/a | same |

### Secondary field (brief)

| App | Platforms | Local? | Dictionary | Pricing | Source |
|---|---|---|---|---|---|
| **Aqua Voice** | Mac, Windows, iOS | ❓ | 5 terms free / 800 Pro; bias-vs-post-correct ❓ | Free 1,000 lifetime words; Pro $8/mo annual | [aquavoice.com](https://aquavoice.com/) · [pricing](https://aquavoice.com/pricing) |
| **Willow Voice** | Mac, Windows (Jan 2026), iOS | ⚠️ offline mode on Mac/iOS [search-summarised] | auto-learning + manual; mechanism ❓ | Free (20 Scribe uses/wk); Pro $15/mo ($12 annual); Business $35/mo | [willowvoice.com/pricing](https://willowvoice.com/pricing) |
| **MacWhisper** | **macOS only** | ✅ local-first, Whisper/Parakeet | Find & Replace only, i.e. post-correct not bias | Free tier; Pro €59–64 **one-time** via Gumroad | [macwhisper.com](https://www.macwhisper.com/) · [spokenly](https://spokenly.app/blog/macwhisper-for-windows) |
| **FluidVoice** (OSS, 8.4k★, GPLv3) | macOS 15+; Windows on waitlist | ✅ Parakeet/Whisper/Apple Speech | ❓ | free/OSS | [github.com/altic-dev/FluidVoice](https://github.com/altic-dev/FluidVoice) |

FluidVoice is the closest open-source reference implementation and is worth reading: it has
Command Mode, a Write Mode for inline rewriting, per-app prompt configuration, a live-preview
overlay, and audio history with budget controls. Note it is **GPLv3 as of 2026-02-23** (Apache 2.0
before that), so it is a design reference, not a code source, for a proprietary Demod.

---

## What Demod is missing

Demod today, verified from `src-tauri/src/settings.rs` and the command surface in `main.rs`:
mic selection, local whisper.cpp / Groq cloud engine toggle, whisper model selection
(small/medium/large-v3), Groq model selection (whisper-large-v3 / turbo), dictionary as a single
initial-prompt string, language selection, hotkey rebinding, tap/hold/auto gesture modes,
start/stop sounds with preview, draggable floating pill with waveform, overlay show/hide,
auto-paste with focus guard, copy-only mode, trailing space, capitalisation, terminal punctuation,
launch at startup, Groq key test, plus a tray menu with "Hide for 1 hour" and "Paste last".

Grouped gaps, each tied to evidence rather than taste:

**1. The dictionary is only half a dictionary.**
Demod passes `dictionary` as an initial prompt, which biases recognition. Wispr runs vocabulary
bias *and* deterministic replacement rules, because bias alone does not reliably fix a name the
model keeps getting wrong. Demod has no replacement rules, no per-entry structure at all (it is one
free-text blob, not a list), no starring/priority, no auto-add-on-correction, no CSV import.
Evidence: https://docs.wisprflow.ai/articles/4052411709-teach-flow-your-words-with-the-dictionary
and SuperWhisper's identical two-mechanism split at
https://superwhisper.com/docs/get-started/interface-vocabulary — two independent products
converged on the same design, which is the strongest signal in this document.

**2. No transcript history.** Every competitor has one. Wispr's is searchable and day-grouped with
retry and undo. Demod keeps only `get_last_transcript()` in memory. This is also a *reliability*
feature, not just convenience: it is what makes a failed paste recoverable.

**3. No failure recovery path.** Wispr's paste-failure handling — "your transcript is safe", open
the Scratchpad, or `Alt+Shift+Z` to re-paste — means a focus miss costs you nothing. Demod's focus
guard prevents the bad paste but the transcript still has nowhere to go.
https://docs.wisprflow.ai/articles/7971211038-fix-text-not-pasting-after-dictation

**4. No retry on transcription failure.** Wispr saves the audio ("Your audio is saved for
retrying") and retries without duplicating history.

**5. Formatting is two booleans where the market has a spectrum.** Demod has `capitalize` and
`ensurePunctuation`. Wispr has context-aware capitalisation (it lowercases mid-sentence to match
surrounding text), filler removal, list detection, and a four-level intensity dial.

**6. No AI post-processing stage at all**, and therefore no tone/style, no cleanup levels, no
custom prompt. Both Wispr and SuperWhisper make this the paid differentiator.

**7. No command mode.** Wispr's is a separate hotkey that operates on the current selection and
replaces it. SuperWhisper's Super Mode does the same by reading selection + clipboard.

**8. No app-context awareness.** Wispr reads the active app id and the text around the cursor by
default on Windows, and uses it for both accuracy and style.

**9. Overlay is display-only.** Wispr's shows the waveform *and* clickable Stop/Cancel controls, a
right-click menu (history, paste last, mic, languages), a language picker, and edge-docking with
re-orientation. Demod's pill shows the waveform and nothing actionable.

**10. Hotkey model is narrower.** No mouse-button binding, no rebindable cancel, no separate
paste-last/copy-last hotkeys, no documented validation rules (Wispr publishes theirs: ≤3 keys, ≥1
modifier, no Caps Lock, no duplicates).

**11. Single-language selection.** Wispr multi-selects to *narrow* the auto-detect candidate set —
"Selecting only the languages you actually use narrows the set Flow chooses from, which improves
accuracy." Demod's single-value `language` field cannot express that.

**12. No snippets.** Voice-triggered text expansion is a Wispr feature on the free tier.

**13. Mic handling is a single pin,** where Wispr ranks preferred devices and handles clamshell/
dock transitions, and surfaces six distinct mic error states. Demod has honest error states already,
but a fixed device pick breaks when a headset disconnects.

---

## Prioritised backlog for Demod, with evidence

Ordered by (user-visible value) ÷ (effort), given Demod is a Windows Tauri app with a local engine.

### P0 — do these first

**1. Transcript history (persisted, searchable, re-pastable).**
Evidence: universal across Wispr, SuperWhisper, MacWhisper, FluidVoice. Wispr groups by day with
search and per-entry copy
(https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android).
Unlocks items 2 and 3 below. Demod already has the transcript in hand; this is mostly storage + UI.

**2. Never lose a transcript: paste-failure recovery.**
Evidence: Wispr treats paste failure as a normal state with a guaranteed-safe transcript and a
`Alt+Shift+Z` re-paste hotkey
(https://docs.wisprflow.ai/articles/7971211038-fix-text-not-pasting-after-dictation).
Concretely for Demod: when the focus guard blocks the paste, keep the text, notify, and offer a
paste-last hotkey. Demod already has `menu_paste_last` in the tray — **promote it to a bindable
hotkey**. This is a small change with outsized reliability payoff.

**3. Replacement rules in the dictionary, and make the dictionary a list.**
Evidence: the strongest signal in this research — Wispr and SuperWhisper independently implement
the same two-mechanism split (bias + deterministic post-correct)
(https://docs.wisprflow.ai/articles/4052411709-teach-flow-your-words-with-the-dictionary ,
https://superwhisper.com/docs/get-started/interface-vocabulary). Demod's single prompt-blob cannot
express "when you hear X, write Y". Post-correct is also cheap and deterministic — it works
identically on the local and cloud engines, which prompt-biasing does not.

**4. Retry on failure, with the audio preserved.**
Evidence: "Your audio is saved for retrying", and retry reprocesses without a duplicate history
entry (https://docs.wisprflow.ai/articles/4984532368-fix-taking-longer-than-usual-and-transcription-errors).
Especially valuable for Demod because the Groq path can fail on network alone.

### P1 — high value, moderate effort

**5. Actionable overlay: Stop and Cancel buttons, plus a right-click menu.**
Evidence: Wispr's recording bubble carries Cancel/Stop, and hands-free stops with a ✓ or discards
with an ✗; the idle bubble right-click menu holds Hide for 1 hour, Settings, Microphone, Languages,
Transcript history, Paste last transcript
(https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android).
Demod already has all of those actions somewhere — this is surfacing, not building.

**6. Optional AI cleanup stage with an intensity dial (None/Light/Medium/High) and an undo.**
Evidence: Wispr replaced a binary Smart Formatting toggle with exactly this four-level control in
v1.5.55 (https://wisprflow.ai/whats-new), keeps it on by default, and provides "Undo AI edit" plus
a View Diff shortcut. Note the design lesson: they shipped a *dial*, not a toggle, and they kept
the raw text recoverable. For Demod this can route through the existing Groq key, so no new
dependency, and it must be off-able for the fully-local users who are Demod's actual differentiator.

**7. Multi-select language list rather than a single language.**
Evidence: "Selecting only the languages you actually use narrows the set Flow chooses from, which
improves accuracy" (https://docs.wisprflow.ai/articles/3191899797-use-flow-with-multiple-languages).
Cheap: whisper.cpp already takes `-l`, and the narrowing logic is a UI + selection concern.

**8. Richer hotkey model: bindable cancel, bindable paste-last/copy-last, mouse buttons.**
Evidence: Wispr's full binding table and Mouse Flow
(https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts ,
https://wisprflow.ai/whats-new); SuperWhisper's click-vs-hold dual mouse behaviour
(https://superwhisper.com/docs/get-started/settings-shortcuts). Also adopt their published
validation rules — Demod's `validate_hotkey` should reject Caps Lock and duplicate bindings.

**9. Filler-word removal and list detection in the formatting layer.**
Evidence: Wispr's Smart Formatting does both, and both are named on the marketing page as
headline behaviours (https://docs.wisprflow.ai/articles/5373093536-how-do-i-use-smart-formatting-and-backtrack ,
https://wisprflow.ai/features). Filler removal is achievable with a rule-based pass and does not
require the AI stage.

### P2 — differentiating, larger

**10. Command mode on the current selection.**
Evidence: Wispr's `Ctrl+Win+Alt`, replaces highlighted text, <1,000 word cap, paid-tier
(https://docs.wisprflow.ai/articles/4816967992-how-to-use-command-mode); SuperWhisper Super Mode
reads active app + selection + clipboard (https://superwhisper.com/docs/modes/modes). Note this
requires reading the selection out of an arbitrary Win32 app, which is the hard part.

**11. App-context awareness (active app id first, cursor text later).**
Evidence: default-on in Wispr on Windows, used for accuracy and style
(https://docs.wisprflow.ai/articles/4678293671-feature-context-awareness). Start with the cheap
half: the foreground process name. That alone enables per-app profiles. Copy their explicit safety
rule verbatim — **"Password field contents are never read or included in context data."**

**12. Per-app profiles (engine/language/style per application).**
Evidence: SuperWhisper's per-app/per-site mode auto-activation is more powerful than Wispr's
four-category style mapping (https://superwhisper.com/docs/modes/modes). Depends on #11.

**13. Snippets (voice-triggered expansion).**
Evidence: https://docs.wisprflow.ai/articles/5784437944-create-and-use-snippets. Lower priority
because it is mostly independent of dictation quality, but it is free-tier at Wispr and shares all
its UI with the dictionary list from #3.

### Cheap wins worth slotting in anywhere

- **"Mute system audio while dictating"** — a named Wispr setting, trivial on Windows.
- **Mic auto-select with ranked preference** instead of a single pinned device, so a disconnected
  headset does not break dictation (Wispr v1.5.751, https://wisprflow.ai/whats-new).
- **Edge docking for the pill with vertical re-orientation** (Wispr, July 9 2026, same source).
  Demod already persists `overlayX`/`overlayY`; docking is an increment.
- **Session cap with a pre-warning.** Wispr caps desktop at 20 min and warns at 19
  (https://docs.wisprflow.ai/articles/6391241694-use-flow-hands-free). Demod has no cap, so a stuck
  hold-gesture can record unbounded.

### Strategic note on positioning

Wispr Flow is **cloud-only and cannot offer local transcription at any price tier**
(https://docs.wisprflow.ai/articles/3467817258-security-and-compliance-faq). Demod's whisper.cpp
path is a genuine capability gap in the market leader, and the same is true versus Aqua Voice and
Willow. The apps that *do* have strong local support (SuperWhisper, MacWhisper, FluidVoice) are
Mac-first — MacWhisper and FluidVoice are Mac-only today, and SuperWhisper's Windows build has
documented missing features (https://superwhisper.com/docs/get-started/windows). **A genuinely
local-first, Windows-first dictation app is an underserved position.** That argues for weighting
the backlog toward features that work with the local engine (replacement rules, formatting,
history, recovery) over ones that require a cloud round-trip.

---

## UNVERIFIED / COULD NOT CONFIRM

Listed rather than guessed.

**Wispr Flow**
1. **Which ASR models/vendors Wispr uses.** The security FAQ explicitly discloses no vendor names
   or model architectures. They mention a "unified speech model" (June 24, 2026 changelog) but do
   not identify it.
2. ~~**The Context Awareness accessibility-vs-OCR default split.**~~ **RESOLVED 2026-07-21.** The
   security & compliance FAQ (fetched HTTP 200 via Apify) states it plainly: **Accessibility-text
   context is DEFAULT ON; Screen OCR is DEFAULT OFF / opt-in**, screenshots flow only when both are
   on, Screen OCR captures the display under the mouse cursor, and turning accessibility-text off
   force-disables Screen OCR. No longer an unknown.
   https://docs.wisprflow.ai/articles/3467817258-security-and-compliance-faq
3. **Whether the desktop Flow Bar shows any elapsed-time indicator.** A timer is documented for iOS
   (v1.62) but I found no desktop equivalent. I did not confirm its *absence* — no doc states there
   is none.
4. **Exact word-count enforcement on the free tier** (rolling week vs calendar week, and what
   happens at the limit mid-dictation).
5. **Auto Cleanup's four levels in detail.** The level names (None/Light/Medium/High) are sourced
   from the changelog; I could not fetch the dedicated "How to Use Auto Cleanup (Beta)" article, so
   what each level actually does is unconfirmed.
6. **Whether dictation has a max dictionary size** beyond the 1,000-row-per-CSV-file import cap.
7. **How Wispr technically performs insertion** (clipboard-paste vs synthetic keystrokes). Their
   troubleshooting article strongly implies clipboard+`Ctrl+V` (the "old clipboard contents pasted"
   bug and the "output was just the letter v" shortcut collision both point that way) but no doc
   states the mechanism outright. Treat as inference, not fact.
8. **Whether the free tier includes Transforms/Scratchpad.** Pricing page lists Command Mode as Pro
   but does not place the newer Beta features on the tier chart.
9. **Any Wispr onboarding/first-run flow detail.** The "Starting your first dictation" article
   404'd on direct fetch; the setup guide was not fetched in full.

**SuperWhisper**
10. **Current lifetime price.** Third-party sources conflict badly: $249.99 vs a claimed 2026
    increase to $849. Not confirmed on an official pricing page. Both are third-party reported.
11. **Factory-default hotkey.** Configurable is documented; the shipped default is not confirmed in
    official docs.
12. **Whether the recording window shows a timer.**
13. **Whether SuperWhisper has auto-add-on-correction, CSV import, or snippets.**
14. **What the Windows build positively supports.** The docs enumerate what is *missing* on Windows
    (FileSync, custom app folder, Hold-Shift-to-auto-send, simulate keypresses, agentic coding
    integrations) and then say only that "many core capabilities are already available" — they do
    not enumerate them.

**Aqua Voice / Willow / MacWhisper**
15. **Whether Aqua Voice's or Willow's dictionary biases the recogniser or post-corrects.** Neither
    confirmed by primary docs; both plausible.
16. **Whether Aqua Voice transcribes locally or in the cloud.** Privacy copy implies local storage;
    the model (Avalon) is proprietary and processing location is unstated.
17. **Aqua Voice month-to-month price** — sources give both $8/mo (annual) and $12/mo (monthly);
    only the annual figure appears consistent.
18. **Willow's transcript history** — not documented on the pages checked.
19. **Willow's offline mode** — reported by a third party (getvoibe), not confirmed on
    willowvoice.com.
20. **MacWhisper's dual pricing** is a real cross-channel discrepancy, not a research error:
    Gumroad sells a €59–64 one-time Pro licence while the App Store "Whisper Transcription" build
    is subscription ($6.99/mo, $29.99/yr, $99.99 lifetime). Which is canonical is unclear.
21. **FluidVoice's custom dictionary handling** — not described in the README.

**Method caveats**
22. The following were read via a search engine's summary of the doc page rather than a direct
    page fetch, so wording may be paraphrased: Wispr snippets, Wispr privacy-mode/cloud-sync local
    storage options, SuperWhisper shortcuts / history / language / settings panes.
23. No claim in this document was verified by running either product. This is documentation
    research, not hands-on testing. Anything about actual behaviour under failure — how the Flow
    Bar really looks mid-recording, whether context awareness measurably helps — would need a real
    install to confirm.

---

## Source index

**Wispr Flow — marketing**
- https://wisprflow.ai/ · https://wisprflow.ai/features · https://wisprflow.ai/pricing · https://wisprflow.ai/whats-new

**Wispr Flow — help centre** (https://docs.wisprflow.ai/)
- What is Flow: `/articles/2772472373-what-is-flow`
- Hotkeys: `/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts`
- Hands-free: `/articles/6391241694-use-flow-hands-free`
- Command Mode: `/articles/4816967992-how-to-use-command-mode`
- App navigation + settings: `/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android`
- Dictionary: `/articles/4052411709-teach-flow-your-words-with-the-dictionary`
- Bulk import: `/articles/8955301725-how-do-i-bulk-import-for-dictionary-and-snippets`
- Snippets: `/articles/5784437944-create-and-use-snippets`
- Smart Formatting + Backtrack: `/articles/5373093536-how-do-i-use-smart-formatting-and-backtrack`
- Styles: `/articles/2368263928-how-to-setup-flow-styles`
- Languages: `/articles/3191899797-use-flow-with-multiple-languages`
- Context Awareness: `/articles/4678293671-feature-context-awareness`
- Security & compliance FAQ: `/articles/3467817258-security-and-compliance-faq`
- Privacy Mode & Cloud Sync: `/articles/4709791908-understanding-privacy-mode-and-cloud-sync`
- Paste troubleshooting: `/articles/7971211038-fix-text-not-pasting-after-dictation`
- Transcription errors: `/articles/4984532368-fix-taking-longer-than-usual-and-transcription-errors`
- Non-QWERTY insertion bug: `/articles/1621472516-flow-fails-to-detect-text-fields-or-inserts-incorrectly-on-non-qwerty-keyboard-layouts`
- IDEs: `/articles/6434410694-use-flow-with-cursor-vs-code-and-other-ides`
- Linux/WSL/terminal: `/articles/6478598909-using-flow-with-linux-wsl-and-terminal-applications`
- Remote desktop: `/articles/7336156466-use-flow-with-remote-desktops-citrix-rdp-vdi`
- Variable Recognition: `/articles/8554805225-variable-recognition`
- Scratchpad / Notes (read 2026-07-21): `/articles/9618237082-using-the-scratchpad-to-save-and-edit-notes`
- iOS Notes (read 2026-07-21): `/articles/3529886556-using-notes-in-wispr-flow-for-ios`
- Pinning notes: `/articles/9879490397-pinning-notes-in-the-notes-hub-and-scratchpad-sidebar`
- Private Cloud Sync & data-sharing prefs: `/articles/9609615338-private-cloud-sync-and-data-sharing-preferences-in-wispr-flow`

**Wispr Trust Center / data controls** (referenced by the help pages, not fetched):
- https://trust.wispr.ai/ · https://wisprflow.ai/data-controls · https://wisprflow.ai/privacy · https://admin.wisprflow.ai

**SuperWhisper**
- https://superwhisper.com/models · https://superwhisper.com/cli
- `/docs/get-started/interface-vocabulary` · `/docs/get-started/interface-history` · `/docs/get-started/interface-rec-window` · `/docs/get-started/settings-shortcuts` · `/docs/get-started/windows` · `/docs/modes/modes` · `/docs/models/language`

**Others**
- https://aquavoice.com/ · https://aquavoice.com/pricing
- https://willowvoice.com/ · https://willowvoice.com/pricing · https://help.willowvoice.com/en/articles/12854184-willow-pricing-plans-overview
- https://www.macwhisper.com/ · https://goodsnooze.gumroad.com/l/macwhisper · https://spokenly.app/blog/macwhisper-for-windows
- https://github.com/altic-dev/FluidVoice
- Third-party pricing aggregators (lower confidence): https://spokenly.app/blog/superwhisper-pricing · https://www.getvoibe.com/resources/superwhisper-pricing/ · https://www.getvoibe.com/resources/aqua-voice-pricing/ · https://www.getvoibe.com/resources/macwhisper-pricing/

**Demod's own code (for the gap analysis)**
- `C:/Users/guruv/tools/demod/src-tauri/src/settings.rs` — the full settings struct
- `C:/Users/guruv/tools/demod/src-tauri/src/main.rs` — the `#[tauri::command]` surface
</content>
