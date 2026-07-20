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
  const reveal = () => next.classList.add("active");
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
// The cloud engine's model is fixed server-side in transcribe_groq.rs; named
// here only so the footer can say what is actually transcribing.
const GROQ_MODEL = "whisper-large-v3-turbo";

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
      "With both the overlay and interaction sounds off, nothing will show that Typr is recording.",
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
      ? `Model: Groq ${GROQ_MODEL}`
      : `Model: ${currentSettings.whisperModel}${suffix}`;
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
    console.error("[Typr] save_settings failed:", err);
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
    console.error("[Typr] test_groq_connection failed:", err);
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
});

overlaySwitch.addEventListener("click", () => {
  setOverlayVisible(!(currentSettings.overlayVisible ?? true));
  saveSettings();
});

languageSelect.addEventListener("change", () => {
  setLanguage(languageSelect.value);
  saveSettings();
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
      console.error("[Typr] preview_sound failed:", err);
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
}

function startHotkeyCapture() {
  if (hotkeyCaptureKeydownHandler) return; // already capturing
  hotkeyCapture.classList.add("capturing");
  hotkeyMessage.textContent = "Press a key combination… (Esc cancels)";

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
      console.error("[Typr] set_hotkey failed:", err);
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
