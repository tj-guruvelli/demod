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
  interactionSounds?: boolean;
  overlayVisible?: boolean;
  // Already deserialized server-side (pill_hittest.rs) — read-only display only,
  // never written back from the frontend. See Overlay → Dock position.
  overlayX?: number | null;
  overlayY?: number | null;
}

interface MicDevice {
  name: string;
  is_default: boolean;
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
const modeAuto = document.getElementById("mode-auto")!;
const modeToggle = document.getElementById("mode-toggle")!;
const modePtt = document.getElementById("mode-ptt")!;
const hotkeyText = document.getElementById("hotkey-text")!;
const soundsSwitch = document.getElementById("sounds-switch")!;
const overlaySwitch = document.getElementById("overlay-switch")!;
const dockPositionText = document.getElementById("dock-position-text")!;
const footerSidecar = document.getElementById("footer-sidecar")!;
const footerModel = document.getElementById("footer-model")!;
const footerHotkey = document.getElementById("footer-hotkey")!;

// Section navigation — role=tablist/tab/tabpanel pattern (chosen over plain
// focusable buttons because the sidebar genuinely IS a section switcher with
// exactly one visible panel at a time — the ARIA tab pattern maps 1:1 onto
// that model). 150ms opacity/transform crossfade (matches overlay.html's own
// ease-in/ease-out grammar) instead of an instant display swap.
const navItems = Array.from(document.querySelectorAll<HTMLElement>(".nav-item"));

function activateTab(item: HTMLElement, { focus = true }: { focus?: boolean } = {}) {
  const target = item.getAttribute("data-section");
  const current = document.querySelector(".content-section.active") as HTMLElement | null;
  const next = document.getElementById(`section-${target}`) as HTMLElement | null;
  if (!next || next === current) {
    if (focus) item.focus();
    return;
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
    current.addEventListener(
      "transitionend",
      () => {
        current.classList.remove("leaving");
        current.style.display = "none";
      },
      { once: true }
    );
  }
  next.style.display = "block";
  // rAF so the browser commits `display:block` before the class flips —
  // otherwise the opacity/transform transition has no paint to animate from.
  requestAnimationFrame(() => next.classList.add("active"));
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

// Static catalog matching exactly the two sizes settings.rs / download_model
// already support (US-014/015 §4.3 — do not list a model id Rust can't download).
const MODEL_CATALOG = [
  { id: "small", label: "Small", sizeLabel: "~466 MB", speedLabel: "Fast" },
  { id: "medium", label: "Medium", sizeLabel: "~1.5 GB", speedLabel: "Balanced" },
] as const;

// Only one download can run at a time — tracked so every row's Download button
// disables while any download is in flight (shared progress bar under the list).
let activeDownloadId: string | null = null;

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
    showAppNotice("No microphone detected, so Typr cannot record. Connect one and reopen Settings.");
  } else {
    micSelect.disabled = false;
    mics.forEach((mic) => {
      const option = document.createElement("option");
      option.value = mic.name;
      option.textContent = mic.name + (mic.is_default ? " (default)" : "");
      micSelect.appendChild(option);
    });
    micSelect.value = currentSettings.microphone;
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
  warnIfNoRecordingFeedback();
}

function setOverlayVisible(enabled: boolean) {
  currentSettings.overlayVisible = enabled;
  overlaySwitch.classList.toggle("active", enabled);
  overlaySwitch.setAttribute("aria-checked", String(enabled));
  warnIfNoRecordingFeedback();
}

/// The overlay and the start/stop sounds are the only two signals that the
/// microphone is live. They are independent settings, so both can be off - and
/// then pressing the hotkey opens the mic with no indication whatsoever. For an
/// always-listening dictation tool that is a trust problem, not a preference,
/// so say it plainly rather than silently allowing a silent hot mic.
/// Not blocked: it is the user's machine and their call. Just never a surprise.
function warnIfNoRecordingFeedback() {
  const silent =
    currentSettings.overlayVisible === false && currentSettings.interactionSounds === false;
  if (silent) {
    showAppNotice(
      "With both the overlay and interaction sounds off, nothing will show that Typr is recording.",
      "warn"
    );
  } else if (appNotice.dataset.kind === "warn") {
    // Only clear our own warning, never an error someone else surfaced.
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
  footerModel.textContent = `Model: ${currentSettings.whisperModel}${suffix}`;
}

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

async function saveSettings() {
  currentSettings.microphone = micSelect.value;
  currentSettings.groqApiKey = groqKey.value;
  try {
    await invoke("save_settings", { settings: currentSettings });
    clearAppNotice();
  } catch (err) {
    // The control has already moved on screen, so staying silent here would
    // leave the user believing a setting persisted when it did not.
    console.error("[Typr] save_settings failed:", err);
    showAppNotice("Could not save your settings, so that last change was not kept.");
  }
}

// Event listeners
engineLocal.addEventListener("click", () => {
  setEngine("local");
  saveSettings();
});

engineCloud.addEventListener("click", () => {
  setEngine("cloud");
  saveSettings();
});

micSelect.addEventListener("change", () => saveSettings());

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
});

overlaySwitch.addEventListener("click", () => {
  setOverlayVisible(!(currentSettings.overlayVisible ?? true));
  saveSettings();
});

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
  console.error("[Typr] loadSettings failed:", err);
  showAppNotice("Could not load your settings. Values shown may be incomplete or stale.");
});
