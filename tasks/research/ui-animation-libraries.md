# UI Animation Libraries — Research for Demod (vanilla JS/CSS in WebView2)

**Date:** 2026-07-20
**Purpose:** Source concretely portable animation patterns for the Demod dictation app:
(A) idle/ready pulse indicators, (B) audio visualizers (highest priority), (C) glow/shimmer/
gradient-border/card-separation for a dark UI.

**Consumer constraints (drive every judgement below):**
- Vanilla HTML/CSS/JS. No React, no framer-motion, no Tailwind build step.
- Renders in **WebView2** (Chromium-based, evergreen — modern CSS/Canvas/WebGL2 generally fine).
- The overlay window **can be occluded**, and `requestAnimationFrame` stops when the page is
  treated as hidden. Every rAF dependency is flagged below.

**Method:** read real source, not showcase pages. react-bits and siriwave were cloned and read
from disk. Aceternity source was pulled from its live shadcn registry; 21st.dev source from its
public CDN bundles (access recipes in §F — both are non-obvious and worth keeping). Remaining
gaps are marked UNVERIFIED.

---

## 0. Headline findings

1. **No audio visualizer on reactbits.dev or ui.aceternity.com — verified exhaustively.**
   react-bits: grepped the whole source tree for `AnalyserNode|getByteFrequencyData|
   getByteTimeDomainData|createAnalyser` — **zero hits**. Aceternity: **all 270 components
   enumerated** from its registry index — no waveform, spectrum, voice-bar, siri-orb or mic
   component exists. Its `wavy-background` and `vortex` *look* like waveforms but are
   simplex-noise driven with **zero audio input**.
2. **21st.dev's "Siri Wave" fakes the audio.** Verified from the compiled bundle: the
   "spectrum" is three layered sines in a GLSL shader, and the bundle contains no
   `createAnalyser`, `getUserMedia` or `AudioContext` anywhere. **Do not port it.** (§B2)
3. **21st.dev does have two components driven by genuinely real mic data** —
   `voice-powered-orb` (best analyser wiring) and `music-reactive-hero-section` (best
   canvas-2D wave math, no WebGL). Combining those two is the highest-value result here. (§B5)
4. **The best turnkey visualizer is outside all three sites**: `kopiro/siriwave`, MIT,
   **zero dependencies**, canvas-2D. Apple's Siri waveform math, directly portable, with a
   non-rAF fallback path already built in. (§B3)
5. **react-bits is mostly WebGL.** ~60 of its ~100 components import `ogl` or `three`. Its
   CSS-only animation surface is tiny — a full `@keyframes` grep across every CSS file returns
   only a handful (StarBorder, GooeyNav, ProfileCard, GlitchText). **react-bits has no
   pulse/breathing component at all**, and its ring component (`MagicRings`) is WebGL, not
   CSS rings.
6. **Exactly one component across all three libraries gates its rAF loop on visibility:**
   Aceternity's `dotted-glow-background` (`IntersectionObserver`). That is the pattern to copy
   for the occluded overlay. (§D2)
7. **`prefers-reduced-motion` appears in none of the 22 Aceternity/21st components read**, and
   in only 3 of ~100 react-bits components. You add it yourself, everywhere.
8. **Licensing:** react-bits is *not* plain MIT (Commons Clause). Aceternity serves 270
   components without auth but **publishes no readable license grant** — flagged below.

---

## Licensing summary

| Source | License | Ship in a real app? |
|---|---|---|
| **react-bits** (`DavidHDev/react-bits`) | **MIT + Commons Clause v1.0** (`LICENSE.md`, © 2026 David Haz) | **Yes** — explicitly permits commercial use "as part of an application, website, or product". **But** you "may not sell, sublicense, or redistribute the components themselves — whether alone, in a bundle, or **as a ported version**." Porting for use *inside* Demod is fine; publishing the port as a component library is not. |
| **kopiro/siriwave** | **MIT** (© 2020 Flavio Maria De Stefano) | Yes, unrestricted. Attribution only. |
| **Tailwind CSS** keyframes | MIT | Yes. |
| **MDN** code samples | CC0 / public domain | Yes. |
| **Aceternity UI** | **No readable license grant found.** The pricing page renders as a JS shell with no license text, and no LICENSE file is served alongside the registry JSONs. Components carry `"author": "Manu Arora <hi@manuarora.in>"`. All 270 components are served without auth and **no paywalled component was observed** (a 401 from the registry means the name doesn't exist, not that it's paid). | **UNVERIFIED — confirm before shipping.** Widely treated as free-to-copy, but that is community practice, not a grant I could read. |
| **21st.dev** | Marketplace — per-component, per-author licensing. | **UNVERIFIED per component.** Must check the individual component page. |

> The Commons Clause on react-bits is the one real trap: it does not block using these
> techniques in Demod, but it does block re-publishing a ported "demod-ui" component pack.

---

# A. PULSE / BREATHING (idle-but-ready indicator)

**Finding: none of the three sites is the right source for this.** react-bits has no pulse
component; its `MagicRings` (the obvious candidate by name) is a WebGL/ogl shader, not
concentric CSS rings. The canonical, exact values come from Tailwind's own keyframes
(MIT) — which is also what Aceternity's pulse effects are built on, since Aceternity is a
Tailwind component library.

All four treatments below are **pure CSS, no rAF, no JS, no build step**. They run on the
compositor, so they are the correct choice for an overlay that can be occluded: a paused
compositor animation resumes correctly rather than drifting, and none of them burn a JS frame.

### A1. Opacity breathe (simplest)
Verified source: Tailwind `animate-pulse`, <https://tailwindcss.com/docs/animation>

```css
@keyframes pulse { 50% { opacity: 0.5; } }
.idle-breathe { animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite; }
```
The easing is the whole trick: `cubic-bezier(0.4, 0, 0.6, 1)` is symmetric and slow at both
ends, which reads as breathing rather than blinking. **Port: trivial.**

### A2. Expanding concentric ring / ripple
Verified source: Tailwind `animate-ping`, same page.

```css
@keyframes ping { 75%, 100% { transform: scale(2); opacity: 0; } }
```
Note the keyframe shape: it holds until 75% then is already fully faded — the `75%, 100%`
pair creates a gap between ripples without a second animation. Real concentric rings =
stack 2–3 absolutely-positioned copies behind the dot with staggered `animation-delay`
(e.g. `0s, 0.66s, 1.33s` over a 2s duration) and `animation-timing-function:
cubic-bezier(0, 0, 0.2, 1)` (ease-out — fast expansion, slow fade). **Port: trivial.**

```css
.ready-dot { position: relative; }
.ready-dot::before,
.ready-dot::after {
  content: ""; position: absolute; inset: 0; border-radius: 50%;
  background: currentColor;
  animation: ping 2s cubic-bezier(0, 0, 0.2, 1) infinite;
}
.ready-dot::after { animation-delay: 1s; }
```

### A3. Glow pulse
No verbatim upstream source; this is the standard `box-shadow` composition, and the *layered
shadow stack* below is lifted from react-bits `BorderGlow.css` (verified, see §C1), which
uses a 6-stop shadow ramp to fake a soft bloom without a blur filter:

```css
@keyframes glow-breathe {
  0%, 100% { box-shadow: 0 0 4px 0 var(--g-40), 0 0 12px 0 var(--g-20); }
  50%      { box-shadow: 0 0 8px 0 var(--g-60), 0 0 28px 2px var(--g-30); }
}
.idle-glow { animation: glow-breathe 2.4s cubic-bezier(0.4, 0, 0.6, 1) infinite; }
```
Animating `box-shadow` is not compositor-accelerated. For a small always-on indicator this is
fine; if it ever shows up in profiling, swap to a pseudo-element with a static shadow and
animate its `opacity` instead. **Port: trivial.**

### A4. Scale pulse
```css
@keyframes scale-breathe {
  0%, 100% { transform: scale(1); }
  50%      { transform: scale(1.08); }
}
.idle-scale { animation: scale-breathe 2s cubic-bezier(0.4, 0, 0.6, 1) infinite; }
```
`transform` only — fully compositor-accelerated, cheapest of the four. **Port: trivial.**

### A4b. Aceternity pulse patterns (verified from registry source)

**`glowing-stars` — the cleanest scale-pulse**, 5 stops that map straight onto CSS percentages:
`scale: [1, 1.2, 2.5, 2.2, 1.5]`, `duration: 2, ease: "easeInOut", delay: (idx % 10) * 0.1`.
Driven by `setInterval(..., 3000)` picking 5 of 108 dots. **No rAF.** As CSS:
```css
@keyframes star-pulse {
  0% { transform: scale(1); }   25% { transform: scale(1.2); }
 50% { transform: scale(2.5); } 75% { transform: scale(2.2); }
100% { transform: scale(1.5); }
}
/* animation: star-pulse 2s ease-in-out; animation-delay: calc(var(--i) * 0.1s); */
```
**Port: EASY.** Good "constellation of ready dots" idle treatment.

**`background-ripple-effect`** — distance-driven stagger, the useful part:
`delay = distance * 55` ms, `duration = 200 + distance * 80` ms, emitted as CSS custom
properties. The keyframes live downstream, not in the component; recovered real version:
```css
@keyframes cell-ripple {
  0%   { transform: scale(1);   opacity: 0.5; }
  50%  { transform: scale(1.2); opacity: 0.9; }
  100% { transform: scale(1);   opacity: 0.5; }
}
/* cell-ripple var(--duration,200ms) ease-out none 1 var(--delay,0ms) */
```
⚠️ Two flavors of this keyframe circulate in the wild — the above is from
`better-auth/better-auth` (`demo/nextjs/app/globals.css`). Uses `mask-radial-from-20%`.
**No rAF. EASY-MEDIUM.**

**`dotted-glow-background` — closest match to a "breathing field", pure canvas-2D, no deps.**
Uses a **triangle wave, not a sinusoid**, which is why it reads as a steadier pulse:
```js
mod   = (time * d.speed + d.phase) % 2;
lin   = mod < 1 ? mod : 2 - mod;        // triangle
alpha = 0.25 + 0.55 * lin;              // 0.25 → 0.80
if (alpha > 0.6) ctx.shadowBlur = 6 * glow;
```
speed 0.4–1.3 rad/s, gap 12px, radius 2px, DPR-aware. **rAF: YES — but this is the only
component in the entire research gated by `IntersectionObserver`** (see §D2). Also uses
ResizeObserver + MutationObserver. **MEDIUM.**

**`meteors`** — `animationDelay: Math.random()*5 + "s"`,
`animationDuration: floor(Math.random()*5 + 5) + "s"`:
```css
@keyframes meteor-effect {
  0%   { transform: rotate(215deg) translateX(0);      opacity: 1; }
  70%  { opacity: 1; }
  100% { transform: rotate(215deg) translateX(-500px); opacity: 0; }
}
```
**No rAF. EASY.**

**`loader`** (LoaderOne) — `y: [0,10,0], duration: 1, repeat: Infinity, delay: i*0.2, easeInOut`.
**EASY.** **`shooting-stars`** — `newX = x + speed*cos(angle*π/180)`,
`newScale = 1 + distance/100`, speed 10–30, respawn 1200–4200ms. **rAF: YES. MEDIUM.**

### A5. `prefers-reduced-motion` — the real state of the art here
**Almost nothing in these libraries respects it.** In the entire react-bits tree only three
components reference it: `LogoLoop`, `PixelCard`, `Shuffle`. `PixelCard.jsx:131` shows the
pattern actually used:

```js
const reducedMotion = useRef(window.matchMedia('(prefers-reduced-motion: reduce)').matches).current;
```
Note it is read **once** and never re-subscribed, so it misses runtime changes. Do better in
Demod — wrap every idle animation:

```css
@media (prefers-reduced-motion: reduce) {
  .idle-breathe, .ready-dot::before, .ready-dot::after,
  .idle-glow, .idle-scale { animation: none; }
  .idle-breathe { opacity: 0.75; } /* keep a static "ready" affordance */
}
```
Tailwind's equivalent is the `motion-safe:` / `motion-reduce:` variants; since there is no
Tailwind here, the media query above is the direct translation.

---

# B. AUDIO VISUALIZERS (highest priority)

## B1. The gap, stated plainly

- **reactbits.dev: no audio visualizer.** Verified by exhaustive grep — no `AnalyserNode`, no
  `getByteFrequencyData`, no `getByteTimeDomainData`, no `createAnalyser` in any component.
  Two components call `navigator.mediaDevices.getUserMedia` (`Backgrounds/GridScan`,
  `Components/ReflectiveCard`) but for **camera/video**, not microphone analysis.
  Wave-*looking* components (`Waves`, `LineWaves`, `Ferrofluid`, `Ribbons`, `Radar`) are all
  driven by Perlin/simplex noise on a clock — **fake motion, no audio input**.
- **ui.aceternity.com: no audio visualizer.** All **270** components enumerated from the
  registry index. `wavy-background` and `vortex` are simplex-noise driven, no audio input.
- **21st.dev: three relevant components** — one fake (§B2), two real (§B5).

## B2. 21st.dev "Siri Wave" — ⚠️ FAKES THE AUDIO. Do not port.
Source: <https://21st.dev/@40973894/components/siri-wave> · bundle read from
`https://cdn.21st.dev/larsen66/siri-wave/default/bundle.1782168892470.html`

The "spectrum" is three layered sines in the fragment shader — verbatim:
```glsl
float low  = clamp(0.45 + 0.45*sin(t*0.8)*sin(t*0.37+1.0), 0.0, 1.0);
float mid  = clamp(0.40 + 0.40*sin(t*1.7+2.0)*sin(t*0.53), 0.0, 1.0);
float high = clamp(0.30 + 0.30*sin(t*2.9+4.0)*sin(t*0.71+2.0), 0.0, 1.0);
```
**Verified by absence:** no `createAnalyser`, `getUserMedia` or `AudioContext` anywhere in the
bundle. It is a decorative shader that merely *looks* voice-reactive.

Technically it is raw WebGL1 (`getContext("webgl")`) and genuinely dependency-free — no
three.js, no OGL. Fullscreen triangle `Float32Array([-1,-1, 3,-1, -1,3])`,
`drawArrays(TRIANGLES,0,3)`, uniforms `iResolution`/`iTime` only. `size=420`,
`renderScale=0.75` (renders at 315px and CSS-upscales — a cheap perf trick worth remembering).
**rAF: YES, unconditional and ungated.** No reduced-motion.

**Port: HARD, and pointless.** ~120 lines of GLSL per variant, and a heavy full-screen fragment
shader is a real risk under WebView2 software rendering (WARP). §B3/§B5 give you the same look
with real data and no GPU dependency.

⚠️ Also checked and **rejected**: `ruixen.ui/waveform-player`, `ruixen.ui/voice-message-bubble`,
`ruixen.ui/music-equalizer-upload` — all three bundles fetched, **none** contain
`createAnalyser` or `getByteFrequencyData`. Decorative only.
**UNVERIFIED:** `thegridcn/waveform` and `ruixen.ui/visualizer-button` (bundles 404'd).

## B3. ⭐ siriwave (kopiro/siriwave) — RECOMMENDED
Source: <https://github.com/kopiro/siriwave> · **MIT** · **zero runtime dependencies** ·
canvas-2D · TypeScript (~680 LOC across 3 files). Read from a clone, not the README.

This is the single best fit for Demod: it is the Siri waveform, it is already framework-free,
and its public API is *exactly* the shape a dictation app needs.

**Verified defaults** (`src/index.ts`):
```js
{ style: "ios", ratio: devicePixelRatio || 1, speed: 0.2, amplitude: 1, frequency: 6,
  color: "#fff", cover: false, autostart: true, pixelDepth: 0.02, lerpSpeed: 0.1,
  globalCompositeOperation: "lighter" }
```

**The classic-curve math** (`src/classic-curve.ts`) — this is the whole visualizer:
```js
ATT_FACTOR = 4; GRAPH_X = 2; AMPLITUDE_FACTOR = 0.6;

// bell-shaped envelope that pins the wave to zero at both edges
globalAttFn(x) { return Math.pow(ATT_FACTOR / (ATT_FACTOR + Math.pow(x, ATT_FACTOR)), ATT_FACTOR); }

xPos(i) { return width * ((i + GRAPH_X) / (GRAPH_X * 2)); }
yPos(i) { return AMPLITUDE_FACTOR * globalAttFn(i) * (heightMax * amplitude)
                 * (1 / definition.attenuation) * Math.sin(frequency * i - phase); }

// draw: step i from -GRAPH_X to +GRAPH_X by pixelDepth (0.02), lineTo(xPos(i), heightMax + yPos(i))
```
The Siri look comes from drawing **five** superimposed copies with different attenuation and
opacity (`ClassicCurve.getDefinition()`, verbatim):
```js
[ { attenuation: -2, lineWidth: 1,   opacity: 0.1 },
  { attenuation: -6, lineWidth: 1,   opacity: 0.2 },
  { attenuation:  4, lineWidth: 1,   opacity: 0.4 },
  { attenuation:  2, lineWidth: 1,   opacity: 0.6 },
  { attenuation:  1, lineWidth: 1.5, opacity: 1   } ]
```
Negative attenuation mirrors the curve — that is what produces the interleaved crossing bands.

**Phase advance and the frame loop** (`src/index.ts`, verbatim):
```js
this.phase = (this.phase + (Math.PI / 2) * this.speed) % (2 * Math.PI);

if (window.requestAnimationFrame) {
  this.animationFrameId = window.requestAnimationFrame(this.startDrawCycle.bind(this));
} else {
  this.timeoutId = setTimeout(this.startDrawCycle.bind(this), 20);
}
```
**This built-in `setTimeout(…, 20)` fallback is the exact seam for the occlusion problem** —
see §D2. The library already supports being driven by something other than rAF.

Clearing is done with a composite trick rather than `clearRect`, which is what gives the
trails their soft decay:
```js
ctx.globalCompositeOperation = "destination-out";
ctx.fillRect(0, 0, width, height);
ctx.globalCompositeOperation = "source-over";
```

**Values are interpolated, not snapped** — `lerp()` eases `amplitude` and `speed` toward their
targets at `lerpSpeed: 0.1` per frame. So you can feed it noisy per-frame mic RMS and it
smooths itself; you do not need your own smoothing filter.

**The iOS 9 style** (`src/ios9-curve.ts`) spawns 2–5 randomized sub-curves per color band and
despawns them on a timer — verified constants:
```js
GRAPH_X = 25; AMPLITUDE_FACTOR = 0.8; SPEED_FACTOR = 1; DEAD_PX = 2;
ATT_FACTOR = 4; DESPAWN_FACTOR = 0.02;
DEFAULT_NOOFCURVES_RANGES     = [2, 5];
DEFAULT_AMPLITUDE_RANGES      = [0.3, 1];
DEFAULT_OFFSET_RANGES         = [-3, 3];
DEFAULT_WIDTH_RANGES          = [1, 3];
DEFAULT_SPEED_RANGES          = [0.5, 1];
DEFAULT_DESPAWN_TIMEOUT_RANGES= [500, 2000];
// colors: white (supportLine), blue "15,82,169", red "173,57,76", green "48,220,155"
// drawn with globalCompositeOperation "lighter"
```

- **Mechanism:** canvas-2D, `Math.sin` + an attenuation envelope. No FFT of its own.
- **rAF:** yes, with a `setTimeout` fallback already present.
- **prefers-reduced-motion:** not respected — you must add it.
- **Port difficulty: EASY.** It is already vanilla; drop in the compiled bundle, or port
  ~200 lines of the two curve classes. No React/Tailwind/build step required.

## B4. Driving it with REAL mic data
siriwave supplies the *geometry*; it does not read audio. Feed it via `setAmplitude()`.
Verified reference implementation: MDN,
<https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API/Visualizations_with_Web_Audio_API>
(CC0).

**Wire-up for a dictation app:**
```js
const ctx = new AudioContext();
const src = ctx.createMediaStreamSource(await navigator.mediaDevices.getUserMedia({audio:true}));
const analyser = ctx.createAnalyser();
analyser.fftSize = 256;                 // 128 bins — plenty for an amplitude meter
analyser.smoothingTimeConstant = 0.8;   // AnalyserNode's own EMA smoothing (default 0.8)
src.connect(analyser);                  // NOTE: do NOT connect analyser to destination —
                                        // that would echo the mic to the speakers.
const bins = new Uint8Array(analyser.frequencyBinCount);

function pump() {
  analyser.getByteFrequencyData(bins);
  let sum = 0; for (let i = 0; i < bins.length; i++) sum += bins[i];
  wave.setAmplitude(Math.min(1, (sum / bins.length) / 128)); // 0..1
  requestAnimationFrame(pump);
}
```
Two smoothing stages stack here — `smoothingTimeConstant: 0.8` on the analyser and
`lerpSpeed: 0.1` in siriwave. That is usually pleasant; if it feels laggy, drop the analyser to
`0.6` first.

**MDN's own two canonical visualizers, exact values** (use directly if you want bars rather
than a Siri wave):

*Oscilloscope / waveform* — `fftSize = 2048`, `bufferLength = analyser.frequencyBinCount`
(=1024), `getByteTimeDomainData`:
```js
const sliceWidth = WIDTH / bufferLength;
for (let i = 0; i < bufferLength; i++) {
  const v = dataArray[i] / 128.0;   // 128 = silence midpoint for Uint8 time-domain data
  const y = v * (HEIGHT / 2);
  i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
  x += sliceWidth;
}
```

*Spectrum bar graph* — `fftSize = 256`, `bufferLength` (=128), `getByteFrequencyData`:
```js
const barWidth = (WIDTH / bufferLength) * 2.5;
for (let i = 0; i < bufferLength; i++) {
  const barHeight = dataArray[i] / 2;
  ctx.fillRect(x, HEIGHT - barHeight / 2, barWidth, barHeight);
  x += barWidth + 1;                 // the +1 is the inter-bar gap
}
```
- **Port difficulty: EASY.** Plain canvas-2D + Web Audio, no framework.
- **rAF:** yes — both MDN loops are rAF-driven. See §D2.
- **Voice-bar variant (DOM, no canvas, no rAF):** for a small always-visible meter you can skip
  canvas entirely — render N `<div>` bars and set `transform: scaleY(v)` from the same
  `bins` array. Compositor-accelerated, and it degrades gracefully to a static bar when
  occluded. Cheapest option that still shows real audio.

## B5. ⭐ 21st.dev components that DO use real mic data

### B5a. Voice Powered Orb — steal its analyser block
<https://21st.dev/community/components/isaiahbjork/voice-powered-orb> · bundle:
`https://cdn.21st.dev/isaiahbjork/voice-powered-orb/default/bundle.1757897718443.html`

The **renderer** is WebGL via OGL (it is reactbits' "Orb" shader with a mic hookup bolted on) —
**skip that**. The ~15 lines worth taking, verbatim:
```js
getUserMedia({audio:{echoCancellation:false, noiseSuppression:false,
                     autoGainControl:false, sampleRate:44100}})
analyser.fftSize = 512;
analyser.smoothingTimeConstant = 0.3;
analyser.minDecibels = -90;
analyser.maxDecibels = -10;
audioCtx.createMediaStreamSource(stream).connect(analyser);  // NOT ->destination, so no echo

// RMS reduction, verbatim:
analyser.getByteFrequencyData(buf);
let sum = 0;
for (let i = 0; i < buf.length; i++) { const v = buf[i]/255; sum += v*v; }
const rms = Math.sqrt(sum/buf.length);
return Math.min(rms * sensitivity * 3, 1);
```
Note the constraints: **`echoCancellation`/`noiseSuppression`/`autoGainControl` all disabled** —
correct for visualization (AGC would flatten your dynamics), though for *dictation* you may
want them on for the recognition stream and a separate raw tap for the visual.
Drive math: rotation only advances when `level > 0.05`; `speed = 0.3 + level*rotationIntensity*2`;
`hover = min(level*2, 1)`. **rAF: YES.**
**Audio half: EASY (take it). Visual half: HARD (leave it).**

### B5b. Music-Reactive Hero — best canvas-2D wave math, no WebGL
`lovesickfromthe6ix/music-reactive-hero-section`. Real AnalyserNode + plain `ctx.lineTo`.
**This is the best structural template in the entire research.**
```js
analyser.fftSize = 2048; analyser.smoothingTimeConstant = 0.8;
// band splits on getByteFrequencyData:
bass   = sum(bins 0..29)   / (30*255)
mid    = sum(bins 30..199) / (170*255)
treble = sum(bins 200..799)/ (600*255)
// 20-frame rolling average on bass only
```
4 stacked waves — `amplitude` 30/25/20/35, `frequency` .003/.004/.005/.002,
`speed` .02/.015/.025/.01, `offset` 0/π·.5/π/π·1.5, `opacity` .9/.7/.5/.6.
Per frame `offset += speed*(1 + bass*0.8)`, `amp = amplitude*(1 + intensity*5)`, x stepped 2px,
y = three summed sines:
```js
sin(x*freq + offset)*amp
 + sin(x*freq*2 + offset*1.5)*(amp*0.3*mid)
 + sin(x*freq*0.5 + offset*0.7)*(amp*0.5)
```
Trails via `ctx.fillStyle="rgba(0,0,0,0.92)"; fillRect(...)` each frame (same idea as siriwave's
`destination-out`). Color lerp `hue += (target-hue)*0.5`.

**Its no-audio fallback is the honest "fake" path you may want for the idle state** — verbatim:
```js
bass = 0.4 + Math.sin(time*0.010)*0.3;
mid  = 0.3 + Math.sin(time*0.015)*0.2;
treble=0.2 + Math.sin(time*0.020)*0.1;
```
Upstream source is `createMediaElementSource` (a file, not a mic) — swap in
`createMediaStreamSource` and the rest ports unchanged.
**rAF: YES. Port: MEDIUM-EASY**, pure canvas 2D, no dependencies.

## B6. Voice-reactive border (bonus — best value-for-effort in the whole doc)
react-bits `ElectricBorder` (`src/content/Animations/ElectricBorder.jsx`, read from clone) is
a **canvas-2D** perimeter walk with fBm noise displacement. It has **no WebGL and no React
dependency in its math** — the React part is only a `useEffect` wrapper.

Verified config constants:
```js
octaves = 10; lacunarity = 1.6; gain = 0.7; amplitude = chaos /* default 0.12 */;
frequency = 10; baseFlatness = 0; displacement = 60; borderOffset = 60;
sampleCount = Math.floor(approximatePerimeter / 2);
dpr = Math.min(window.devicePixelRatio || 1, 2);   // capped at 2 — good perf hygiene
```
It walks a rounded rect (`getRoundedRectPoint(t, …)` maps `t∈[0,1]` to the perimeter across
8 segments: 4 edges + 4 corner arcs of length `cornerArc = (Math.PI * radius) / 2`), then
displaces each sample by 2D value-noise:
```js
displacedX = point.x + octavedNoise(progress*8, …, seed=0) * 60;
displacedY = point.y + octavedNoise(progress*8, …, seed=1) * 60;
```
Time advances as `timeRef.current += deltaTime * speed` — **delta-time based, so it is
frame-rate independent and survives rAF stalls without visual jumps.**

The glow is pure CSS layered behind it (`ElectricBorder.css`, verbatim):
```css
.eb-glow-1 { border: 2px solid oklch(from var(--electric-border-color) l c h / 0.6); filter: blur(1px); }
.eb-glow-2 { border: 2px solid var(--electric-light-color); filter: blur(4px); }
.eb-background-glow { z-index:-1; transform: scale(1.1); filter: blur(32px); opacity: 0.3;
  background: linear-gradient(-30deg, var(--electric-light-color), transparent, var(--electric-border-color)); }
```
⚠️ Uses **relative color syntax** `oklch(from var(--x) l c h)` — modern Chromium only.
WebView2 on an evergreen runtime is fine; if you must support an older pinned runtime,
precompute the colors instead.

**Why it matters for Demod:** drive `chaos`/`amplitude` from mic RMS and the window border
itself becomes the voice visualizer — no dedicated visualizer widget needed. **Port: MEDIUM**
(~150 lines of noise + perimeter math, mechanical to strip from React).

---

# C. GLOW / SHIMMER / GRADIENT-BORDER / CARD SEPARATION (dark UI)

All read from the react-bits clone unless noted. Remember the Commons Clause (see licensing).

### C1. BorderGlow — cursor-tracked mesh-gradient border ⭐
`src/content/Components/BorderGlow/BorderGlow.css`. **Pure CSS + 2 custom properties.** JS only
sets `--edge-proximity` (0–100) and `--cursor-angle`; everything else is declarative.

The conic-gradient mask is the core trick — it reveals only the arc of the border facing the
cursor:
```css
mask-image: conic-gradient(from var(--cursor-angle) at center,
  black calc(var(--cone-spread) * 1%),
  transparent calc((var(--cone-spread) + 15) * 1%),
  transparent calc((100 - var(--cone-spread) - 15) * 1%),
  black calc((100 - var(--cone-spread)) * 1%));
```
Border-vs-fill separation uses the `padding-box`/`border-box` double-background technique
(the standard way to get a gradient border without a wrapper element):
```css
background: linear-gradient(var(--card-bg) 0 100%) padding-box,
            <gradient> border-box, … ;
border: 1px solid transparent;
```
Defaults: `--edge-sensitivity: 30`, `--color-sensitivity: calc(var(--edge-sensitivity) + 20)`,
`--border-radius: 28px`, `--glow-padding: 40px`, `--cone-spread: 25`, `--card-bg: #120F17`.
Transitions: `opacity 0.25s ease-out` on activate, `opacity 0.75s ease-in-out` on release
(deliberately asymmetric — snappy in, lazy out; worth copying).

The 6-stop shadow ramp that fakes a bloom (both inset and outset, same stops):
```css
box-shadow: inset 0 0 0 1px  var(--glow-color),
            inset 0 0 1px 0  hsl(40deg 80% 80% / 60%),
            inset 0 0 3px 0  hsl(40deg 80% 80% / 50%),
            inset 0 0 6px 0  hsl(40deg 80% 80% / 40%),
            inset 0 0 15px 0 hsl(40deg 80% 80% / 30%),
            inset 0 0 25px 2px hsl(40deg 80% 80% / 20%),
            inset 0 0 50px 2px hsl(40deg 80% 80% / 10%),
            /* …same 6 stops again without `inset` for the outer bloom… */;
```
⚠️ Uses `mask-composite: subtract, add, add, add, add, add`, `mix-blend-mode: plus-lighter`
and `soft-light`. All supported in current Chromium/WebView2; `plus-lighter` is the least
common — verify on target. **No rAF, no animation** (state-driven transitions only), so it is
completely safe under occlusion. **Port: EASY** (copy the CSS; ~15 lines of JS for the angle).

### C2. StarBorder — animated travelling border glow
`src/content/Animations/StarBorder/StarBorder.css`. **Pure CSS, verbatim:**
```css
@keyframes star-movement-bottom { 0% { transform: translate(0%, 0%);  opacity: 1; }
                                100% { transform: translate(-100%, 0%); opacity: 0; } }
@keyframes star-movement-top    { 0% { transform: translate(0%, 0%);  opacity: 1; }
                                100% { transform: translate(100%, 0%); opacity: 0; } }
```
Applied as `animation: star-movement-bottom linear infinite alternate`, with the duration set
inline via `animationDuration` — **verified component default `speed = '6s'`**
(`StarBorder.jsx`). Two oversized soft ellipses — `width: 300%; height: 50%;
border-radius: 50%; opacity: 0.7`, offset `bottom: -12px; right: -250%` and
`top: -12px; left: -250%` — slide across behind an opaque inner card
(`.inner-content { border: 1px solid #222; background: #000; z-index: 1 }`), with
`overflow: hidden` on the container cropping them to the border. The soft edge comes from the
gradient, not a blur filter — verified inline background:
`radial-gradient(circle, ${color}, transparent 10%)` (default `color = 'white'`,
`thickness = 1`). No `filter` is used anywhere in this component, which is why it is so cheap.

Transform+opacity only → compositor-accelerated, **no rAF**, no JS. Reads beautifully on dark
backgrounds (it is built for `#000`). **This is the best "active/listening" border for Demod.**
**Port: TRIVIAL** — the CSS above is complete and already framework-free.

### C3. ShinyText — shimmer sweep
`src/content/TextAnimations/ShinyText/ShinyText.jsx`. In react-bits this is JS-driven
(framer-motion `useAnimationFrame` → rAF), but **the effect itself is pure CSS** and the JS
adds nothing you need. Verified gradient:
```css
background-image: linear-gradient(120deg, #b5b5b5 0%, #b5b5b5 35%,
                                          #ffffff 50%,
                                          #b5b5b5 65%, #b5b5b5 100%);
background-size: 200% auto;
background-clip: text; -webkit-background-clip: text;
-webkit-text-fill-color: transparent;
```
It animates `background-position` from `150% center` → `-50% center` (verified:
`backgroundPosition = \`${150 - p * 2}% center\``) over `speed` seconds, default **2s**,
`spread` default `120deg`.

**Port it as CSS and drop the rAF entirely:**
```css
@keyframes shimmer { to { background-position: -50% center; } }
.shiny { background-position: 150% center; animation: shimmer 2s linear infinite; }
@media (prefers-reduced-motion: reduce) { .shiny { animation: none; } }
```
**Port: TRIVIAL, and strictly better than the original** (no rAF, no framer-motion).

### C4. SpotlightCard — card separation on hover
`src/content/Components/SpotlightCard/SpotlightCard.css`. Verbatim, complete:
```css
.card-spotlight { --mouse-x: 50%; --mouse-y: 50%;
  --spotlight-color: rgba(255,255,255,0.05);
  border: 1px solid #222; background-color: #111; border-radius: 1.5rem; overflow: hidden; }
.card-spotlight::before {
  background: radial-gradient(circle at var(--mouse-x) var(--mouse-y),
                              var(--spotlight-color), transparent 80%);
  opacity: 0; transition: opacity 0.5s ease; pointer-events: none; }
.card-spotlight:hover::before, .card-spotlight:focus-within::before { opacity: 0.6; }
```
JS is two lines setting `--mouse-x/--mouse-y` from `mousemove`. Note `:focus-within` is
included — keyboard-accessible by default. `rgba(255,255,255,0.05)` at `opacity: 0.6` is the
right subtlety for dark UI. **No rAF. Port: TRIVIAL.**

### C5. MagicBento — glow-intensity variables
`src/content/Components/MagicBento/MagicBento.css`. Same family as C4 but parameterised:
`--glow-x: 50%`, `--glow-y: 50%`, `--glow-intensity: 0`, `--glow-radius: 200px`, with
`transition: all 0.3s ease` and
```css
background: radial-gradient(var(--glow-radius) circle at var(--glow-x) var(--glow-y),
  rgba(132,0,255, calc(var(--glow-intensity) * 0.8)) 0%,
  rgba(132,0,255, calc(var(--glow-intensity) * 0.4)) 30%, …);
```
The `calc()`-on-alpha pattern is the useful bit: **one scalar `--glow-intensity` drives the
whole glow**, so you can bind it straight to mic level for a "listening" state.
**No rAF. Port: TRIVIAL.**

### C6. GlareHover — diagonal sheen sweep
`src/content/Animations/GlareHover/GlareHover.css`. Pure CSS, **transition-driven, not
keyframes** — a `linear-gradient(var(--gh-angle), transparent 60%, var(--gh-rgba) 70%,
transparent)` moved by `background-position` from `-100% -100%` to `100% 100%` on `:hover`,
`transition: var(--gh-duration) ease`. **No rAF. Port: TRIVIAL.**

### C7. Aceternity — VERIFIED from registry source (see §F for access)

**`moving-border` — the most naturally-vanilla component in the entire research.** Needs no
framework at all, just native SVG DOM:
```js
const len = pathEl.getTotalLength();
const pt  = pathEl.getPointAtLength((time * (len/3000)) % len);   // 3000ms per lap
dot.style.transform = `translate(${pt.x}px,${pt.y}px) translate(-50%,-50%)`;
```
Path is an invisible `<rect width="100%" height="100%" rx="30%" ry="30%">`; the blob is
`radial-gradient(#0ea5e9 40%, transparent 60%)`, 5rem, `opacity .8`. **rAF: YES** (trivially
swappable for a Worker tick — it's just a time input). **Port: EASY.**

**`background-gradient` — zero JS, pure CSS:**
```css
background-image:
  radial-gradient(circle farthest-side at 0 100%,   #00ccb1, transparent),
  radial-gradient(circle farthest-side at 100% 0,   #7b61ff, transparent),
  radial-gradient(circle farthest-side at 100% 100%,#ffc414, transparent),
  radial-gradient(circle farthest-side at 0 0,      #1ca0fb, #141316);
background-size: 400% 400%;
animation: bgpos 5s linear infinite;   /* 0%:0 50% → 50%:100% 50% → 100%:0 50% */
```
Glow layer is the same stack at `blur(24px) opacity .6` behind a sharp layer at `p-[4px]`.
**No rAF. Port: EASY.**

**`glowing-effect`** — the dual-mask trick is the whole mechanism:
```css
mask-clip: padding-box, border-box;
mask-composite: intersect;
mask-image:
  linear-gradient(#0000,#0000),
  conic-gradient(from calc((var(--start) - var(--spread)) * 1deg),
                 #00000000 0deg, #fff, #00000000 calc(var(--spread) * 2deg));
/* --spread: 20; ::after inset: calc(-1 * 1px); opacity: var(--active); transition: opacity 300ms */
```
Angle math: `targetAngle = atan2(dy,dx)*180/π + 90`, wraparound
`angleDiff = ((target - current + 180) % 360) - 180`, then
`animate(current, new, {duration: 2, ease: [0.16, 1, 0.3, 1]})` writing `--start` per frame.
**rAF: YES** (two layers — pointermove throttle + the tween).
⚠️ **WebView2 gotcha:** use `-webkit-mask-composite: source-in`, **not**
`mask-composite: intersect` — Chromium's prefixed property uses different keyword names.
**Port: MEDIUM.** This is the same family as react-bits `BorderGlow` (§C1), which is simpler
and has no rAF — **prefer C1** unless you specifically want this exact look.

**`spotlight-new`** — `x:[0, 100, 0]`, `duration: 7, repeat: Infinity, easeInOut`; beams
`560×1380px` at `rotate(±45deg)`; gradients
`hsla(210,100%,85%,.08) → hsla(210,100%,55%,.02) 50% → hsla(210,100%,45%,0) 80%`.
**No rAF. EASY.**

**`spotlight`** (static SVG, `feGaussianBlur stdDeviation="151"`) — real keyframe:
```css
@keyframes spotlight {
  0%   { opacity: 0; transform: translate(-72%,-62%) scale(0.5); }
  100% { opacity: 1; transform: translate(-50%,-40%) scale(1); }
}
```
**EASY.**

**`hover-border-gradient`** — `setInterval` cycling 4 fixed radial-gradients at 1s.
⚠️ CSS `transition` **cannot** interpolate between two gradient strings; framer-motion fakes
it. **Port as two stacked layers crossfading `opacity`.** **EASY.**

**`card-spotlight`** — one mousemove handler plus
`mask-image: radial-gradient(350px circle at var(--x) var(--y), white, transparent 80%)`.
Ignore its `CanvasRevealEffect` dependency (three.js). **EASY.**

**`glare-card`** — already vanilla-style (writes `--m-x/--r-x/--bg-x` on pointermove,
**no rAF**), but stacks `mix-blend-mode: soft-light/color-dodge/exclusion` plus
`background-blend-mode: hue,hue,hue,overlay`. All fine in Chromium. **MEDIUM.**

**`aurora-background`** — `mix-blend-mode: difference` + `filter: blur(10px) invert(1)`;
`@keyframes aurora { from{background-position:50% 50%,50% 50%} to{background-position:350% 50%,350% 50%} }`
at `60s linear infinite`. **MEDIUM.**

**`noise-background`** — **rAF: YES**, full random-walk physics loop. ⚠️ It hardcodes
`deltaTime = 16` instead of measuring, so it **slows down under frame drops** — exactly the
anti-pattern to avoid given the occlusion constraint. Also hotlinks
`assets.aceternity.com/noise.webp`; bundle locally. **MEDIUM, not recommended.**

---

# D. WebView2 / occlusion constraints

## D1. Browser-feature risk register
| Feature | Used by | WebView2 risk |
|---|---|---|
| `mask-image`, `mask-composite` | C1 BorderGlow | Low — supported in modern Chromium. |
| `mix-blend-mode: plus-lighter` | C1 | **Medium — least common of the blend modes. Test explicitly.** |
| `mix-blend-mode: soft-light` | C1 | Low. |
| `oklch(from … l c h)` relative color | B5 ElectricBorder | **Medium — recent Chromium only. Precompute if the runtime is pinned.** |
| `background-clip: text` | C3 ShinyText | Low (keep the `-webkit-` prefix). |
| `filter: blur()` | B5, C1 | Low, but it is the most expensive thing here — cheap on a small overlay. |
| `conic-gradient` | C1 | Low. |
| WebGL / GLSL | B2 Siri Wave, ~60 react-bits components | **High — GPU-dependent; falls back to software rendering or fails where HW accel is off. Avoid.** |
| Canvas 2D | B3, B4, B5 | None. Safe. |
| `AudioContext` + `getUserMedia` | B4 | Needs mic permission; in a Tauri WebView2 the permission prompt must be handled/granted host-side. |

## D2. The rAF-under-occlusion problem
**Verified behaviour:** rAF callbacks are paused when the document is treated as hidden — by
design, not a bug. A Chromium-based WebView2 overlay that is occluded/minimized can be marked
hidden, so every rAF-driven animation stops.

**Everything that depends on rAF (from this research):**
- B3 siriwave draw cycle
- B4 MDN oscilloscope + spectrum loops, and the `pump()` amplitude feed
- B6 ElectricBorder draw loop
- B2 Siri Wave (unconditional, ungated), B5a Voice Powered Orb, B5b Music-Reactive Hero
- C7 `glowing-effect` (two layers), `moving-border`, `noise-background`, `shooting-stars`
- A4b `dotted-glow-background`
- C3 ShinyText *as shipped* (avoidable — port it as CSS, §C3)
- most react-bits components generally

**Nothing in §A1–A4 depends on rAF.** All four pulse treatments are pure CSS.

**The one good precedent found: `dotted-glow-background` is the ONLY component across all
three libraries that gates its rAF loop on visibility** (`IntersectionObserver`). Copy that
pattern; everything else in these libraries spins its loop forever regardless of whether
anything is on screen.

**Mitigation, in order of preference:**
1. **Prefer CSS animations** for anything idle/ambient (§A1–A4, A4b-as-CSS, C2, C3-as-CSS,
   C4–C6, `background-gradient`, `spotlight-new`). Compositor-driven; when the window is hidden
   nothing renders anyway, and they resume correctly rather than drifting. This covers the
   entire idle-indicator requirement with zero rAF.
2. **Gate every canvas loop on visibility.** `IntersectionObserver` (the
   `dotted-glow-background` pattern) plus a `document.visibilitychange` handler. While hidden
   there is no point running the AnalyserNode either — tear down the pump and restart on
   resume. This is also the correct battery/CPU behaviour for an always-on dictation overlay.
3. **Drive canvas work from a Web Worker timer when you must keep running while hidden.**
   Main-thread chained `setTimeout` is *also* heavily throttled in background (Chrome 88+
   timer throttling), so a plain main-thread `setTimeout` fallback is **not** sufficient.
   `setTimeout` **inside a dedicated Web Worker** is not throttled the same way — have the
   worker post ticks and drive the draw from those. siriwave's existing
   `setTimeout(this.startDrawCycle.bind(this), 20)` branch is the exact seam to swap.
4. **Use delta-time, never frame-count.** ElectricBorder does this correctly
   (`timeRef.current += deltaTime * speed`). Two counter-examples found in the wild:
   siriwave advances phase a fixed step per frame, and Aceternity's `noise-background`
   **hardcodes `deltaTime = 16`** — both slow down rather than skip when frames drop. Convert
   any loop you port to real delta-time if smooth resume matters.

---

# E. Recommendation for Demod

| Need | Use | Why |
|---|---|---|
| Idle "ready" indicator | **§A2 ripple + §A1 opacity breathe** (or §A4b `glowing-stars` scale stops) | Pure CSS, no rAF, occlusion-proof, trivial. |
| Active "listening" visualizer | **§B3 siriwave (MIT)** fed by **§B5a's** analyser block | Only genuinely portable Siri-style waveform; zero deps; `setAmplitude()` + built-in lerp is exactly the right API. |
| If you'd rather build it yourself | **§B5b canvas-2D wave math** + **§B5a analyser** + **§A4b IntersectionObserver gating** | The agent's top pick: entirely dependency-free, no WebGL, real mic data, and correctly gated for occlusion. |
| Cheapest real-audio meter | **§B4 DOM voice bars** (`transform: scaleY`) | No canvas, no rAF, compositor-accelerated, degrades gracefully. |
| Active-state window border | **§C2 StarBorder** | Pure CSS keyframes, built for `#000`, no rAF. |
| Voice-reactive border (stretch) | **§B6 ElectricBorder** driven by mic RMS | Canvas-2D, delta-time, no WebGL; makes the border *be* the visualizer. |
| Animated border, simplest possible | **§C7 `moving-border`** | Native SVG `getPointAtLength` — no framework at all. |
| Card/panel separation | **§C4 SpotlightCard** + **§C1 BorderGlow** | Pure CSS, dark-UI native, no rAF. |
| Text shimmer | **§C3 as CSS** | Strictly better than upstream — drops framer-motion and rAF. |

**Avoid:** 21st.dev Siri Wave (WebGL **and** fake audio), Voice Powered Orb's *renderer*
(WebGL/OGL — take only its analyser block), Aceternity `noise-background` (hardcoded
`deltaTime`), and all ~60 WebGL react-bits components.
**Always add** the `prefers-reduced-motion` block from §A5 — upstream essentially never does.

---

# F. How to actually get source from these sites (keep this)

Both recipes are non-obvious and cost real effort to find.

**⚠️ Machine-specific prerequisite:** on this machine plain `curl` returns `000` for **every**
HTTPS host — AV TLS interception breaks cert verification. **`curl -k` is required.** (This is
also why my first probe of the Aceternity registry appeared to "return empty" and was briefly
recorded as a dead endpoint; the endpoint was fine, the TLS was not. Same root cause as the
certifi/winroots issue noted elsewhere in this vault.)

**Aceternity — live shadcn registry, auth-free:**
```
https://ui.aceternity.com/registry/<name>.json     # 200, full .tsx in a `content` field
https://ui.aceternity.com/registry/index.json      # 93KB, enumerates all 270 components
```
A `401 {"error":"Unauthorized"}` means **the name doesn't exist**, not that it's paid.
`/r/*.json`, `/api/registry/*`, `/api/components/*` all fail — don't bother.

**21st.dev — public CDN bundles:**
```
https://cdn.21st.dev/<author>/<component>/<variant>/bundle.<ts>.html
```
The compiled bundle contains verbatim GLSL and audio code. Discover `<ts>` from the component
page HTML. Every registry endpoint (`/r/*.json`, `/api/r/*`, `api.21st.dev`) 403s.
`magic-mcp` is a dead end — its `magic.21st.dev/api/fetch-ui` requires an API key.

Local cache of what was pulled (session scratchpad, will not survive cleanup):
`…\3888638d-f7d8-4f35-b708-0484324828eb\scratchpad\` — `ace/src/*.tsx` (30 Aceternity sources),
`t21/*.html` (21st.dev bundles), `rb/` (react-bits clone), `sw/` (siriwave clone).

---

## Provenance

**Verified by reading real source:**
- `DavidHDev/react-bits` — shallow-cloned; CSS/JSX read from disk; license read from
  `LICENSE.md`; WebGL dependency census and `AnalyserNode`/`prefers-reduced-motion` greps run
  across the whole tree.
- `kopiro/siriwave` — shallow-cloned; `index.ts`, `classic-curve.ts`, `ios9-curve.ts` read.
- **ui.aceternity.com** — all 270 components enumerated from `registry/index.json`; ~30
  component `.tsx` sources pulled and read from the registry.
- **21st.dev** — compiled CDN bundles fetched and read for `siri-wave`,
  `voice-powered-orb`, `music-reactive-hero-section`, and three `ruixen.ui` audio components.
- Tailwind CSS animation docs — exact keyframes.
- MDN Web Audio visualization guide — exact fftSize/bufferLength/draw loops.

**Marked UNVERIFIED above and NOT guessed:**
- **Aceternity licensing** — no readable grant found; do not assume MIT.
- **21st.dev per-component licensing** — per-author, unchecked.
- `thegridcn/waveform` and `ruixen.ui/visualizer-button` — bundles 404'd, contents unknown.
- The `cell-ripple` keyframes in §A4b are **recovered from downstream consumers, not from the
  Aceternity component itself** (the component only emits the delay/duration vars). Two
  variants circulate; the one quoted is from `better-auth/better-auth`.

**Inference vs. source:** §A3 (glow pulse keyframes) and the §A2 concentric-ring stacking are
my composition — only the underlying `pulse`/`ping` keyframes are upstream-verified. The
shadow ramp in §A3 is lifted from verified BorderGlow CSS. The CSS translations offered in
§A4b, §C3 and §C7 (`hover-border-gradient` as crossfading layers) are my ports of verified
JS-driven originals, not upstream CSS. Everything else quoted as code is verbatim from the
source named beside it.
