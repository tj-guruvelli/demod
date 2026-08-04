import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface Settings {
  microphone: string;
  engine: string;
  whisperModel: string;
  groqApiKey: string;
  recordingMode: string;
  hotkey: string;
  language: string;
  interactionSounds?: boolean;
  overlayVisible?: boolean;
  // Already deserialized server-side (pill_hittest.rs) — read-only display only,
  // never written back from the frontend. See Overlay → Dock position.
  overlayX?: number | null;
  overlayY?: number | null;
  // Groq model id (settings.rs cloud_model, #[serde(rename = "cloudModel")]).
  cloudModel?: string;
  // Custom vocabulary biasing recognition on both engines (settings.rs dictionary).
  dictionary?: string;
  // Text-processing toggles applied to the transcript before it's delivered
  // (cleanup.rs / recorder.rs). All round-trip through get_settings/save_settings.
  autoPaste?: boolean;
  trailingSpace?: boolean;
  capitalize?: boolean;
  ensurePunctuation?: boolean;
  voiceSensitivity?: number;
  whisperThreshold?: number;
  // History (history.rs) — persisted local-history opt-in and auto-delete window.
  historyEnabled?: boolean;
  historyRetentionDays?: number;
  // Clean-up & AI edits (cleanup.rs) — both persisted, both default true.
  removeFillers?: boolean;
  autoEdits?: boolean;
}

interface MicDevice {
  name: string;
  is_default: boolean;
}

interface HistoryItem {
  id: string;
  text: string;
  tsMs: number;
  engine: string;
  model: string;
  charCount: number;
  durationMs: number | null;
}

interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

// DOM elements
const appNotice = document.getElementById("app-notice")!;
const statusDot = document.getElementById("status-dot")!;
const statusText = document.getElementById("status-text")!;
const micSelect = document.getElementById("mic-select") as HTMLSelectElement;
const engineLocal = document.getElementById("engine-local")!;
const engineCloud = document.getElementById("engine-cloud")!;
const localSettings = document.getElementById("local-settings")!;
const cloudSettings = document.getElementById("cloud-settings")!;
const modelList = document.getElementById("model-list")!;
const downloadProgress = document.getElementById("download-progress")!;
const progressFill = document.getElementById("progress-fill")!;
const groqKey = document.getElementById("groq-key") as HTMLInputElement;
const groqKeySaveBtn = document.getElementById("groq-key-save") as HTMLButtonElement;
const groqKeyStatus = document.getElementById("groq-key-status") as HTMLElement;
const groqTestBtn = document.getElementById("groq-test-btn") as HTMLButtonElement;
const groqTestStatus = document.getElementById("groq-test-status") as HTMLElement;
const modeAuto = document.getElementById("mode-auto")!;
const modeToggle = document.getElementById("mode-toggle")!;
const modePtt = document.getElementById("mode-ptt")!;
const hotkeyText = document.getElementById("hotkey-text")!;
const hotkeyCapture = document.getElementById("hotkey-capture") as HTMLButtonElement;
const hotkeyMessage = document.getElementById("hotkey-message")!;
const localLanguageSettings = document.getElementById("local-language-settings")!;
const languageSelect = document.getElementById("language-select") as HTMLSelectElement;
const cloudModelSelect = document.getElementById("cloud-model-select") as HTMLSelectElement;
const dictionaryTextarea = document.getElementById("dictionary-textarea") as HTMLTextAreaElement;
const voiceSensitivitySlider = document.getElementById("voice-sensitivity-slider") as HTMLInputElement;
const voiceSensitivityValue = document.getElementById("voice-sensitivity-value") as HTMLSpanElement;
const whisperThresholdSlider = document.getElementById("whisper-threshold-slider") as HTMLInputElement;
const whisperThresholdValue = document.getElementById("whisper-threshold-value") as HTMLSpanElement;
const textToggleList = document.getElementById("text-toggle-list")!;
const soundsSwitch = document.getElementById("sounds-switch")!;
const overlaySwitch = document.getElementById("overlay-switch")!;
const autostartSwitch = document.getElementById("autostart-switch")!;
const dockPositionText = document.getElementById("dock-position-text")!;
const footerSidecar = document.getElementById("footer-sidecar")!;
const footerModel = document.getElementById("footer-model")!;
const savedToast = document.getElementById("saved-toast")!;
const footerHotkey = document.getElementById("footer-hotkey")!;
const historyList = document.getElementById("history-list")!;
const historyCount = document.getElementById("history-count")!;
const historyEmpty = document.getElementById("history-empty")!;
const historyClearBtn = document.getElementById("history-clear-btn") as HTMLButtonElement;
const historyEnabledSwitch = document.getElementById("history-enabled-switch")!;
const historyRetentionSelect = document.getElementById("history-retention-select") as HTMLSelectElement;
const privacyClearHistoryBtn = document.getElementById("privacy-clear-history-btn") as HTMLButtonElement;
const historySearch = document.getElementById("history-search") as HTMLInputElement;
const historyStats = document.getElementById("history-stats")!;
const historyStatTotal = document.getElementById("history-stat-total")!;
const historyStatWords = document.getElementById("history-stat-words")!;
const historyStatSaved = document.getElementById("history-stat-saved")!;
const historyEmptyTitle = document.getElementById("history-empty-title")!;
const removeFillersSwitch = document.getElementById("remove-fillers-switch")!;
const autoEditsSwitch = document.getElementById("auto-edits-switch")!;
const dataflowCloud = document.getElementById("dataflow-cloud")!;

// Section navigation — role=tablist/tab/tabpanel pattern (chosen over plain
// focusable buttons because the sidebar genuinely IS a section switcher with
// exactly one visible panel at a time — the ARIA tab pattern maps 1:1 onto
// that model). 150ms opacity/transform crossfade (matches overlay.html's own
// ease-in/ease-out grammar) instead of an instant display swap.
const navItems = Array.from(document.querySelectorAll<HTMLElement>(".nav-item"));

// Tracked synchronously rather than read back via `.content-section.active`.
// `.active` is only added to `next` asynchronously (rAF/setTimeout, see
// `reveal` below), so a second activateTab() call arriving before that
// callback runs would find NO section with `.active` yet, treat `current` as
// null, skip its cleanup entirely, and leave it stuck visible. Reproduced by
// firing all six tabs back-to-back with zero delay: five sections ended up
// simultaneously `display:block; opacity:1`. Tracking the intended current
// section in a plain variable, updated the instant a switch is decided
// (not when the animation happens to catch up), makes cleanup targeting
// correct regardless of how fast switches are requested.
let activeSectionId: string | null =
  (document.querySelector(".content-section.active") as HTMLElement | null)?.id ?? null;

function activateTab(item: HTMLElement, { focus = true }: { focus?: boolean } = {}) {
  const target = item.getAttribute("data-section");
  const current = activeSectionId ? document.getElementById(activeSectionId) : null;
  const next = document.getElementById(`section-${target}`) as HTMLElement | null;
  if (!next || next === current) {
    if (focus) item.focus();
    return;
  }
  activeSectionId = next.id;

  // Refresh every time the History tab opens rather than relying on the
  // load-time render staying current — a dictation made while another tab
  // is open must show up without reopening the whole window.
  if (target === "history") {
    renderHistory().catch((err) => console.error("[Demod] renderHistory failed:", err));
  }

  navItems.forEach((n) => {
    n.classList.remove("active");
    n.setAttribute("aria-selected", "false");
    n.tabIndex = -1;
  });
  item.classList.add("active");
  item.setAttribute("aria-selected", "true");
  item.tabIndex = 0;
  if (focus) item.focus();

  if (current) {
    current.classList.add("leaving");
    current.classList.remove("active");
    // CRITICAL REGRESSION FIX: cleanup used to run ONLY on "transitionend".
    // Transition events never fire when the window is hidden/occluded/
    // backgrounded (same class of bug as rAF pausing) — so on a window that
    // was hidden at the moment of a tab switch, `current` was left stuck
    // with the "leaving" class forever. That alone was cosmetic (the section
    // was hidden anyway), EXCEPT the section it belongs to could later become
    // `next` again on a subsequent switch. `next.classList.add("active")`
    // below only ADDS "active" — it never removed a leftover "leaving" — so
    // a section could carry BOTH classes at once. Because `.content-section
    // .leaving { opacity: 0 }` is declared AFTER `.content-section.active
    // { opacity: 1 }` in style.css, equal-specificity cascade means .leaving
    // wins: the pane got `display:block` but stayed invisible — a completely
    // blank content area with a correctly-selected sidebar tab. Reproduced
    // directly: General -> Recording -> General leaves General blank.
    // Fix has two parts: (1) never let a section be revealed while still
    // carrying "leaving" (below), and (2) never let cleanup depend solely on
    // an event that can fail to fire — the setTimeout fallback guarantees it
    // runs regardless, matching the rAF+setTimeout pattern already used for
    // `reveal` below. Whichever runs first wins; both are idempotent.
    const cleanupCurrent = () => {
      // Guard against A -> B -> A back-to-back: if `current` was re-selected
      // as `next` again before this fallback fired, it now carries "active"
      // (not "leaving") and must be left alone — otherwise this stale
      // callback would hide the section the user is currently looking at.
      if (!current.classList.contains("leaving")) return;
      current.classList.remove("leaving");
      current.style.display = "none";
    };
    current.addEventListener("transitionend", cleanupCurrent, { once: true });
    setTimeout(cleanupCurrent, 200);
  }
  // Never reveal a section that still carries a stale "leaving" class from a
  // prior switch whose cleanup hadn't run yet — see the note above.
  next.classList.remove("leaving");
  next.style.display = "block";
  // rAF so the browser commits `display:block` before the class flips -
  // otherwise the opacity/transform transition has no paint to animate from.
  //
  // The timeout is not redundant. `.active` carries opacity:1, so the pane is
  // display:block but INVISIBLE until this callback runs - and rAF is paused
  // in hidden, minimised or occluded windows. Measured directly: in a
  // backgrounded window rAF did not fire within 600ms and the pane sat at
  // opacity 0. Never let a functional visibility change depend on a frame
  // being painted; the animation is decoration, the content is not.
  // Whichever fires first wins; classList.add is idempotent.
  const reveal = () => {
    // Guard against a superseded switch: if another activateTab() call has
    // since made a different section current, this callback belongs to a
    // switch the user has already navigated away from. Without this check,
    // firing anyway blindly re-adds "active" to a section that was
    // correctly marked "leaving" (or already hidden) by the newer switch —
    // reproduced by firing all six tabs back-to-back with no delay: every
    // superseded section ended up stuck as `display:block` with the
    // "active" class (opacity:0 from a still-pending "leaving" transition,
    // but present in the layout and in the DOM's notion of "active").
    if (activeSectionId !== next.id) return;
    next.classList.remove("leaving");
    next.classList.add("active");
  };
  requestAnimationFrame(reveal);
  setTimeout(reveal, 60);
}

navItems.forEach((item) => {
  item.addEventListener("click", () => activateTab(item, { focus: false }));
});

// Roving tabindex arrow-key navigation — required for the tab pattern to be
// keyboard-usable at all: Up/Down (and Left/Right) move between sections,
// Home/End jump to the first/last. Only the active tab is in the Tab order;
// arrow keys move both focus and selection between the others.
document.getElementById("sidebar-nav")!.addEventListener("keydown", (e) => {
  const key = (e as KeyboardEvent).key;
  const currentIndex = navItems.findIndex((n) => n === document.activeElement);
  if (currentIndex === -1) return;

  let nextIndex: number | null = null;
  if (key === "ArrowDown" || key === "ArrowRight") {
    nextIndex = (currentIndex + 1) % navItems.length;
  } else if (key === "ArrowUp" || key === "ArrowLeft") {
    nextIndex = (currentIndex - 1 + navItems.length) % navItems.length;
  } else if (key === "Home") {
    nextIndex = 0;
  } else if (key === "End") {
    nextIndex = navItems.length - 1;
  }

  if (nextIndex !== null) {
    e.preventDefault();
    activateTab(navItems[nextIndex]);
  }
});

// Window drag — titlebar and sidebar empty space
const titlebar = document.getElementById("titlebar")!;
const sidebar = document.getElementById("sidebar")!;
const appWindow = getCurrentWindow();

titlebar.addEventListener("mousedown", (e) => {
  if ((e.target as HTMLElement).closest("button, select, input, a, .nav-item")) return;
  appWindow.startDragging();
});

sidebar.addEventListener("mousedown", (e) => {
  if ((e.target as HTMLElement).closest("button, select, input, a, .nav-item")) return;
  appWindow.startDragging();
});

let currentSettings: Settings;

// Static catalog matching exactly the sizes settings.rs / download_model
// already support (US-014/015 §4.3 — do not list a model id Rust can't download).
// "large-v3" id maps to ggml-large-v3.bin via the existing ggml-{id}.bin URL
// scheme in transcribe_local::model_download_url — verified present at
// https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin


const MODEL_CATALOG = [
  { id: "small", label: "Small", sizeLabel: "~466 MB", speedLabel: "Fast" },
  { id: "medium", label: "Medium", sizeLabel: "~1.5 GB", speedLabel: "Balanced" },
  { id: "large-v3", label: "Large", sizeLabel: "~2.9 GB", speedLabel: "Slowest" },
] as const;

// Only one download can run at a time — tracked so every row's Download button
// disables while any download is in flight (shared progress bar under the list).
let activeDownloadId: string | null = null;

// Common languages whisper.cpp supports well, plus "auto" for detection.
// Passed straight through to the Rust `language` setting -> whisper.cpp `-l`
// (transcribe_local.rs); the Groq cloud engine hardcodes "en" server-side and
// has no language setting, which is why this card is local-engine only.
const LANGUAGE_CATALOG = [
  { id: "auto", label: "Auto-detect" },
  { id: "en", label: "English" },
  { id: "es", label: "Spanish" },
  { id: "fr", label: "French" },
  { id: "de", label: "German" },
  { id: "it", label: "Italian" },
  { id: "pt", label: "Portuguese" },
  { id: "nl", label: "Dutch" },
  { id: "ru", label: "Russian" },
  { id: "zh", label: "Chinese" },
  { id: "ja", label: "Japanese" },
  { id: "ko", label: "Korean" },
  { id: "hi", label: "Hindi" },
  { id: "ar", label: "Arabic" },
] as const;

// Groq cloud model ids — must match a model transcribe_groq.rs actually
// accepts (settings.rs default_cloud_model / cloud_model). Adding a new Groq
// model later is a one-line addition here; nothing else changes.
const CLOUD_MODEL_CATALOG = [
  {
    id: "whisper-large-v3",
    label: "Large v3 (most accurate)",
  },
  {
    id: "whisper-large-v3-turbo",
    label: "Large v3 Turbo (fastest, default)",
  },
  {
    id: "distil-whisper-large-v3-en",
    label: "Distil Large v3 English (fastest, English-only)",
  },
] as const;

// Generic <select> populator shared by every catalog-backed dropdown
// (Language, Cloud model, and any future one) — one renderer, data-driven.
function populateSelectCatalog(
  select: HTMLSelectElement,
  catalog: readonly { id: string; label: string }[]
) {
  if (select.options.length > 0) return;
  catalog.forEach((entry) => {
    const option = document.createElement("option");
    option.value = entry.id;
    option.textContent = entry.label;
    select.appendChild(option);
  });
}

// Text-processing toggles applied to the transcript before delivery
// (cleanup.rs on the Rust side). Each entry is a one-line addition to expose
// a new boolean setting as a switch row — renderToggleRows() below is the
// single generic renderer/handler for all of them.
interface ToggleCatalogEntry {
  key: "autoPaste" | "trailingSpace" | "capitalize" | "ensurePunctuation";
  label: string;
  hint: string;
  default: boolean;
}

const TEXT_TOGGLE_CATALOG: ToggleCatalogEntry[] = [
  {
    key: "autoPaste",
    label: "Auto-paste transcript",
    hint: "Type the transcript into the focused app. When off, it's placed on the clipboard for you to paste yourself.",
    default: true,
  },
  {
    key: "trailingSpace",
    label: "Trailing space",
    hint: "Append a space after the transcript so back-to-back dictations don't run together.",
    default: false,
  },
  {
    key: "capitalize",
    label: "Capitalize sentences",
    hint: "Capitalize the first letter of each sentence.",
    default: true,
  },
  {
    key: "ensurePunctuation",
    label: "Ensure punctuation",
    hint: "Add a period if the transcript has no terminal punctuation.",
    default: true,
  },
];

// Single generic renderer for the whole toggle catalog: builds every switch
// row from data, and (via the delegated listener registered once below)
// writes to currentSettings AND calls saveSettings() on every click — the
// one code path that can't silently forget to persist (footgun #13).
function renderToggleRows(container: HTMLElement, entries: ToggleCatalogEntry[]) {
  container.innerHTML = entries
    .map((t) => {
      const checked = Boolean(currentSettings[t.key] ?? t.default);
      return `
      <div class="setting-row">
        <div class="setting-label">
          <span class="label-text">${t.label}</span>
          <span class="label-hint">${t.hint}</span>
        </div>
        <div class="setting-control">
          <button
            type="button"
            class="switch${checked ? " active" : ""}"
            role="switch"
            aria-checked="${checked}"
            aria-label="${t.label}"
            data-toggle-key="${t.key}"
          >
            <span class="switch-knob"></span>
          </button>
        </div>
      </div>`;
    })
    .join("");
}

function formatHotkey(hotkey: string): string {
  const tokenMap: Record<string, string> = {
    CmdOrCtrl: "Ctrl",
    Cmd: "Ctrl",
    Super: "Win",
  };
  return hotkey
    .split("+")
    .map((token) => tokenMap[token] ?? token)
    .join("+");
}

async function loadSettings() {
  currentSettings = await invoke<Settings>("get_settings");

  // Populate mic dropdown
  const mics = await invoke<MicDevice[]>("list_microphones");
  micSelect.innerHTML = "";
  if (mics.length === 0) {
    // An empty dropdown would leave the user guessing why dictation does
    // nothing. For a dictation app this is a critical condition, so name it.
    const option = document.createElement("option");
    option.textContent = "No microphone detected";
    option.disabled = true;
    option.selected = true;
    micSelect.appendChild(option);
    micSelect.disabled = true;
    showAppNotice("No microphone detected, so Demod cannot record. Connect one and reopen Settings.");
  } else {
    micSelect.disabled = false;
    mics.forEach((mic) => {
      const option = document.createElement("option");
      option.value = mic.name;
      option.textContent = mic.name + (mic.is_default ? " (default)" : "");
      micSelect.appendChild(option);
    });
    // If saved mic name not in list (e.g. empty string or removed device),
    // fall back to the default device so dictation works out of the box.
    micSelect.value = currentSettings.microphone;
    if (!micSelect.value) {
      const defaultMic = mics.find((m) => m.is_default);
      if (defaultMic) micSelect.value = defaultMic.name;
    }
  }

  // Engine
  setEngine(currentSettings.engine);

  // Model — per-model cards, downloaded-vs-available state
  await renderModelCards();

  // Groq key
  groqKey.value = currentSettings.groqApiKey;

  // Recording mode
  setRecordingMode(currentSettings.recordingMode);

  // Hotkey
  hotkeyText.textContent = formatHotkey(currentSettings.hotkey);
  footerHotkey.textContent = formatHotkey(currentSettings.hotkey);

  // Language (local engine only; see setEngine)
  if (languageSelect.options.length === 0) {
    LANGUAGE_CATALOG.forEach((l) => {
      const option = document.createElement("option");
      option.value = l.id;
      option.textContent = l.label;
      languageSelect.appendChild(option);
    });
  }
  setLanguage(currentSettings.language ?? "en");

  // Cloud model (Groq, cloud engine only)
  populateSelectCatalog(cloudModelSelect, CLOUD_MODEL_CATALOG);
  setCloudModel(currentSettings.cloudModel ?? "whisper-large-v3-turbo");

  // Dictionary (custom vocabulary, both engines)
  dictionaryTextarea.value = currentSettings.dictionary ?? "";

  // Voice detection (silence gate + whisper threshold)
  const vs = currentSettings.voiceSensitivity ?? 0.035;
  voiceSensitivitySlider.value = String(vs);
  voiceSensitivityValue.textContent = vs.toFixed(3);
  const wt = currentSettings.whisperThreshold ?? 0.5;
  whisperThresholdSlider.value = String(wt);
  whisperThresholdValue.textContent = wt.toFixed(2);

  // Text-processing toggles (auto-paste, trailing space, capitalize, punctuation)
  renderToggleRows(textToggleList, TEXT_TOGGLE_CATALOG);

  // Clean-up & AI edits
  setRemoveFillers(currentSettings.removeFillers ?? true);
  setAutoEdits(currentSettings.autoEdits ?? true);

  // History (opt-in local transcript log) + Privacy retention controls
  const historyEnabled = currentSettings.historyEnabled ?? true;
  historyEnabledSwitch.classList.toggle("active", historyEnabled);
  historyEnabledSwitch.setAttribute("aria-checked", String(historyEnabled));
  // Clamp to a known option rather than trusting a persisted value verbatim —
  // an unrecognized value would leave the <select> showing nothing selected.
  const retentionOptions = ["0", "7", "30", "90"];
  const retentionValue = String(currentSettings.historyRetentionDays ?? 0);
  historyRetentionSelect.value = retentionOptions.includes(retentionValue) ? retentionValue : "0";
  await renderHistory();

  // Interaction sounds
  setInteractionSounds(currentSettings.interactionSounds ?? true);

  // Show overlay
  setOverlayVisible(currentSettings.overlayVisible ?? true);

  // Dock position (Overlay tab) — read-only, sourced from the already-deserialized
  // overlayX/overlayY; null means the pill has never been dragged off default.
  const { overlayX, overlayY } = currentSettings;
  dockPositionText.textContent =
    overlayX != null && overlayY != null
      ? `Custom (${overlayX}, ${overlayY})`
      : "default (bottom center)";
}

function setEngine(engine: string) {
  currentSettings.engine = engine;
  const isLocal = engine === "local";
  engineLocal.classList.toggle("active", isLocal);
  engineLocal.setAttribute("aria-checked", String(isLocal));
  engineCloud.classList.toggle("active", !isLocal);
  engineCloud.setAttribute("aria-checked", String(!isLocal));
  localSettings.classList.toggle("hidden", engine !== "local");
  cloudSettings.classList.toggle("hidden", engine !== "cloud");
  // Language only reaches whisper.cpp (transcribe_local.rs); the Groq cloud
  // path hardcodes "en" server-side and has no language parameter at all.
  localLanguageSettings.classList.toggle("hidden", engine !== "local");
  refreshAdvisories();
}

function setLanguage(language: string) {
  currentSettings.language = language;
  languageSelect.value = language;
}

function setCloudModel(model: string) {
  currentSettings.cloudModel = model;
  cloudModelSelect.value = model;
}

function setRecordingMode(mode: string) {
  currentSettings.recordingMode = mode;
  modeAuto.classList.toggle("active", mode === "auto");
  modeAuto.setAttribute("aria-checked", String(mode === "auto"));
  modeToggle.classList.toggle("active", mode === "toggle");
  modeToggle.setAttribute("aria-checked", String(mode === "toggle"));
  modePtt.classList.toggle("active", mode === "push-to-talk");
  modePtt.setAttribute("aria-checked", String(mode === "push-to-talk"));
}

function setInteractionSounds(enabled: boolean) {
  currentSettings.interactionSounds = enabled;
  soundsSwitch.classList.toggle("active", enabled);
  soundsSwitch.setAttribute("aria-checked", String(enabled));
  refreshAdvisories();
}

function setOverlayVisible(enabled: boolean) {
  currentSettings.overlayVisible = enabled;
  overlaySwitch.classList.toggle("active", enabled);
  overlaySwitch.setAttribute("aria-checked", String(enabled));
  refreshAdvisories();
}

function setRemoveFillers(enabled: boolean) {
  currentSettings.removeFillers = enabled;
  removeFillersSwitch.classList.toggle("active", enabled);
  removeFillersSwitch.setAttribute("aria-checked", String(enabled));
}

function setAutoEdits(enabled: boolean) {
  currentSettings.autoEdits = enabled;
  autoEditsSwitch.classList.toggle("active", enabled);
  autoEditsSwitch.setAttribute("aria-checked", String(enabled));
  updateDataflowCopy();
}

// Privacy tab's cloud data-flow line: honest about whether the transcript
// (not just audio) also goes to Groq, which only happens when auto-edits is on.
function updateDataflowCopy() {
  const autoEdits = currentSettings?.autoEdits ?? true;
  dataflowCloud.innerHTML = autoEdits
    ? "<strong>Cloud (Groq):</strong> audio is sent to Groq for transcription, and the transcript is sent to Groq for AI clean-up."
    : "<strong>Cloud (Groq):</strong> audio is sent to Groq for transcription.";
}

/// Reflects real Windows autostart state in the switch. This is NOT part of
/// `currentSettings`/save_settings — it is backed by the registry Run key
/// (and legacy Startup-folder shortcut) directly, read/written via their own
/// commands, so the switch position must always be set from an actual read
/// or write result, never assumed.
function setAutostartSwitch(enabled: boolean) {
  autostartSwitch.classList.toggle("active", enabled);
  autostartSwitch.setAttribute("aria-checked", String(enabled));
}

/// Single owner of the advisory state. Two independent warnings that each
/// cleared "the" notice stomped each other, and a successful save wiped one
/// of them outright - so all advisory decisions live here, with explicit
/// precedence, and every caller just re-runs this.
///
/// Errors outrank advice: a real failure is never replaced by a hint. Among
/// advisories, cloud-without-a-key wins, because dictation will fail outright
/// rather than merely fail to show something.
function refreshAdvisories() {
  if (appNotice.dataset.kind === "error") return;
  const noKey =
    currentSettings.engine === "cloud" && (currentSettings.groqApiKey ?? "").trim() === "";
  const silent =
    currentSettings.overlayVisible === false && currentSettings.interactionSounds === false;
  if (noKey) {
    showAppNotice("Groq Cloud is selected but no API key is set, so dictation will fail.", "warn");
  } else if (silent) {
    showAppNotice(
      "With both the overlay and interaction sounds off, nothing will show that Demod is recording.",
      "warn"
    );
  } else if (appNotice.dataset.kind === "warn") {
    clearAppNotice();
  }
}


// Per-model cards: downloaded-vs-available state derived ONLY from the existing
// check_model_downloaded invoke (called once per catalog entry \u2014 no new command).
async function renderModelCards() {
  const rows = await Promise.all(
    MODEL_CATALOG.map(async (m) => ({
      ...m,
      // Caught per model, never fatal. The catalog (name, size, speed) is
      // local and must still render if the backend check fails; previously a
      // single rejection failed the whole Promise.all, the innerHTML
      // assignment below never ran, and the Voice Model card rendered as an
      // empty box with nothing explaining why. null means "we could not
      // determine this", which is not the same as "not downloaded".
      downloaded: await invoke<boolean>("check_model_downloaded", { modelSize: m.id }).catch(
        () => null
      ),
    }))
  );

  modelList.innerHTML = rows
    .map((m) => {
      const isActive = m.id === currentSettings.whisperModel;
      const statusHtml =
        m.downloaded === null
          ? // Status unreadable: say so rather than offering an action whose
            // outcome we cannot predict. A missing affordance beats a wrong one.
            `<span class="gate-badge">Status unavailable</span>`
          : m.downloaded
        ? `<span class="gate-badge downloaded">Downloaded</span><button type="button" class="btn-primary" data-use="${m.id}" ${isActive ? "disabled" : ""}>${isActive ? "Active" : "Use"}</button>`
        : `<button type="button" class="btn-primary" data-download="${m.id}" ${activeDownloadId ? "disabled" : ""}>Download</button>`;
      return `
      <div class="model-card-row" data-model="${m.id}">
        <div class="model-info">
          <span class="model-name">${m.label}${isActive ? " (active)" : ""}</span>
          <span class="model-meta"><span>${m.sizeLabel}</span><span>${m.speedLabel}</span></span>
        </div>
        <div class="model-status">${statusHtml}</div>
      </div>`;
    })
    .join("");

  const activeRow = rows.find((r) => r.id === currentSettings.whisperModel);
  // Three states, not two: downloaded, definitely not downloaded, and unknown.
  // Reporting "(not downloaded)" when the check failed would assert something
  // we did not actually learn.
  const suffix =
    activeRow?.downloaded === null || activeRow === undefined
      ? " (status unknown)"
      : activeRow.downloaded
        ? ""
        : " (not downloaded)";
  // Engine-aware: on cloud the local Whisper model is not what is transcribing,
  // so reporting "medium (not downloaded)" there is both irrelevant and alarming
  // - it reads as broken when nothing is wrong. Name what is actually running.
  footerModel.textContent =
    currentSettings.engine === "cloud"
      ? `Model: Groq ${currentSettings.cloudModel}`
      : `Model: ${currentSettings.whisperModel}${suffix}`;
}

// Transcript text is arbitrary user content injected into innerHTML below —
// every interpolated field must go through this first or it's an XSS hole.
function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

// Cached full result of the last list_history call, so the search box can
// filter client-side without a round trip on every keystroke.
let historyItems: HistoryItem[] = [];

function wordCount(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

/// Renders the History tab list from list_history. Re-run on every entry
/// into the History tab (activateTab) and after any mutating action
/// (copy/delete/clear) so the list never goes stale.
async function renderHistory() {
  historyItems = await invoke<HistoryItem[]>("list_history");
  renderHistoryStats();
  renderHistoryList();
  const hasItems = historyItems.length > 0;
  historyClearBtn.disabled = !hasItems;
  privacyClearHistoryBtn.disabled = !hasItems;
}

// Stats strip: total dictations, total words, and a rough "typed vs spoken"
// time saved estimate. Hidden entirely when there is nothing to summarize.
function renderHistoryStats() {
  if (historyItems.length === 0) {
    historyStats.classList.add("hidden");
    return;
  }
  const totalWords = historyItems.reduce((sum, item) => sum + wordCount(item.text), 0);
  const minutesSaved = Math.max(0, Math.round(totalWords * (1 / 40 - 1 / 150)));
  historyStats.classList.remove("hidden");
  historyStatTotal.textContent = String(historyItems.length);
  historyStatWords.textContent = String(totalWords);
  historyStatSaved.textContent = `~${minutesSaved} min`;
}

// Live, case-insensitive, client-only filter over the cached items — applied
// on every keystroke in #history-search, nothing persisted.
function renderHistoryList() {
  const query = historySearch.value.trim().toLowerCase();
  const filtered = query
    ? historyItems.filter((item) => item.text.toLowerCase().includes(query))
    : historyItems;

  if (historyItems.length === 0) {
    historyEmpty.hidden = false;
    historyEmptyTitle.textContent =
      currentSettings.historyEnabled === false
        ? "History saving is off — turn it on in Privacy to keep transcripts."
        : "No transcripts yet — dictate something and it'll show up here.";
    historyList.innerHTML = "";
    historyCount.textContent = "";
    return;
  }

  historyCount.textContent = `${historyItems.length} saved`;

  if (filtered.length === 0) {
    historyEmpty.hidden = false;
    historyEmptyTitle.textContent = "No transcripts match your search.";
    historyList.innerHTML = "";
    return;
  }

  historyEmpty.hidden = true;
  historyList.innerHTML = filtered
    .map((item) => {
      const when = new Date(item.tsMs).toLocaleString();
      const words = wordCount(item.text);
      const metaParts = [
        escapeHtml(when),
        escapeHtml(item.engine),
        escapeHtml(item.model),
        `${words} word${words === 1 ? "" : "s"}`,
      ];
      if (item.durationMs != null) {
        metaParts.push(`${(item.durationMs / 1000).toFixed(1)}s`);
      }
      return `
      <div class="history-row" data-history-id="${escapeHtml(item.id)}">
        <div class="history-text">${escapeHtml(item.text)}</div>
        <div class="history-meta">
          <span>${metaParts.join(" &middot; ")}</span>
        </div>
        <div class="history-actions">
          <button type="button" class="btn-secondary" data-copy="${escapeHtml(item.id)}">Copy</button>
          <button type="button" class="btn-secondary" data-delete="${escapeHtml(item.id)}">Delete</button>
        </div>
      </div>`;
    })
    .join("");
}

historySearch.addEventListener("input", () => renderHistoryList());

/// Window-level failure message. Kept to one line of plain language: the
/// detail belongs in the log, not on screen.
function showAppNotice(message: string, kind: "error" | "warn" = "error") {
  appNotice.textContent = message;
  appNotice.dataset.kind = kind;
  appNotice.hidden = false;
}

function clearAppNotice() {
  appNotice.hidden = true;
  appNotice.textContent = "";
  delete appNotice.dataset.kind;
}

/// Brief forest-neon-green "Saved" confirmation, shown on any settings change so
/// a dropdown selection gives the same visible confirmation a button does. Uses
/// its own element so it never clobbers a persistent advisory in appNotice.
let savedToastTimer: number | undefined;
function flashSaved(msg = "✓ Saved") {
  savedToast.textContent = msg;
  savedToast.classList.add("show");
  if (savedToastTimer) window.clearTimeout(savedToastTimer);
  savedToastTimer = window.setTimeout(() => savedToast.classList.remove("show"), 1500);
}

/// Keep the sidebar footer's "Model: …" line in sync the moment a dropdown or
/// engine toggle changes it. It was only refreshed by the heavier model-card
/// render before, so cloud-model picks never updated the footer.
function refreshFooterModel() {
  footerModel.textContent =
    currentSettings.engine === "cloud"
      ? `Model: Groq ${currentSettings.cloudModel}`
      : `Model: ${currentSettings.whisperModel}`;
}

/// Returns whether the save actually reached disk, so callers that need to
/// show their own "it was stored" confirmation (the Groq key Save button)
/// never claim success the invoke did not report.
async function saveSettings(): Promise<boolean> {
  currentSettings.microphone = micSelect.value;
  currentSettings.groqApiKey = groqKey.value;
  try {
    await invoke("save_settings", { settings: currentSettings });
    // Clear only a previous FAILURE. Advisories describe the settings as they
    // now are, so a successful save is exactly when they should still stand.
    if (appNotice.dataset.kind === "error") clearAppNotice();
    refreshAdvisories();
    return true;
  } catch (err) {
    // The control has already moved on screen, so staying silent here would
    // leave the user believing a setting persisted when it did not.
    console.error("[Demod] save_settings failed:", err);
    showAppNotice("Could not save your settings, so that last change was not kept.");
    return false;
  }
}

/// Sets the small badge next to a settings control, or hides it. `kind`
/// selects the CSS tint (status-saved/status-error/etc, see style.css);
/// omit it to hide.
function setStatusBadge(el: HTMLElement, text: string, kind?: string) {
  el.className = "gate-badge" + (kind ? ` ${kind}` : "");
  el.textContent = text;
  el.hidden = false;
}

function hideStatusBadge(el: HTMLElement) {
  el.hidden = true;
  el.textContent = "";
}

// Event listeners
engineLocal.addEventListener("click", () => {
  setEngine("local");
  saveSettings();
  refreshFooterModel();
  flashSaved();
});

engineCloud.addEventListener("click", () => {
  setEngine("cloud");
  saveSettings();
  refreshFooterModel();
  flashSaved();
});

micSelect.addEventListener("change", () => {
  saveSettings();
  flashSaved();
});

// Event delegation: one listener for every model card's Download/Use button.
modelList.addEventListener("click", async (e) => {
  const target = (e.target as HTMLElement).closest("button");
  if (!target) return;
  const useId = target.getAttribute("data-use");
  const downloadId = target.getAttribute("data-download");

  if (useId) {
    currentSettings.whisperModel = useId;
    await saveSettings();
    await renderModelCards();
    return;
  }

  if (downloadId) {
    activeDownloadId = downloadId;
    await renderModelCards(); // disables every row's Download button while one runs
    downloadProgress.classList.remove("hidden");
    progressFill.style.width = "0%";
    try {
      await invoke("download_model", { modelSize: downloadId });
    } catch (err) {
      console.error("Download failed:", err);
    }
    activeDownloadId = null;
    downloadProgress.classList.add("hidden");
    await renderModelCards();
  }
});

groqKey.addEventListener("change", () => saveSettings());
// Live, not just on commit: the warning should disappear as soon as a key is
// typed, rather than persisting until the field loses focus.
groqKey.addEventListener("input", () => {
  currentSettings.groqApiKey = groqKey.value;
  refreshAdvisories();
  // The field changed since the last Save/Test, so any prior "Saved" /
  // connection-test badge no longer describes what's in the box now.
  hideStatusBadge(groqKeyStatus);
  hideStatusBadge(groqTestStatus);
});

// Explicit Save button: the field previously saved silently on blur with no
// on-screen confirmation, so a bad key was only discovered after a failed
// dictation. This makes "it was stored" (or wasn't) unambiguous.
groqKeySaveBtn.addEventListener("click", async () => {
  groqKeySaveBtn.disabled = true;
  setStatusBadge(groqKeyStatus, "Saving…", "status-pending");
  const ok = await saveSettings();
  groqKeySaveBtn.disabled = false;
  if (ok) {
    setStatusBadge(groqKeyStatus, "Saved", "status-saved");
  } else {
    // saveSettings() already surfaced the failure via showAppNotice; this
    // badge just must not claim "Saved" when it wasn't.
    setStatusBadge(groqKeyStatus, "Not saved", "status-error");
  }
});

// "Test connection": contacts Groq for real (GET /v1/models, no audio) and
// reports one of three honest, distinct outcomes. A timeout/offline result
// must never be reported as "key rejected" - they need completely different
// fixes from the user.
groqTestBtn.addEventListener("click", async () => {
  groqTestBtn.disabled = true;
  setStatusBadge(groqTestStatus, "Testing…", "status-pending");
  // test_groq_connection reads the key from the SAVED settings on the Rust
  // side, not straight off this input - save first so a key typed but not
  // yet blurred/saved is actually the one that gets tested, not a stale one.
  await saveSettings();
  try {
    const result = await invoke<string>("test_groq_connection");
    if (result === "empty") {
      setStatusBadge(groqTestStatus, "No key entered", "status-error");
    } else if (result === "valid") {
      setStatusBadge(groqTestStatus, "Key accepted", "status-valid");
    } else if (result === "rejected") {
      setStatusBadge(groqTestStatus, "Key rejected by Groq", "status-rejected");
    } else if (result.startsWith("unreachable:")) {
      setStatusBadge(groqTestStatus, "Could not reach Groq", "status-unreachable");
    } else {
      setStatusBadge(groqTestStatus, "Unknown result", "status-error");
    }
  } catch (err) {
    console.error("[Demod] test_groq_connection failed:", err);
    setStatusBadge(groqTestStatus, "Could not run the test", "status-error");
    showAppNotice("Could not test the Groq connection.", "error");
  } finally {
    groqTestBtn.disabled = false;
  }
});

modeAuto.addEventListener("click", () => {
  setRecordingMode("auto");
  saveSettings();
});

modeToggle.addEventListener("click", () => {
  setRecordingMode("toggle");
  saveSettings();
});

modePtt.addEventListener("click", () => {
  setRecordingMode("push-to-talk");
  saveSettings();
});

soundsSwitch.addEventListener("click", () => {
  setInteractionSounds(!(currentSettings.interactionSounds ?? true));
  saveSettings();
  flashSaved();
});

overlaySwitch.addEventListener("click", () => {
  setOverlayVisible(!(currentSettings.overlayVisible ?? true));
  saveSettings();
  flashSaved();
});

removeFillersSwitch.addEventListener("click", () => {
  setRemoveFillers(!(currentSettings.removeFillers ?? true));
  saveSettings();
  flashSaved();
});

autoEditsSwitch.addEventListener("click", () => {
  setAutoEdits(!(currentSettings.autoEdits ?? true));
  saveSettings();
  flashSaved();
});

// Launch at startup: not part of currentSettings/save_settings — each click
// calls the real enable/disable command and the switch only ever shows what
// that command actually achieved. On failure, re-query the true state rather
// than assume the prior one still holds (the enable/disable command may have
// partially applied before failing, e.g. deleted the .lnk but failed the
// registry write).
autostartSwitch.addEventListener("click", async () => {
  const wasEnabled = autostartSwitch.getAttribute("aria-checked") === "true";
  const nextEnabled = !wasEnabled;
  autostartSwitch.setAttribute("aria-disabled", "true");
  try {
    await invoke(nextEnabled ? "autostart_enable" : "autostart_disable");
    setAutostartSwitch(nextEnabled);
  } catch (err) {
    console.error("[Demod] autostart toggle failed:", err);
    showAppNotice(
      nextEnabled
        ? `Could not enable launch at startup: ${err}`
        : `Could not disable launch at startup: ${err}`
    );
    try {
      const actual = await invoke<boolean>("autostart_get_state");
      setAutostartSwitch(actual);
    } catch {
      // Could not even re-read the true state — leave the switch as it was
      // rather than assert either state we didn't verify.
      setAutostartSwitch(wasEnabled);
    }
  } finally {
    autostartSwitch.removeAttribute("aria-disabled");
  }
});

languageSelect.addEventListener("change", () => {
  setLanguage(languageSelect.value);
  saveSettings();
  flashSaved();
});

cloudModelSelect.addEventListener("change", () => {
  setCloudModel(cloudModelSelect.value);
  saveSettings();
  refreshFooterModel();
  flashSaved();
});

// Committed on blur (matches groqKey's pattern) rather than every keystroke —
// a multi-line vocabulary list shouldn't trigger a save per character.
dictionaryTextarea.addEventListener("change", () => {
  currentSettings.dictionary = dictionaryTextarea.value;
  saveSettings();
});

voiceSensitivitySlider.addEventListener("input", () => {
  const v = parseFloat(voiceSensitivitySlider.value);
  currentSettings.voiceSensitivity = v;
  voiceSensitivityValue.textContent = v.toFixed(3);
});
voiceSensitivitySlider.addEventListener("change", () => { saveSettings(); });

whisperThresholdSlider.addEventListener("input", () => {
  const v = parseFloat(whisperThresholdSlider.value);
  currentSettings.whisperThreshold = v;
  whisperThresholdValue.textContent = v.toFixed(2);
});
whisperThresholdSlider.addEventListener("change", () => { saveSettings(); });

// Single delegated listener for every toggle the generic renderToggleRows()
// produces (currently TEXT_TOGGLE_CATALOG, but works for any future catalog
// rendered into a container this way) — writes the cached setting AND saves,
// so a new toggle can never be added without both halves of footgun #13.
textToggleList.addEventListener("click", (e) => {
  const btn = (e.target as HTMLElement).closest("[data-toggle-key]") as HTMLElement | null;
  if (!btn) return;
  const key = btn.getAttribute("data-toggle-key") as ToggleCatalogEntry["key"];
  const next = !(currentSettings[key] ?? false);
  currentSettings[key] = next;
  btn.classList.toggle("active", next);
  btn.setAttribute("aria-checked", String(next));
  saveSettings();
});

// Rows waiting on a second Delete click to confirm — cleared on click-away
// re-render (renderHistoryList rebuilds fresh buttons, so a stale armed
// state can never linger past the next render).
const armedDeleteTimers = new Map<string, ReturnType<typeof setTimeout>>();

// Delegated listener for every history row's Copy/Delete button.
historyList.addEventListener("click", async (e) => {
  const target = (e.target as HTMLElement).closest("button");
  if (!target) return;
  const copyId = target.getAttribute("data-copy");
  const deleteId = target.getAttribute("data-delete");

  if (copyId) {
    const btn = target as HTMLButtonElement;
    const original = btn.textContent ?? "Copy";
    try {
      await invoke("copy_history_item", { id: copyId });
      btn.textContent = "Copied";
      btn.disabled = true;
      setTimeout(() => {
        btn.textContent = original;
        btn.disabled = false;
      }, 1500);
    } catch (err) {
      console.error("[Demod] copy_history_item failed:", err);
      showAppNotice("Could not copy that transcript.", "error");
    }
    return;
  }

  if (deleteId) {
    const btn = target as HTMLButtonElement;
    const original = btn.textContent ?? "Delete";
    if (!armedDeleteTimers.has(deleteId)) {
      btn.textContent = "Confirm?";
      const timer = setTimeout(() => {
        armedDeleteTimers.delete(deleteId);
        btn.textContent = original;
      }, 3000);
      armedDeleteTimers.set(deleteId, timer);
      return;
    }

    const timer = armedDeleteTimers.get(deleteId);
    if (timer) clearTimeout(timer);
    armedDeleteTimers.delete(deleteId);

    try {
      await invoke("delete_history_item", { id: deleteId });
    } catch (err) {
      console.error("[Demod] delete_history_item failed:", err);
      showAppNotice("Could not delete that transcript.", "error");
    }
    await renderHistory();
  }
});

// Two-click inline confirm for a destructive, no-undo action — no modal (the
// user hates nags). First click arms a 3s window and relabels the button;
// a second click within that window actually clears; letting the window
// lapse reverts the label with no action taken. Shared by both Clear-all
// entry points (History tab card header, Privacy tab).
function wireClearHistoryButton(btn: HTMLButtonElement) {
  const originalLabel = btn.textContent ?? "Clear all";
  let armed = false;
  let timer: ReturnType<typeof setTimeout> | null = null;

  btn.addEventListener("click", async () => {
    if (!armed) {
      armed = true;
      btn.textContent = "Click again to confirm";
      timer = setTimeout(() => {
        armed = false;
        btn.textContent = originalLabel;
        timer = null;
      }, 3000);
      return;
    }

    if (timer) clearTimeout(timer);
    armed = false;
    btn.textContent = originalLabel;
    try {
      await invoke("clear_history");
    } catch (err) {
      console.error("[Demod] clear_history failed:", err);
      showAppNotice("Could not clear history.", "error");
    }
    await renderHistory();
  });
}

wireClearHistoryButton(historyClearBtn);
wireClearHistoryButton(privacyClearHistoryBtn);

historyEnabledSwitch.addEventListener("click", () => {
  const next = !(currentSettings.historyEnabled ?? true);
  currentSettings.historyEnabled = next;
  historyEnabledSwitch.classList.toggle("active", next);
  historyEnabledSwitch.setAttribute("aria-checked", String(next));
  saveSettings();
  flashSaved();
});

historyRetentionSelect.addEventListener("change", () => {
  currentSettings.historyRetentionDays = parseInt(historyRetentionSelect.value, 10);
  saveSettings();
  flashSaved();
});

// ── Sound preview (diagnostic: "the sounds dont play") ────────────────
// Works regardless of the interaction-sounds switch above - that is the
// whole point. A rejection from Windows is a real error and must surface,
// not be swallowed; success is NOT claimed as "you heard it" (PlaySound
// returning true only means the OS accepted the buffer, not that it was
// audible), so there is deliberately no success message here.
document.querySelectorAll<HTMLButtonElement>("[data-preview]").forEach((btn) => {
  btn.addEventListener("click", async () => {
    const which = btn.getAttribute("data-preview")!;
    try {
      await invoke("preview_sound", { which });
      if (appNotice.dataset.kind === "error") clearAppNotice();
    } catch (err) {
      console.error("[Demod] preview_sound failed:", err);
      showAppNotice(String(err), "error");
    }
  });
});

// ── Hotkey capture ──────────────────────────────────────────────────
// Replaces the old read-only kbd: click the control, press a combination,
// it commits on a valid combo or Escape cancels (keeping the existing
// binding). Persistence + the actual re-registration (with rollback on
// failure) happens in Rust via set_hotkey - this only builds the
// accelerator string and reports what the backend says.
const HOTKEY_MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta", "AltGraph", "OS"]);

// Named keys the Tauri accelerator parser accepts, keyed by KeyboardEvent.key.
const HOTKEY_NAMED_KEYS: Record<string, string> = {
  " ": "Space",
  Escape: "Escape",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Enter: "Return",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Insert: "Insert",
  CapsLock: "CapsLock",
};

function hotkeyPressedModifiers(e: KeyboardEvent): string[] {
  const mods: string[] = [];
  if (e.ctrlKey || e.metaKey) mods.push("CmdOrCtrl");
  if (e.shiftKey) mods.push("Shift");
  if (e.altKey) mods.push("Alt");
  return mods;
}

/// Maps a non-modifier key press to its accelerator token, or null if it
/// isn't one the parser accepts as a final key.
function hotkeyKeyToken(e: KeyboardEvent): string | null {
  const key = e.key;
  if (key.length === 1) return key.toUpperCase();
  if (HOTKEY_NAMED_KEYS[key]) return HOTKEY_NAMED_KEYS[key];
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) return key;
  return null;
}

let hotkeyCaptureKeydownHandler: ((e: KeyboardEvent) => void) | null = null;

function stopHotkeyCapture() {
  hotkeyCapture.classList.remove("capturing");
  if (hotkeyCaptureKeydownHandler) {
    window.removeEventListener("keydown", hotkeyCaptureKeydownHandler, true);
    hotkeyCaptureKeydownHandler = null;
  }
  window.removeEventListener("pointerdown", onWindowPointerDownOutsideCapture, true);
  window.removeEventListener("blur", stopHotkeyCapture);
}

// Teardown when focus/attention leaves the capture control without a commit
// or Escape (e.g. clicking elsewhere in the settings window). Without this,
// the capture-phase window keydown listener stays installed indefinitely,
// and its preventDefault()/stopPropagation() swallows every keystroke in the
// window — including typing in the Groq API key field or the dictionary
// textarea — with no visible sign anything is wrong.
function onWindowPointerDownOutsideCapture(e: PointerEvent) {
  if (e.target instanceof Node && hotkeyCapture.contains(e.target)) return;
  stopHotkeyCapture();
  hotkeyMessage.textContent = "";
  hotkeyText.textContent = formatHotkey(currentSettings.hotkey);
}

function startHotkeyCapture() {
  if (hotkeyCaptureKeydownHandler) return; // already capturing
  hotkeyCapture.classList.add("capturing");
  hotkeyMessage.textContent = "Press a key combination… (Esc cancels)";
  window.addEventListener("pointerdown", onWindowPointerDownOutsideCapture, true);
  window.addEventListener("blur", stopHotkeyCapture);

  const onKeyDown = async (e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      stopHotkeyCapture();
      hotkeyMessage.textContent = "";
      hotkeyText.textContent = formatHotkey(currentSettings.hotkey);
      return;
    }

    const mods = hotkeyPressedModifiers(e);

    if (HOTKEY_MODIFIER_KEYS.has(e.key)) {
      // Live preview while only modifiers are held - not yet a valid combo.
      hotkeyText.textContent = mods.length ? `${formatHotkey(mods.join("+"))}+…` : "…";
      return;
    }

    if (mods.length === 0) {
      hotkeyMessage.textContent =
        "Add a modifier (Ctrl, Shift, or Alt) — a bare key would intercept ordinary typing.";
      return;
    }

    const keyToken = hotkeyKeyToken(e);
    if (!keyToken) {
      hotkeyMessage.textContent = "That key can't be used in a hotkey. Try another.";
      return;
    }

    const accelerator = [...mods, keyToken].join("+");
    // Live preview of the committed-looking combo before the backend confirms it.
    hotkeyText.textContent = formatHotkey(accelerator);
    stopHotkeyCapture();
    hotkeyMessage.textContent = "Saving…";

    try {
      await invoke("set_hotkey", { hotkey: accelerator });
      currentSettings.hotkey = accelerator;
      hotkeyText.textContent = formatHotkey(accelerator);
      footerHotkey.textContent = formatHotkey(accelerator);
      hotkeyMessage.textContent = "";
    } catch (err) {
      // The backend already rolled back to the previous accelerator, so
      // reflect that - never leave the display showing a hotkey that isn't
      // actually the one registered.
      console.error("[Demod] set_hotkey failed:", err);
      hotkeyText.textContent = formatHotkey(currentSettings.hotkey);
      hotkeyMessage.textContent = String(err);
    }
  };

  hotkeyCaptureKeydownHandler = onKeyDown;
  window.addEventListener("keydown", onKeyDown, true);
}

hotkeyCapture.addEventListener("click", startHotkeyCapture);

// Render one recording state into the header indicator and the sidebar footer
// health row. Shared by the initial read and the live event so the two can
// never drift apart.
function renderRecordingState(state: string) {
  statusDot.className = "";
  let label = "Ready";
  if (state === "Recording") {
    statusDot.classList.add("recording");
    label = "Recording...";
  } else if (state === "Transcribing") {
    statusDot.classList.add("transcribing");
    label = "Transcribing...";
  } else {
    statusDot.classList.add("ready");
    label = "Ready";
  }
  statusText.textContent = label;
  // Sidebar footer health row (visible from every section) mirrors the same state.
  footerSidecar.innerHTML = `<span class="dot"></span>${label}`;
  footerSidecar.querySelector(".dot")!.className = `dot ${statusDot.className}`;
}

// Read the CURRENT state when the window opens. Without this the indicator sat
// on a hardcoded "Ready" until the next transition, so opening Settings while
// a dictation was in flight reported a status that was simply false. The
// get_recording_state command already existed and was registered; nothing had
// ever called it.
invoke<string>("get_recording_state")
  .then(renderRecordingState)
  .catch(() => {
    // Never assert "Ready" we did not read.
    statusText.textContent = "Unknown";
  });

// Launch-at-startup switch: read the REAL registry/.lnk state. Never default
// this to on or off — a failed read leaves the switch in its last-known
// (initial "off") position and surfaces the error, rather than guessing.
invoke<boolean>("autostart_get_state")
  .then(setAutostartSwitch)
  .catch((err) => {
    console.error("[Demod] autostart_get_state failed:", err);
    showAppNotice("Could not read the launch-at-startup state, so the switch above may not reflect reality.");
  });

// Listen for recording state changes
listen<string>("recording-state", (event) => renderRecordingState(event.payload));

// Listen for download progress
listen<DownloadProgress>("download-progress", (event) => {
  const { percent } = event.payload;
  progressFill.style.width = `${percent}%`;
});

// Initialize
// A rejection here used to abort mid-population, leaving a half-filled window
// (blank mic list, unrendered model cards) with nothing saying why.
loadSettings().catch((err) => {
  console.error("[Demod] loadSettings failed:", err);
  showAppNotice("Could not load your settings. Values shown may be incomplete or stale.");
});
