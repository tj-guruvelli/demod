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
const statusDot = document.getElementById("status-dot")!;
const statusText = document.getElementById("status-text")!;
const micSelect = document.getElementById("mic-select") as HTMLSelectElement;
const engineLocal = document.getElementById("engine-local")!;
const engineCloud = document.getElementById("engine-cloud")!;
const localSettings = document.getElementById("local-settings")!;
const cloudSettings = document.getElementById("cloud-settings")!;
const modelSelect = document.getElementById("model-select") as HTMLSelectElement;
const downloadBtn = document.getElementById("download-btn")!;
const downloadProgress = document.getElementById("download-progress")!;
const progressFill = document.getElementById("progress-fill")!;
const groqKey = document.getElementById("groq-key") as HTMLInputElement;
const modeAuto = document.getElementById("mode-auto")!;
const modeToggle = document.getElementById("mode-toggle")!;
const modePtt = document.getElementById("mode-ptt")!;
const hotkeyText = document.getElementById("hotkey-text")!;
const soundsSwitch = document.getElementById("sounds-switch")!;
const overlayOn = document.getElementById("overlay-on")!;
const overlayOff = document.getElementById("overlay-off")!;
const dockPositionText = document.getElementById("dock-position-text")!;
const footerSidecar = document.getElementById("footer-sidecar")!;
const footerModel = document.getElementById("footer-model")!;
const footerHotkey = document.getElementById("footer-hotkey")!;

// Section navigation — 150ms opacity/transform crossfade (matches overlay.html's
// own ease-in/ease-out grammar) instead of an instant display swap.
const navItems = document.querySelectorAll(".nav-item");

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
  });
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
  mics.forEach((mic) => {
    const option = document.createElement("option");
    option.value = mic.name;
    option.textContent = mic.name + (mic.is_default ? " (default)" : "");
    micSelect.appendChild(option);
  });
  micSelect.value = currentSettings.microphone;

  // Engine
  setEngine(currentSettings.engine);

  // Model
  modelSelect.value = currentSettings.whisperModel;
  await checkModelStatus();

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
      ? `Custom · (${overlayX}, ${overlayY})`
      : "Default · bottom-center";
}

function setEngine(engine: string) {
  currentSettings.engine = engine;
  engineLocal.classList.toggle("active", engine === "local");
  engineCloud.classList.toggle("active", engine === "cloud");
  localSettings.classList.toggle("hidden", engine !== "local");
  cloudSettings.classList.toggle("hidden", engine !== "cloud");
}

function setRecordingMode(mode: string) {
  currentSettings.recordingMode = mode;
  modeAuto.classList.toggle("active", mode === "auto");
  modeToggle.classList.toggle("active", mode === "toggle");
  modePtt.classList.toggle("active", mode === "push-to-talk");
}

function setInteractionSounds(enabled: boolean) {
  currentSettings.interactionSounds = enabled;
  soundsSwitch.classList.toggle("active", enabled);
  soundsSwitch.setAttribute("aria-checked", String(enabled));
}

function setOverlayVisible(enabled: boolean) {
  currentSettings.overlayVisible = enabled;
  overlayOn.classList.toggle("active", enabled);
  overlayOff.classList.toggle("active", !enabled);
}

async function checkModelStatus() {
  const downloaded = await invoke<boolean>("check_model_downloaded", {
    modelSize: modelSelect.value,
  });
  downloadBtn.textContent = downloaded ? "\u2713" : "Download";
  (downloadBtn as HTMLButtonElement).disabled = downloaded;
  footerModel.textContent = `Model: ${modelSelect.value}${downloaded ? "" : " (not downloaded)"}`;
}

async function saveSettings() {
  currentSettings.microphone = micSelect.value;
  currentSettings.whisperModel = modelSelect.value;
  currentSettings.groqApiKey = groqKey.value;
  await invoke("save_settings", { settings: currentSettings });
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

modelSelect.addEventListener("change", async () => {
  await checkModelStatus();
  saveSettings();
});

downloadBtn.addEventListener("click", async () => {
  (downloadBtn as HTMLButtonElement).disabled = true;
  downloadProgress.classList.remove("hidden");
  progressFill.style.width = "0%";

  try {
    await invoke("download_model", { modelSize: modelSelect.value });
    downloadBtn.textContent = "\u2713";
  } catch (e) {
    downloadBtn.textContent = "Retry";
    (downloadBtn as HTMLButtonElement).disabled = false;
    console.error("Download failed:", e);
  }
  downloadProgress.classList.add("hidden");
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

overlayOn.addEventListener("click", () => {
  setOverlayVisible(true);
  saveSettings();
});

overlayOff.addEventListener("click", () => {
  setOverlayVisible(false);
  saveSettings();
});

// Listen for recording state changes
listen<string>("recording-state", (event) => {
  const state = event.payload;
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
  // Sidebar footer health row (visible from every section) mirrors the same event.
  footerSidecar.innerHTML = `<span class="dot"></span>${label}`;
  footerSidecar.querySelector(".dot")!.className = `dot ${statusDot.className}`;
});

// Listen for download progress
listen<DownloadProgress>("download-progress", (event) => {
  const { percent } = event.payload;
  progressFill.style.width = `${percent}%`;
});

// Initialize
loadSettings();
