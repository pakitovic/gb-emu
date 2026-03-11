export const SCREEN_WIDTH = 160;
export const SCREEN_HEIGHT = 144;
export const MIN_SCREEN_SCALE = 1;
export const MAX_SCREEN_SCALE = 4;
export const DEFAULT_SCREEN_SCALE = 4;
export const CONTROL_SECTIONS = ["data", "system", "audio", "debug"];

export function frameDimensionsForEmulator(emulator) {
  const width =
    emulator && typeof emulator.screen_width === "function" ? Number(emulator.screen_width()) : NaN;
  const height =
    emulator && typeof emulator.screen_height === "function"
      ? Number(emulator.screen_height())
      : NaN;

  return {
    width: Number.isFinite(width) && width > 0 ? Math.trunc(width) : SCREEN_WIDTH,
    height: Number.isFinite(height) && height > 0 ? Math.trunc(height) : SCREEN_HEIGHT,
  };
}

export function canvasPromptForState({ hasRom = false } = {}) {
  if (!hasRom) {
    return {
      text: "Load ROM",
      clickable: true,
    };
  }

  return {
    text: null,
    clickable: false,
  };
}

export function normalizeControlSection(section) {
  if (typeof section !== "string") {
    return "data";
  }

  const normalized = section.toLowerCase().trim();
  return CONTROL_SECTIONS.includes(normalized) ? normalized : "data";
}

export function nextControlPanelState({
  currentSection = "data",
  isOpen = false,
  requestedSection,
} = {}) {
  const normalizedCurrent = normalizeControlSection(currentSection);
  const normalizedRequested = normalizeControlSection(requestedSection);
  if (isOpen && normalizedCurrent === normalizedRequested) {
    return {
      section: normalizedRequested,
      isOpen: false,
    };
  }

  return {
    section: normalizedRequested,
    isOpen: true,
  };
}

export function shouldCloseSettingsPanelOnPointerDown({
  isPanelOpen = false,
  isInsidePanel = false,
  isInsideToggle = false,
} = {}) {
  return Boolean(isPanelOpen) && !isInsidePanel && !isInsideToggle;
}

export function batteryPowerOnForState({ hasRom = false } = {}) {
  return Boolean(hasRom);
}

export function normalizeScreenScale({
  scale = DEFAULT_SCREEN_SCALE,
  minScale = MIN_SCREEN_SCALE,
  maxScale = MAX_SCREEN_SCALE,
} = {}) {
  const parsedScale = Number.parseInt(`${scale}`, 10);
  const candidate = Number.isFinite(parsedScale) ? parsedScale : DEFAULT_SCREEN_SCALE;

  return Math.max(minScale, Math.min(maxScale, Math.trunc(candidate)));
}

export function createUi(doc = document) {
  const stage = doc.querySelector(".stage");
  const contextPanel = doc.querySelector(".context-panel");
  const settingsToggleButton = doc.getElementById("settings-toggle");
  const batteryLed = doc.getElementById("battery-led");
  const romFileInput = doc.getElementById("rom-file");
  const bootRomFileInput = doc.getElementById("bootrom-file");
  const bootRomFileButton = doc.getElementById("bootrom-file-button");
  const savFileInput = doc.getElementById("sav-file");
  const savFileButton = doc.getElementById("sav-file-button");
  const savDownloadButton = doc.getElementById("sav-download");
  const rtcFileInput = doc.getElementById("rtc-file");
  const rtcFileButton = doc.getElementById("rtc-file-button");
  const rtcDownloadButton = doc.getElementById("rtc-download");
  const romResetButton = doc.getElementById("rom-reset");
  const modelSelect = doc.getElementById("model");
  const paletteSelect = doc.getElementById("palette");
  const paletteOverrideFileInput = doc.getElementById("palette-override-file");
  const paletteOverrideFileButton = doc.getElementById("palette-override-file-button");
  const paletteOverrideInfoLabel = doc.getElementById("palette-override-info");
  const paletteOverrideClearButton = doc.getElementById("palette-override-clear");
  const videoSizeSelect = doc.getElementById("video-size");
  const bootRomModelCheck = doc.getElementById("bootrom-model-check");
  const statusLabel = doc.getElementById("status");
  const persistenceInfoLabel = doc.getElementById("persistence-info");
  const bootRomInfoLabel = doc.getElementById("bootrom-info");
  const audioTelemetryLabel = doc.getElementById("audio-telemetry");
  const cartInfoPre = doc.getElementById("cart-info");
  const serialPre = doc.getElementById("serial");
  const testToneCheckbox = doc.getElementById("test-tone");
  const audioEnableButton = doc.getElementById("audio-enable");
  const audioResamplerSelect = doc.getElementById("audio-resampler");
  const debugRunStateLabel = doc.getElementById("debug-run-state");
  const controlNavDataButton = doc.getElementById("control-nav-data");
  const controlNavSystemButton = doc.getElementById("control-nav-system");
  const controlNavAudioButton = doc.getElementById("control-nav-audio");
  const controlNavDebugButton = doc.getElementById("control-nav-debug");
  const controlNavButtons = Array.from(doc.querySelectorAll("[data-control-section]"));
  const controlPanels = Array.from(doc.querySelectorAll("[data-control-panel]"));
  const canvas = doc.getElementById("screen");
  const ctx = canvas?.getContext("2d");

  if (!canvas || !ctx) {
    throw new Error("Web demo UI is missing the screen canvas or 2D context");
  }

  let screenWidth = SCREEN_WIDTH;
  let screenHeight = SCREEN_HEIGHT;
  let frameRgba = new Uint8ClampedArray(screenWidth * screenHeight * 4);
  let frameImage = new ImageData(frameRgba, screenWidth, screenHeight);
  let activeControlSection = "data";
  let isContextPanelOpen = false;

  function setScreenDimensions(width, height) {
    screenWidth = width;
    screenHeight = height;
    canvas.width = screenWidth;
    canvas.height = screenHeight;
    frameRgba = new Uint8ClampedArray(screenWidth * screenHeight * 4);
    frameImage = new ImageData(frameRgba, screenWidth, screenHeight);
  }

  function applyScreenScale(scale) {
    const clampedScale = normalizeScreenScale({ scale });
    canvas.style.width = `${screenWidth * clampedScale}px`;
    canvas.style.height = `${screenHeight * clampedScale}px`;
    doc.documentElement?.style?.setProperty("--video-scale", `${clampedScale}`);
    return clampedScale;
  }

  function updateScreenScale({ scale } = {}) {
    if (videoSizeSelect && scale !== undefined) {
      videoSizeSelect.value = `${normalizeScreenScale({ scale })}`;
    }
    const selectedScale = videoSizeSelect?.value ?? scale ?? DEFAULT_SCREEN_SCALE;
    return applyScreenScale(selectedScale);
  }

  function setStatus(message) {
    if (statusLabel) {
      statusLabel.textContent = message;
    }
  }

  function setBatteryPowerOn(hasRom = false) {
    if (!batteryLed) {
      return;
    }
    batteryLed.classList.toggle("is-on", batteryPowerOnForState({ hasRom }));
  }

  function setAudioTelemetryText(message) {
    if (audioTelemetryLabel) {
      audioTelemetryLabel.textContent = message;
    }
  }

  function setPersistenceInfoText(message) {
    if (persistenceInfoLabel) {
      persistenceInfoLabel.textContent = message;
    }
  }

  function setBootRomInfoText(message) {
    if (bootRomInfoLabel) {
      bootRomInfoLabel.textContent = message;
    }
  }

  function setPaletteOverrideInfoText(message) {
    if (paletteOverrideInfoLabel) {
      paletteOverrideInfoLabel.textContent = message;
    }
  }

  function setDebugRunStateText(message) {
    if (debugRunStateLabel) {
      debugRunStateLabel.textContent = message;
    }
  }

  function setBootRomModelCheck(isValid) {
    if (!bootRomModelCheck) {
      return;
    }

    bootRomModelCheck.classList.toggle("is-valid", Boolean(isValid));
    bootRomModelCheck.textContent = isValid ? "✓" : "✗";
  }

  function setPersistenceControlsEnabled({
    hasRom = false,
    batterySave = false,
    rtc = false,
  } = {}) {
    if (savFileInput) {
      savFileInput.disabled = !(hasRom && batterySave);
    }
    if (savFileButton) {
      savFileButton.classList.toggle("is-disabled", !(hasRom && batterySave));
    }
    if (savDownloadButton) {
      savDownloadButton.disabled = !(hasRom && batterySave);
    }

    if (rtcFileInput) {
      rtcFileInput.disabled = !(hasRom && rtc);
    }
    if (rtcFileButton) {
      rtcFileButton.classList.toggle("is-disabled", !(hasRom && rtc));
    }
    if (rtcDownloadButton) {
      rtcDownloadButton.disabled = !(hasRom && rtc);
    }
  }

  function setRomControlsEnabled({ hasRom = false, isRunning = false } = {}) {
    if (romResetButton) {
      romResetButton.disabled = !hasRom;
    }

    if (!hasRom) {
      setDebugRunStateText("State: idle");
      return;
    }
    setDebugRunStateText(isRunning ? "State: running" : "State: stopped");
  }

  function syncControlPanelVisibility() {
    if (contextPanel) {
      contextPanel.hidden = !isContextPanelOpen;
    }
    if (settingsToggleButton) {
      settingsToggleButton.setAttribute("aria-expanded", isContextPanelOpen ? "true" : "false");
    }

    for (const button of controlNavButtons) {
      const buttonSection = normalizeControlSection(button.dataset.controlSection);
      const isActive = isContextPanelOpen && buttonSection === activeControlSection;
      button.classList.toggle("is-active", isActive);
      button.setAttribute("aria-pressed", isActive ? "true" : "false");
    }

    for (const panel of controlPanels) {
      const panelSection = normalizeControlSection(panel.dataset.controlPanel);
      panel.hidden = !isContextPanelOpen || panelSection !== activeControlSection;
    }
  }

  function setActiveControlSection(section, { openPanel = true } = {}) {
    activeControlSection = normalizeControlSection(section);
    isContextPanelOpen = Boolean(openPanel);
    syncControlPanelVisibility();

    return {
      section: activeControlSection,
      isOpen: isContextPanelOpen,
    };
  }

  function toggleControlSection(section) {
    const nextState = nextControlPanelState({
      currentSection: activeControlSection,
      isOpen: isContextPanelOpen,
      requestedSection: section,
    });
    activeControlSection = nextState.section;
    isContextPanelOpen = nextState.isOpen;
    syncControlPanelVisibility();
    return nextState;
  }

  function setSettingsPanelOpen(open = false) {
    isContextPanelOpen = Boolean(open);
    syncControlPanelVisibility();
    return isContextPanelOpen;
  }

  function toggleSettingsPanel() {
    return setSettingsPanelOpen(!isContextPanelOpen);
  }

  function setPersistenceInfoFromEmulator(emulator) {
    if (!persistenceInfoLabel) {
      return;
    }
    if (!emulator) {
      setPersistenceInfoText("Info: no ROM loaded.");
      setPersistenceControlsEnabled();
      return;
    }

    const batterySave =
      typeof emulator.cartridge_has_battery_save === "function"
        ? Boolean(emulator.cartridge_has_battery_save())
        : false;
    const rtc =
      typeof emulator.cartridge_has_rtc_persistence === "function"
        ? Boolean(emulator.cartridge_has_rtc_persistence())
        : false;
    setPersistenceControlsEnabled({ hasRom: true, batterySave, rtc });
    setPersistenceInfoText(
      `Info: battery-save=${batterySave ? "yes" : "no"} | rtc=${rtc ? "yes" : "no"}`
    );
  }

  function setCartridgeInfoFromEmulator(emulator) {
    if (!cartInfoPre) {
      return;
    }
    if (!emulator) {
      cartInfoPre.textContent = "Cartridge: not loaded.";
      return;
    }
    cartInfoPre.textContent = emulator.cartridge_debug_report();
  }

  function drawFrameFromEmulator(emulator) {
    if (!emulator) {
      return;
    }

    const { width, height } = frameDimensionsForEmulator(emulator);
    if (width !== screenWidth || height !== screenHeight) {
      setScreenDimensions(width, height);
      updateScreenScale({ scale: videoSizeSelect?.value ?? DEFAULT_SCREEN_SCALE });
    }

    const frame = emulator.rgba_frame();
    if (frame.length !== frameRgba.length) {
      throw new Error(
        `RGBA frame length mismatch (expected ${frameRgba.length}, got ${frame.length})`
      );
    }
    frameRgba.set(frame);
    ctx.putImageData(frameImage, 0, 0);
    canvas.style.cursor = "default";
  }

  function drawScreenPrompt(promptText) {
    ctx.fillStyle = "#000000";
    ctx.fillRect(0, 0, screenWidth, screenHeight);
    ctx.fillStyle = "#b8c8d2";
    ctx.font = "11px IBM Plex Mono, monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(promptText, screenWidth / 2, screenHeight / 2);
  }

  function setScreenPromptForState({ hasRom = false, isRunning = false } = {}) {
    const prompt = canvasPromptForState({ hasRom, isRunning });
    if (!prompt.text) {
      canvas.style.cursor = "default";
      return;
    }

    drawScreenPrompt(prompt.text);
    canvas.style.cursor = prompt.clickable ? "pointer" : "default";
  }

  function clearScreen() {
    setScreenDimensions(SCREEN_WIDTH, SCREEN_HEIGHT);
    updateScreenScale({ scale: videoSizeSelect?.value ?? DEFAULT_SCREEN_SCALE });
    setScreenPromptForState({ hasRom: false, isRunning: false });
  }

  function setSerialOutputRaw(rawSerial) {
    if (!serialPre) {
      return;
    }
    serialPre.textContent = formatSerialOutputForDisplay(rawSerial);
  }

  function clearSerialOutput() {
    if (serialPre) {
      serialPre.textContent = "";
    }
  }

  syncControlPanelVisibility();
  setScreenDimensions(SCREEN_WIDTH, SCREEN_HEIGHT);
  updateScreenScale({ scale: DEFAULT_SCREEN_SCALE });
  setBatteryPowerOn(false);

  return {
    refs: {
      stage,
      contextPanel,
      settingsToggleButton,
      batteryLed,
      romFileInput,
      bootRomFileInput,
      bootRomFileButton,
      savFileInput,
      savFileButton,
      savDownloadButton,
      rtcFileInput,
      rtcFileButton,
      rtcDownloadButton,
      romResetButton,
      modelSelect,
      paletteSelect,
      paletteOverrideFileInput,
      paletteOverrideFileButton,
      paletteOverrideInfoLabel,
      paletteOverrideClearButton,
      videoSizeSelect,
      bootRomModelCheck,
      statusLabel,
      persistenceInfoLabel,
      bootRomInfoLabel,
      audioTelemetryLabel,
      cartInfoPre,
      serialPre,
      testToneCheckbox,
      audioEnableButton,
      audioResamplerSelect,
      debugRunStateLabel,
      controlNavDataButton,
      controlNavSystemButton,
      controlNavAudioButton,
      controlNavDebugButton,
      controlNavButtons,
      controlPanels,
      canvas,
    },
    setStatus,
    setBatteryPowerOn,
    setAudioTelemetryText,
    setPersistenceInfoText,
    setBootRomInfoText,
    setPaletteOverrideInfoText,
    setDebugRunStateText,
    setBootRomModelCheck,
    setPersistenceControlsEnabled,
    setRomControlsEnabled,
    setActiveControlSection,
    toggleControlSection,
    setSettingsPanelOpen,
    toggleSettingsPanel,
    setScreenPromptForState,
    updateScreenScale,
    setPersistenceInfoFromEmulator,
    setCartridgeInfoFromEmulator,
    drawFrameFromEmulator,
    clearScreen,
    setSerialOutputRaw,
    clearSerialOutput,
  };
}

export function formatSerialOutputForDisplay(rawSerial) {
  if (!rawSerial) {
    return "";
  }

  const MAX_VISIBLE_CHARS = 4096;
  let nonTextCount = 0;
  let formatted = "";

  for (const ch of rawSerial) {
    const code = ch.codePointAt(0);
    if (code === undefined) {
      continue;
    }
    if (ch === "\n" || ch === "\r" || ch === "\t" || (code >= 0x20 && code <= 0x7e)) {
      formatted += ch;
      continue;
    }
    nonTextCount += 1;
  }

  if (formatted.length === 0 && nonTextCount > 0) {
    return `[serial debug text hidden: received ${nonTextCount} non-text byte(s)]`;
  }

  if (formatted.length > MAX_VISIBLE_CHARS) {
    formatted = formatted.slice(-MAX_VISIBLE_CHARS);
    formatted = `[serial output truncated, showing last ${MAX_VISIBLE_CHARS} chars]\n${formatted}`;
  }

  if (nonTextCount > 0) {
    formatted += `\n[filtered ${nonTextCount} non-text byte(s)]`;
  }

  return formatted;
}
