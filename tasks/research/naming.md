# Naming research: replacing "Typr"

Date: 2026-07-20
Product: Windows voice dictation. Global hotkey, speak, transcript pastes into the focused app.
Local whisper.cpp by default, optional Groq cloud engine. Aesthetic: cyberpunk / vaporwave,
midnight purple, neon accents.

Frameworks applied: `brand-perception-psychologist` (perception gap analysis),
`brand-guidelines` (voice and naming discipline), `marketing-psychology` (PLFS scoring on the
naming decision itself). This is not freeform brainstorming; the reasoning below is traceable to
those frameworks.

> **NOT LEGAL ADVICE.** Nothing here is a trademark clearance opinion. This document reports
> observed facts with source URLs. A real trademark search by a qualified attorney is required
> before committing to any name. Several USPTO checks below could not be completed because TESS
> and TSDR are JavaScript applications that resist automated fetching; those gaps are named
> explicitly in the UNVERIFIED section rather than filled with inference.

---

## Headline finding: the naming brief was aimed at the wrong target

The brief optimises for names that *sound like voice*. Availability research says that is the
single most crowded territory in this category, and that crowding is not accidental. Every
founder building a dictation app reasons identically ("quiet speech, sound, tuning"), lands in the
same lexical basin, and the basin is now full.

Concretely, of the 23 names checked, **every single dictionary word in the sound/voice/music
register already has a live product on it**, and in most cases a live product *in dictation
specifically*:

| Name | What is already there | Source |
|---|---|---|
| Murmur | "Murmur - Offline Speech to Text" on Microsoft Store; "Murmur - Voice Dictation for Programmers"; MurMur VT | [MS Store](https://apps.microsoft.com/detail/9n2xfgv12x1q) · [usemurmur.com](https://usemurmur.com/) · [murmurvt.com](https://murmurvt.com/) |
| Sotto | Live macOS local-AI dictation app by indie dev Kitze | [sotto.to](https://sotto.to/) · [kitze.io/projects/sotto](https://www.kitze.io/projects/sotto) |
| Murmr | Live on-device voice-to-text for macOS/iOS with diarization | [murmrvoice.com](https://murmrvoice.com/) |
| Timbre | AI video/podcast editor with AI transcription | [timbrehq.com](https://www.timbrehq.com/) |
| Cadence | Multiple live dictation/transcription apps | [cadencevoice.ai](https://cadencevoice.ai) · [writewithcadence.com](https://writewithcadence.com) |
| Susurrus | Open-source speech-to-text GUI on whisper.cpp, 26 stars | [github.com/CrispStrobe/Susurrus](https://github.com/CrispStrobe/Susurrus) |
| Vocoder | `ricky0123/vocoder`, "source code dictation and voice control" | [github.com/ricky0123/vocoder](https://github.com/ricky0123/vocoder) |
| Attune | attune.ai, live AI voice company | [attune.ai/agentic-voice](https://www.attune.ai/agentic-voice) |
| Modulate | Modulate.ai, live voice AI company (ToxMod) | [modulate.ai](https://www.modulate.ai/) |
| Vocode | YC-backed voice-AI agent framework, 3,779 stars | [github.com/vocodedev/vocode-core](https://github.com/vocodedev/vocode-core) |

The prior research file already recorded the same pattern from the other direction: the market has
converged on Aqua Voice, Willow Voice, superwhisper, MacWhisper, FluidVoice, VoiceInk, VoiceDash,
Spokenly (`tasks/research/wispr-flow.md`, section E).

**Strategic consequence.** A voice-metaphor name puts this product *inside* the cluster it needs to
be distinguishable from. The differentiator, per the same research, is not that it hears you. It is
that **the audio never leaves the machine** (Wispr Flow is cloud-only and states its backend must
decrypt audio to transcribe: [security FAQ](https://docs.wisprflow.ai/articles/3467817258-security-and-compliance-faq)).
The name should carry *locality and signal recovery*, not *quietness*. That reframing is what
opens up the only uncrowded territory found: **signal processing / radio**, which is also, at no
extra cost, the most natively cyberpunk vocabulary available.

---

## A) The three favourites, evaluated

Scored against the distinctiveness ladder used in naming practice: generic < descriptive <
suggestive < arbitrary < fanciful. Names get more ownable as you move right.

### The user's objection, diagnosed precisely

The stated concern was "these are generic words that are used in normal speech." That instinct is
correct, but the mechanism is worth naming, because it changes which fixes actually work:

1. **On-the-nose placement is what weakens them, not dictionary-ness per se.** "Apple" is a
   dictionary word and is extremely ownable, because computers have nothing to do with fruit
   (arbitrary). "Murmur" for a voice app is *suggestive*: it describes the product's own domain.
   Suggestive names sit on the weak end of the ownable range. So the fix is not "avoid real words",
   it is "avoid real words that describe what you do."
2. **The measurable cost is search.** You cannot own the results page for "attune" or "murmur".
   Every support query, every review, every word-of-mouth recommendation leaks to someone else.
3. **The verbing argument runs backwards from what was assumed.** The brief likes that Modulate
   names the action. But **an existing verb can never become your verb.** "Google it" works
   because Google was a coinage with no prior meaning to compete with. "Modulate that" will never
   mean this product, because it already means "adjust your tone of voice", which is a *different
   thing the user might actually do*. Verbability is an argument **for coinage and against**
   Modulate, not the reverse.

### Attune

| Criterion | Assessment |
|---|---|
| Distinctiveness | **Low.** Suggestive, in-register. Three live USPTO ATTUNE registrations observed, one held by Attune Media Labs PBC covering AI speech/NLU software ([uspto.report/TM/90550230](https://uspto.report/TM/90550230)). |
| Memorability | Medium. Pleasant but soft, blends with wellness-app naming. |
| Pronounceability | High, but **spelling is ambiguous** (Attune / Atune / A-tune). A name you cannot spell after hearing it is a word-of-mouth tax. |
| Semantic fit | **Weak.** "Attune" means gradual adjustment into harmony. Dictation is instantaneous. It describes a *relationship*, not a transcription. It would fit a product that learns your vocabulary over time, which is a Wispr feature, not this product's differentiator. |
| Verbable | Poorly. The verb is intransitive with a preposition ("attune *to*"). "Attune it" fights the grammar. |
| Aesthetic fit | **Clashes.** Register is therapy, meditation, HR engagement software. Actively wrong for midnight purple and neon. |

**Verdict: drop.** Weakest of the three on both fit and availability. The aesthetic clash alone
disqualifies it; the live AI-speech trademark makes it moot.

### Murmur

| Criterion | Assessment |
|---|---|
| Distinctiveness | **Low**, and worse than expected: this is the single most contested name found. |
| Memorability | **High.** The reduplicated mm-r-mm-r is phonetically sticky. Genuinely the best-sounding of the three. |
| Pronounceability | High, unambiguous spelling. |
| Semantic fit | **The best of the three, and underrated.** A murmur is quiet speech, which implies you can dictate without projecting, which implies privacy in a shared space. It carries the *private* meaning, not merely the *voice* meaning. Very few names do that. |
| Verbable | Yes. "Just murmur it" is grammatical and sounds good. |
| Aesthetic fit | Good. Dark, low, intimate. Works with the palette. |

**Verdict: drop, on availability alone.** The semantics are the best in the shortlist and it is
still unusable. Confirmed collisions: at least three live dictation products named Murmur
(Microsoft Store, usemurmur.com, murmurvt.com), the Mumble VoIP server's daemon is historically
named Murmur ([Wikipedia](https://en.wikipedia.org/wiki/Mumble_(software))), `Kieirra/murmure` is a
949-star speech-to-text repo, and The Subvocal Company's silent-speech wearable is *also* called
Murmur. The medical "heart murmur" sense adds negative valence on top. This is not a close call.

### Modulate

| Criterion | Assessment |
|---|---|
| Distinctiveness | Low as a word. **But the territory it points at is the strongest one found.** |
| Memorability | Medium. Three syllables, slightly clinical. |
| Pronounceability | High. |
| Semantic fit | **Mixed, and technically inverted.** In signal processing, modulation *imposes* information onto a carrier (encoding). Dictation does the opposite: it *recovers* information from an acoustic signal. The technically correct term for what this product does is **demodulation**. The brief already spotted this and it is exactly right. |
| Verbable | Misleadingly. "Modulate that" already means "adjust your tone", a plausible competing action. |
| Aesthetic fit | **Excellent.** Radio, carrier waves, signal intelligence. Pure cyberpunk register. |

**Verdict: drop the word, keep the territory.** Blocked regardless: Modulate.ai is a live voice-AI
company ([modulate.ai](https://www.modulate.ai/)). But the diagnosis is the useful part. Modulate
was the favourite because it names an action in the right technical world. The right move is to
stay in that world and pick the technically accurate term, which is also the more ownable one.

---

## B) Extensions and coinages

### Modulate family

| Candidate | Read |
|---|---|
| **Demod** | Clipping of "demodulator". **The strongest candidate in this document.** Technically precise: demodulation recovers information from a carrier wave, which is literally what speech-to-text does (voice is the carrier, text is the information). Not a word anyone uses in speech, which solves the stated objection directly. Verbs cleanly *as a new verb*: "demod it." Five letters. Reads as a hacker tool. |
| Modu | Semantically hollow, collides with "module", and both .com and .app are live commercial products. Dead. |
| Modul | Domains merely parked, but MODUL-CONNECT is a live registered USPTO software-class mark (Reg. #7518194). Also just reads as a typo of "module". Weak. |
| Modulus | Registered USPTO mark explicitly covering software development tools and AI (Reg. #4,755,560). Also the wrong domain (mathematics, not signal). Dead. |
| Modem | Conceptually lovely (MOdulator-DEModulator, the thing that bridges analog and digital, deeply retro-cyberpunk). But it is a common noun for a household object. That is the worst of both worlds: generic *and* misdirecting. Dead. |
| Demodu | demodu.com is a live monetised SaaS (AI demo-video generator). Dead. |

### Murmur family

| Candidate | Read |
|---|---|
| **Murmuration** | **The richest identity available, and the only survivor of the murmur family.** Three meanings stack: (1) a murmuring sound; (2) the flowing, wave-like flight of a starling flock; (3) *emergence from many local agents with no central controller*, which is a precise structural metaphor for local-first, on-device computing. Meaning (3) is the reason to take it seriously: it describes the architecture, not just the input method. It is also a low-frequency word, so it satisfies "not used in normal speech" while remaining a real word with real resonance. |
| **Murmurate** | The verb form, and an attested (rare) English word from Latin *murmuratus*, per Wiktionary, not a back-formation. Solves Murmuration's one real weakness (it does not verb). Compromised by a same-category GitHub collision, see availability. |
| Murmr | **Actively recommended against.** Vowel-dropping is the exact dated 2010s convention that makes "Typr" feel stale. Renaming Typr to Murmr repeats the mistake being corrected. Also blocked: murmrvoice.com is a live on-device voice-to-text product. |

### Attune family

Attune, Atune, and Tune are all blocked by live products (attune.ai; atune.io SaaS; tune.com
partner-marketing platform). Nothing in this family survives. Additionally, "tune" points at music
and pitch, not at transcription, so even a clear variant would be semantically off.

### Adjacent territory

| Candidate | Read |
|---|---|
| **Nyquist** | **Worth serious consideration.** The Nyquist-Shannon sampling theorem is the foundational law of digital audio (sample above twice the highest frequency or lose information). For a product whose whole thesis is that it processes audio properly and locally, this is an apt and unusually intelligent reference. Proper-noun names feel ownable (compare Tesla, Fourier). No dictation product found under it. Prior meaning is real but niche: Nyquist is also a Lisp dialect used for Audacity plugins ([Audacity manual](https://manual.audacityteam.org/man/nyquist.html)). |
| **Baseband** | Apt: baseband is the recovered original signal *before* modulation, so it names exactly the output. No dictation competitor found. But "baseband processor" is entrenched telecom-hardware jargon ([Wikipedia](https://en.wikipedia.org/wiki/Baseband_processor)) and GitHub is dense with cellular-baseband security research. Heavy prior association. |
| Sideband | Good word, heavily collided: Sideband Networks, an open-source LXMF mesh messenger, sideband.tech, and .com is confirmed for sale. |
| Sotto | Blocked by a live macOS dictation app. See headline table. |
| Vocode / Vocoder | Vocode is a YC company with a 3,779-star repo. Vocoder is a saturated generic audio term with a dictation-adjacent GitHub project on it. Both dead. |
| Subvocal | The Subvocal Company is live and makes a silent-speech wearable; subvocal.io redirects to subvocal.company. Dead. |
| Timbre, Cadence, Overtone, Phon, Sibil | All collided. Cadence and Timbre have live same-category transcription products; Overtone has a 6,201-star OSS project plus a VC-backed consumer app; Phon is a linguistics tool with a parked .com; Sibil has sibil.sh live and .io taken. |

---

## C) Availability findings

Method: WebSearch for product/company collisions, WebFetch to confirm what live sites actually are,
`nslookup` for DNS resolution, `gh search repos` for GitHub, web search for trademark signals.

**Read the domain column carefully.** DNS resolution proves a domain is *registered and pointed
somewhere*. Non-resolution proves **nothing** and is recorded as UNVERIFIED, not as "available."
No WHOIS lookup was performed for any domain in this pass. That is a real gap.

### Survivors

| Name | Product collision | Domains | GitHub | Trademark signal | Confidence |
|---|---|---|---|---|---|
| **Demod** | SDR radio client for iOS/iPadOS ([rtl-sdr.com](https://www.rtl-sdr.com/demod-an-ios-ipados-client/), demod-sdr.pages.dev); a Python energy-modeling lib on PyPI; a chromatic tuner. **None in voice dictation.** | .com and .app do not resolve (UNVERIFIED, not proof of availability). .io returned NXDOMAIN, suggesting unregistered, still not conclusive without WHOIS. | Small SDR repos only, no major collision | None observed | Medium |
| **Murmuration** | Murmuration Inc., civic-engagement tech ([murmuration.org](https://murmuration.org/)); "Organizer by Murmuration" on Play Store. **No dictation collision.** | .com, .app, .io all resolve, so all registered. Live content not confirmed. | `Hover-Labs/murmuration` (Tezos DAO, 17 stars); `chrdebru/murmuration` (SPARQL). No dictation collision. | USPTO record for "M INSIGHTS" owned by Murmuration, Inc. ([uspto.report/TM/86593602](https://uspto.report/TM/86593602)). That is a *different* wordmark by the same company, not "Murmuration" itself. | Medium |
| **Nyquist** | No dictation product found. Nyquist programming language for Audacity plugins ([Wikipedia](https://en.wikipedia.org/wiki/Nyquist_(programming_language))). | All four TLDs resolve, so registered. No fetch succeeded, occupancy UNVERIFIED. | Moderate DSP/audio repos | Not confirmed either way | Medium on collision, UNVERIFIED on domains |
| **Murmurate** | No company or commercial product owns it. **But `mustafa-zidan/murmurate` is a macOS offline transcription app on whisper.cpp**, which is the same category as this product. | .com does not resolve (UNVERIFIED). .app/.io/.dev resolve into known parking ranges, content unconfirmed. | The same-category repo above | None observed | Medium |
| **Baseband** | No dedicated dictation product found. Term is entrenched telecom jargon. | baseband.io is live ("Launching Soon" holding page, fetched directly). .com resolves, fetch refused. .app/.dev do not resolve (UNVERIFIED). | Dense cellular-baseband security research repos | Not confirmed | Medium-high on jargon baggage |

### Blocked

Reported compactly; each has at least one live product or registered software-class mark.

- **Attune** — attune.ai live AI voice company. Three live USPTO ATTUNE registrations, one covering AI speech/NLU software ([90550230](https://uspto.report/TM/90550230)). All domains for sale.
- **Murmur** — three-plus live dictation products; Mumble/Murmur VoIP confirmed; `Kieirra/murmure` 949 stars; Subvocal Company wearable also named Murmur.
- **Modulate** — [modulate.ai](https://www.modulate.ai/), live voice-AI company.
- **Sotto** — [sotto.to](https://sotto.to/) live macOS local dictation app; multiple GitHub dictation clones including `smirk-dev/sotto`, explicitly a "Wispr Flow alternative for Windows".
- **Murmr** — [murmrvoice.com](https://murmrvoice.com/) live on-device voice-to-text.
- **Vocode** — YC company, `vocodedev/vocode-core` 3,779 stars; .dev redirects to their GitHub.
- **Timbre** — [timbrehq.com](https://www.timbrehq.com/) AI transcription; `taoensso/timbre` Clojure logging lib; .app listed at $9,995.
- **Cadence** — live dictation apps at cadencevoice.ai and writewithcadence.com; Cadence Design Systems holds an active Class 9 mark, serial #99289738, Notice of Allowance 2026-05-05 ([Trademarkia](https://www.trademarkia.com/cadence-99289738)).
- **Overtone** — `overtone/overtone` 6,201 stars; [overto.ne](https://overto.ne/) VC-backed AI dating app.
- **Susurrus** — [CrispStrobe/Susurrus](https://github.com/CrispStrobe/Susurrus), 26-star whisper.cpp speech-to-text GUI.
- **Modu** — modu.com and modu.app both live commercial products.
- **Modulus** — registered USPTO mark covering software development tools and AI (Reg. #4,755,560).
- **Tune** — tune.com live partner-marketing SaaS; .io and .dev taken.
- **Atune** — atune.io live SaaS; two trademark filings observed.
- **Vocoder** — generic term saturation plus `ricky0123/vocoder`, a dictation and voice-control project.
- **Sideband** — multiple live products; .com confirmed for sale via GoDaddy.
- **Demodu** — demodu.com live monetised SaaS.
- **Subvocal** — The Subvocal Company live; subvocal.io 301s to subvocal.company.
- **Modul** — MODUL-CONNECT live registered USPTO software-class mark (Reg. #7518194).
- **Sibil** — sibil.sh live (VPS monitoring); .io taken. Homophone cluster: Sybill.ai, sibyls.ai.
- **Phon** — `phon-ca/phon` linguistics software; .com is a GoDaddy for-sale page.

---

## D) Identity sketches for the survivors

The aesthetic target is a sound-wave or vaporwave form in midnight purple with a neon outline.
Noting explicitly which names pair naturally with a waveform mark, since that was asked.

### 1. Demod

- **Mark:** the demodulation diagram itself. A dense modulated carrier wave entering on the left,
  a clean flat baseline with text exiting on the right. This is a **waveform mark by construction**
  and it *explains the product in one glyph*. Neon cyan wave resolving into magenta text on
  midnight purple. Strongest logo concept in this document.
- **Positioning:** "Speech in, text out, nothing leaves the machine."
- **Tagline direction:** lean on signal recovery and locality. "Recover the signal." /
  "Your voice, demodulated locally." / "The carrier never leaves."
- **Voice:** terse, technical, confident. Matches the Plain Speech default from `brand-guidelines`
  for UI, with room for dry personality in empty states.
- **Verb:** "demod it." A new verb, therefore an ownable one.

### 2. Murmuration

- **Mark:** the best purely *visual* opportunity here. A starling murmuration rendered as a
  particle flock whose silhouette reads as a waveform. Neon particles on midnight purple, denser at
  the peaks. Animates beautifully for the recording overlay, and the existing floating pill could
  literally become a flocking animation while capturing. **Pairs with a waveform mark exceptionally
  well**, and more distinctively than a plain oscilloscope trace.
- **Positioning:** many small local computations, no central server, one emergent result.
- **Tagline direction:** "Thousands of parts. No centre." / "It all happens here."
- **Trade-off:** four syllables and eleven letters. Needs a short form. "Murm" is a natural
  in-product nickname and hotkey label.

### 3. Nyquist

- **Mark:** a sampling-lattice motif. A continuous neon curve with sample points marked on it,
  reconstructing cleanly. Also **inherently a waveform mark**, and it signals engineering
  seriousness rather than consumer-app friendliness.
- **Positioning:** the audio is processed correctly, on your hardware, at full fidelity.
- **Tagline direction:** "Sampled properly. Stored nowhere." / "Above the limit."
- **Trade-off:** reads as insider-technical. That is an asset for a power-user Windows tool and a
  liability if the product ever aims at general consumers.

### 4. Murmurate

- **Mark:** shares the Murmuration flock concept, so the visual work transfers if you switch
  between them later.
- **Positioning:** identical to Murmuration, with a usable verb. "Murmurate it."
- **Trade-off:** the same-category GitHub project is a genuine problem for a name whose entire
  purpose is ownability.

### 5. Baseband

- **Mark:** a modulated band collapsing to a clean flat baseline. Waveform-native.
- **Positioning:** the recovered original message.
- **Trade-off:** telecom-hardware association is heavy enough that developers may assume it is a
  radio or modem project.

---

## Recommendation

**Lead with Demod.** It is the only candidate that is simultaneously (a) technically precise about
what the product actually does, (b) absent from the voice/dictation naming cluster, (c) not a word
used in normal speech, which is the stated requirement, (d) natively cyberpunk, (e) short enough
for a hotkey-driven utility, and (f) verbable *as a new verb*, which is the thing Modulate could
never be.

**Hold Murmuration as the alternative** if the priority shifts from technical precision toward
visual identity and story. It has the better mark and the better metaphor for local-first
architecture; it costs syllables and verbability.

Both need a real trademark search before commitment, and both need WHOIS confirmation on domains.

---

## UNVERIFIED / could not confirm

Listed rather than guessed.

1. **No WHOIS lookup was performed for any domain.** All domain findings are DNS resolution plus
   HTTP reachability. "Does not resolve" is recorded above as UNVERIFIED and must not be read as
   "available." Demod's non-resolving .com/.app and NXDOMAIN .io are the most consequential
   instance: promising, unconfirmed.
2. **USPTO TESS and TSDR were never queried directly.** Both are JavaScript applications that did
   not yield to WebFetch. Every trademark signal above came from secondary aggregators
   (uspto.report, Trademarkia) via web search. No formal class-scoped search was run for any name.
   This is the largest gap in the document.
3. **Demod trademark status entirely unconfirmed.** No mark found, but no authoritative search was
   run. Absence of evidence only.
4. **Murmuration domain content unconfirmed.** .com, .app and .io all resolve, but none was
   successfully fetched, so whether they are live businesses, parked, or redirects is unknown.
5. **Nyquist domain occupancy unconfirmed.** All four TLDs resolve; no fetch succeeded.
6. **Modulate trademark** — multiple USPTO case IDs surfaced but mark text and class were not
   confirmed. Irrelevant to the decision since modulate.ai already blocks it on product grounds.
7. **Sibil .com and Phon .io** could not be resolved or fetched (expired certificate, DNS timeout).
8. **International trademark registers were not checked at all.** EUIPO, UKIPO, WIPO: nothing.
9. **Social handle availability was not checked** for any name (X, GitHub org, Discord, npm,
   crates.io). For a developer tool the npm and GitHub-org namespaces are worth checking before
   commitment.
10. **Pronunciation testing with real users was not done.** "Demod" could plausibly be read
    DEE-mod or de-MOD. Worth a five-person check.
11. Whether `mustafa-zidan/murmurate` is actively maintained or abandoned was not established; only
    that it exists and is in the same category.

---

## Source index

**Availability, primary**
- https://www.modulate.ai/ · https://sotto.to/ · https://www.kitze.io/projects/sotto
- https://murmrvoice.com/ · https://usemurmur.com/ · https://murmurvt.com/
- https://apps.microsoft.com/detail/9n2xfgv12x1q
- https://www.attune.ai/ · https://www.attune.ai/agentic-voice
- https://www.timbrehq.com/ · https://cadencevoice.ai · https://writewithcadence.com
- https://overto.ne/ · https://murmuration.org/ · https://sibil.sh · https://www.phon.ca/
- https://baseband.io · https://subvocal.company · https://demodu.com · https://www.modu.com/

**GitHub**
- https://github.com/vocodedev/vocode-core (3,779) · https://github.com/overtone/overtone (6,201)
- https://github.com/CrispStrobe/Susurrus (26) · https://github.com/ricky0123/vocoder
- https://github.com/markqvist/Sideband · https://github.com/Hover-Labs/murmuration (17)
- https://github.com/mustafa-zidan/murmurate · `Kieirra/murmure` (949)

**Trademark signals (secondary aggregators, not authoritative)**
- https://uspto.report/TM/90550230 (Attune Media Labs, AI speech/NLU)
- https://uspto.report/TM/86593602 (M INSIGHTS, Murmuration Inc.)
- https://www.trademarkia.com/cadence-99289738 (Cadence Design Systems, Class 9, active)
- https://www.trademarkia.com/modul-connect-79347170 (Reg. #7518194)
- https://uspto.report/TM/88572275 (Modulus, Reg. #4,755,560)

**Background**
- https://en.wikipedia.org/wiki/Mumble_(software) · https://en.wikipedia.org/wiki/Baseband_processor
- https://en.wikipedia.org/wiki/Nyquist_(programming_language) · https://manual.audacityteam.org/man/nyquist.html
- `tasks/research/wispr-flow.md` section E (prior naming observations)
- https://docs.wisprflow.ai/articles/3467817258-security-and-compliance-faq (the cloud-only fact the positioning rests on)
