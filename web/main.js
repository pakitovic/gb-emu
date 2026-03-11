import initWasm, * as wasmBindings from "./pkg/gb_emu.js";
import { createWebAudioController } from "./audio-controller.mjs";
import {
  classifyBootRomForWebHardware,
  isValidStoredBootRomForModel,
} from "./bootrom-normalizer.mjs";
import { createWebBootRomPersistence } from "./bootrom-persistence.mjs";
import { bindKeyboardInput } from "./input.mjs";
import {
  buildRomLoadedStatusMessage,
  createWebEmulatorFromRomBytes,
  createWebEmulatorFromRomFile,
} from "./rom-loading.mjs";
import { createWebPaletteOverridePersistence } from "./palette-override-persistence.mjs";
import { createWebSavePersistence } from "./save-persistence.mjs";
import { createUi, shouldCloseSettingsPanelOnPointerDown } from "./ui.mjs";

const ui = createUi();
const WebEmulator = wasmBindings.WebEmulator;
const classifyBootRomFileName = wasmBindings.classifyBootRomFileName;

let wasmReady = false;
let emulator = null;
let rafId = 0;
let lastFrameTimeMs = 0;
let loadedRomState = null;
let loadedPaletteOverridesState = null;
let isRunning = false;

const audioController = createWebAudioController({
  getEmulator: () => emulator,
  getResamplerQuality: () => ui.refs.audioResamplerSelect?.value || "cubic",
  getTestToneEnabled: () => ui.refs.testToneCheckbox?.checked || false,
  setStatus: (message) => ui.setStatus(message),
  setAudioTelemetryText: (message) => ui.setAudioTelemetryText(message),
});
const savePersistence = createWebSavePersistence({ debounceMs: 2000 });
const bootRomPersistence = createWebBootRomPersistence();
const paletteOverridePersistence = createWebPaletteOverridePersistence();

function selectedModel() {
  return ui.refs.modelSelect?.value || "dmg";
}

function selectedPalette() {
  return ui.refs.paletteSelect?.value || "auto";
}

function applySelectedPaletteToEmulator() {
  if (!emulator || typeof emulator.set_video_palette !== "function") {
    return;
  }
  emulator.set_video_palette(selectedPalette());
}

function applyLoadedPaletteOverridesToEmulator() {
  if (!emulator) {
    return;
  }
  if (!loadedPaletteOverridesState) {
    if (typeof emulator.clearPaletteOverrides === "function") {
      emulator.clearPaletteOverrides();
    }
    return;
  }
  if (typeof emulator.setPaletteOverridesIni === "function") {
    emulator.setPaletteOverridesIni(loadedPaletteOverridesState.text);
    loadedPaletteOverridesState = {
      ...loadedPaletteOverridesState,
      entryCount: emulator.paletteOverrideCount(),
    };
  }
}

function refreshPaletteOverrideInfo() {
  if (!loadedPaletteOverridesState) {
    ui.setPaletteOverrideInfoText("Palette overrides: none loaded.");
    if (ui.refs.paletteOverrideClearButton) {
      ui.refs.paletteOverrideClearButton.disabled = true;
    }
    return;
  }

  const countLabel =
    typeof loadedPaletteOverridesState.entryCount === "number"
      ? `${loadedPaletteOverridesState.entryCount} entry(s)`
      : "pending apply";
  ui.setPaletteOverrideInfoText(
    `Palette overrides: ${loadedPaletteOverridesState.name} (${countLabel}).`
  );
  if (ui.refs.paletteOverrideClearButton) {
    ui.refs.paletteOverrideClearButton.disabled = false;
  }
}

function loadStoredPaletteOverrides() {
  const stored = paletteOverridePersistence.loadPaletteOverrideState();
  if (!stored) {
    loadedPaletteOverridesState = null;
    return false;
  }

  try {
    const parsedEntryCount = wasmBindings.parsePaletteOverridesIniEntryCount(stored.text);
    loadedPaletteOverridesState = {
      ...stored,
      entryCount: parsedEntryCount,
    };
    return false;
  } catch {
    paletteOverridePersistence.removePaletteOverrideState();
    loadedPaletteOverridesState = null;
    return true;
  }
}

function refreshAudioButtonLabel() {
  if (!ui.refs.audioEnableButton) {
    return;
  }
  ui.refs.audioEnableButton.textContent = audioController.isEnabled()
    ? "Disable audio"
    : "Enable audio";
}

function refreshRomControls() {
  ui.setRomControlsEnabled({ hasRom: Boolean(emulator), isRunning });
  ui.setScreenPromptForState({ hasRom: Boolean(emulator), isRunning });
  ui.setBatteryPowerOn(Boolean(emulator));
}

function getBootRomValidationState(model, { removeInvalid } = { removeInvalid: true }) {
  const bootRomBytes = bootRomPersistence.loadBootRomBytesForModel(model);
  if (!(bootRomBytes instanceof Uint8Array) || bootRomBytes.length < 0x100) {
    return {
      bytes: null,
      isValid: false,
      hadStored: false,
      removedInvalid: false,
    };
  }

  if (!wasmReady || typeof classifyBootRomFileName !== "function") {
    return {
      bytes: bootRomBytes,
      isValid: false,
      hadStored: true,
      removedInvalid: false,
    };
  }

  const isValid = isValidStoredBootRomForModel({
    model,
    bootRomBytes,
    classifyBootRomFileName,
  });
  if (isValid) {
    return {
      bytes: bootRomBytes,
      isValid: true,
      hadStored: true,
      removedInvalid: false,
    };
  }

  if (removeInvalid) {
    bootRomPersistence.removeBootRomForModel(model);
    return {
      bytes: null,
      isValid: false,
      hadStored: true,
      removedInvalid: true,
    };
  }

  return {
    bytes: null,
    isValid: false,
    hadStored: true,
    removedInvalid: false,
  };
}

function refreshBootRomInfo() {
  const model = selectedModel();
  if (!wasmReady || typeof classifyBootRomFileName !== "function") {
    ui.setBootRomModelCheck(false);
    ui.setBootRomInfoText(`Boot ROM (${model}): waiting for WASM classifier.`);
    return;
  }

  const state = getBootRomValidationState(model, { removeInvalid: true });
  ui.setBootRomModelCheck(state.isValid);

  if (state.isValid) {
    ui.setBootRomInfoText(`Boot ROM (${model}): configured and validated.`);
    return;
  }

  if (state.hadStored && state.removedInvalid) {
    ui.setBootRomInfoText(`Boot ROM (${model}): invalid entry removed from storage.`);
    return;
  }

  ui.setBootRomInfoText(`Boot ROM (${model}): not configured.`);
}

function loadBootRomBytesForModel(model) {
  const state = getBootRomValidationState(model, { removeInvalid: true });
  return state.isValid ? state.bytes : null;
}

function stepFrame(nowMs) {
  if (!emulator) {
    ui.setCartridgeInfoFromEmulator(emulator);
    ui.setPersistenceInfoFromEmulator(emulator);
    audioController.updateTelemetry();
    rafId = requestAnimationFrame(stepFrame);
    return;
  }

  if (!isRunning) {
    lastFrameTimeMs = 0;
    audioController.updateTelemetry();
    rafId = requestAnimationFrame(stepFrame);
    return;
  }

  const elapsedMs = lastFrameTimeMs > 0 ? nowMs - lastFrameTimeMs : 16.0;
  lastFrameTimeMs = nowMs;
  const clampedMs = Math.max(0, Math.min(elapsedMs, 100));

  try {
    emulator.set_host_rtc_epoch_secs(Math.floor(Date.now() / 1000));
    emulator.run_for_elapsed_micros(Math.floor(clampedMs * 1000));
    ui.drawFrameFromEmulator(emulator);
    ui.setSerialOutputRaw(emulator.serial_output());
    savePersistence.tick();
  } catch (error) {
    console.error(error);
    isRunning = false;
    refreshRomControls();
    ui.setStatus(`Runtime error: ${error}`);
  }

  audioController.updateTelemetry();
  rafId = requestAnimationFrame(stepFrame);
}

async function loadRom(file) {
  if (!wasmReady) {
    ui.setStatus("WASM is still loading.");
    return;
  }
  if (!file) {
    return;
  }

  const model = selectedModel();
  const bootRomBytes = loadBootRomBytesForModel(model);
  await closeRom({ keepStatus: true, keepAudio: true });

  const loaded = await createWebEmulatorFromRomFile({
    file,
    WebEmulator,
    model,
    bootRomBytes,
  });
  if (!loaded) {
    return;
  }
  await finishRomActivation({
    nextEmulator: loaded.emulator,
    romBytes: loaded.romBytes,
    fileName: file.name,
    model,
    bootRomBytes,
    resetSerialOutput: true,
    statusMessage: null,
    autoStart: true,
  });
}

async function closeRom({ keepStatus = false, keepAudio = false } = {}) {
  flushCurrentPersistence();
  savePersistence.detachRom();
  emulator = null;
  loadedRomState = null;
  isRunning = false;
  lastFrameTimeMs = 0;

  if (!keepAudio && audioController.isEnabled()) {
    await audioController.disable();
    refreshAudioButtonLabel();
  }

  ui.clearSerialOutput();
  ui.clearScreen();
  ui.setCartridgeInfoFromEmulator(emulator);
  ui.setPersistenceInfoFromEmulator(emulator);
  refreshRomControls();

  if (!keepStatus) {
    ui.setStatus("ROM closed. Load a ROM.");
  }
}

async function resetRom() {
  if (!loadedRomState) {
    ui.setStatus("Load a ROM before resetting.");
    return;
  }

  const { fileName, model, romBytes, bootRomBytes } = loadedRomState;
  await closeRom({ keepStatus: true, keepAudio: true });
  const nextEmulator = createWebEmulatorFromRomBytes({
    romBytes,
    WebEmulator,
    model,
    bootRomBytes,
  });
  await finishRomActivation({
    nextEmulator,
    romBytes,
    fileName,
    model,
    bootRomBytes,
    resetSerialOutput: true,
    statusMessage: `Reset loaded: ${fileName}.`,
    autoStart: true,
  });
}

async function importBootRoms(files) {
  if (!wasmReady || typeof classifyBootRomFileName !== "function") {
    ui.setStatus("WASM classifier is still loading.");
    return;
  }

  if (!Array.isArray(files) || files.length === 0) {
    return;
  }

  const storedModels = new Set();
  const invalidFiles = [];
  const unsupportedFiles = [];
  const storageErrors = [];

  for (const file of files) {
    const bootRomBytes = new Uint8Array(await file.arrayBuffer());
    const classification = classifyBootRomForWebHardware(
      bootRomBytes,
      classifyBootRomFileName
    );

    if (classification.kind === "invalid") {
      invalidFiles.push(file.name);
      continue;
    }

    if (classification.kind === "known_unsupported") {
      unsupportedFiles.push(`${file.name} -> ${classification.canonicalFileName}`);
      continue;
    }

    const persisted = bootRomPersistence.storeBootRomBytesForModel(
      classification.model,
      bootRomBytes
    );
    if (!persisted.ok) {
      storageErrors.push(`${file.name} (${classification.model}): ${persisted.error}`);
      continue;
    }

    storedModels.add(classification.model);
  }

  if (loadedRomState && storedModels.has(loadedRomState.model)) {
    loadedRomState = {
      ...loadedRomState,
      bootRomBytes: loadBootRomBytesForModel(loadedRomState.model),
    };
  }

  refreshBootRomInfo();

  const parts = [];
  if (storedModels.size > 0) {
    parts.push(`stored for ${Array.from(storedModels).sort().join(", ")}`);
  }
  if (invalidFiles.length > 0) {
    parts.push(`invalid=${invalidFiles.length}`);
  }
  if (unsupportedFiles.length > 0) {
    parts.push(`known-but-unsupported=${unsupportedFiles.length}`);
  }
  if (storageErrors.length > 0) {
    parts.push(`storage-errors=${storageErrors.length}`);
  }

  if (parts.length === 0) {
    ui.setStatus("Boot ROM import finished: no valid web-compatible boot ROM was found.");
    return;
  }

  let message = `Boot ROM import finished (${files.length} file(s)): ${parts.join(" | ")}.`;
  if (loadedRomState && storedModels.has(loadedRomState.model)) {
    message += " Use Reset to apply it to the loaded ROM.";
  }
  ui.setStatus(message);
}

async function importPaletteOverrides(file) {
  if (!wasmReady) {
    ui.setStatus("WASM is still loading.");
    return;
  }
  if (!file) {
    return;
  }

  const ini = await file.text();
  const parsedEntryCount = wasmBindings.parsePaletteOverridesIniEntryCount(ini);
  if (emulator && typeof emulator.setPaletteOverridesIni === "function") {
    emulator.setPaletteOverridesIni(ini);
  }

  loadedPaletteOverridesState = {
    name: file.name,
    text: ini,
    entryCount:
      emulator && typeof emulator.paletteOverrideCount === "function"
        ? emulator.paletteOverrideCount()
        : parsedEntryCount,
  };
  const persisted = paletteOverridePersistence.storePaletteOverrideState({
    name: file.name,
    text: ini,
  });
  refreshPaletteOverrideInfo();

  if (emulator) {
    ui.drawFrameFromEmulator(emulator);
  }

  ui.setStatus(
    persisted.ok
      ? `Palette overrides loaded from ${file.name} (${parsedEntryCount} entry(s)).`
      : `Palette overrides loaded from ${file.name} (${parsedEntryCount} entry(s)), but browser storage persistence failed.`
  );
}

function clearPaletteOverrides() {
  loadedPaletteOverridesState = null;
  const removedFromStorage = paletteOverridePersistence.removePaletteOverrideState();
  if (emulator && typeof emulator.clearPaletteOverrides === "function") {
    emulator.clearPaletteOverrides();
  }
  refreshPaletteOverrideInfo();
  if (emulator) {
    ui.drawFrameFromEmulator(emulator);
  }
  ui.setStatus(
    removedFromStorage
      ? "Palette overrides cleared."
      : "Palette overrides cleared for the current session, but browser storage removal failed."
  );
}

async function importSav(file) {
  if (!emulator) {
    ui.setStatus("Load a ROM before importing a SAV file.");
    return;
  }
  if (!file) {
    return;
  }

  const saveBytes = new Uint8Array(await file.arrayBuffer());
  const imported = savePersistence.importSavBytes(saveBytes);
  if (!imported) {
    ui.setStatus("Failed to import SAV (storage unavailable or write blocked).");
    return;
  }

  ui.setStatus(
    `Imported SAV from ${file.name} (${saveBytes.length} bytes). Use Reset to reload game state if needed.`
  );
}

async function importRtc(file) {
  if (!emulator) {
    ui.setStatus("Load a ROM before importing an RTC file.");
    return;
  }
  if (!file) {
    return;
  }

  const rtcBytes = new Uint8Array(await file.arrayBuffer());
  const imported = savePersistence.importRtcBytes(rtcBytes);
  if (!imported) {
    ui.setStatus("Failed to import RTC (invalid data, unsupported cartridge, or storage blocked).");
    return;
  }

  ui.setStatus(
    `Imported RTC from ${file.name} (${rtcBytes.length} bytes). Use Reset to reload RTC state if needed.`
  );
}

function exportSav() {
  if (!emulator) {
    ui.setStatus("Load a ROM before exporting a SAV file.");
    return;
  }

  savePersistence.flushNow();
  const saveBytes = savePersistence.exportSavBytes();
  if (!saveBytes) {
    ui.setStatus("This cartridge does not expose battery-backed save RAM.");
    return;
  }

  const fileName = savePersistence.exportSavFileName();
  downloadBytesAsFile(saveBytes, fileName, "application/octet-stream");
  ui.setStatus(`Exported SAV to ${fileName} (${saveBytes.length} bytes).`);
}

function exportRtc() {
  if (!emulator) {
    ui.setStatus("Load a ROM before exporting an RTC file.");
    return;
  }

  savePersistence.flushNow();
  const rtcBytes = savePersistence.exportRtcBytes();
  if (!rtcBytes) {
    ui.setStatus("This cartridge does not expose RTC persistence data.");
    return;
  }

  const fileName = savePersistence.exportRtcFileName();
  downloadBytesAsFile(rtcBytes, fileName, "application/octet-stream");
  ui.setStatus(`Exported RTC to ${fileName} (${rtcBytes.length} bytes).`);
}

function flushCurrentPersistence() {
  if (!emulator) {
    return;
  }
  savePersistence.flushNow();
}

function handlePagePersistenceFlush() {
  flushCurrentPersistence();
  ui.setPersistenceInfoFromEmulator(emulator);
}

function bindDomEvents() {
  ui.refs.settingsToggleButton?.addEventListener("click", () => {
    ui.toggleSettingsPanel();
  });

  document.addEventListener("pointerdown", (event) => {
    const panel = ui.refs.contextPanel;
    if (!panel) {
      return;
    }

    const target = event.target;
    const isNodeTarget = typeof Node !== "undefined" && target instanceof Node;
    const isInsidePanel = isNodeTarget ? panel.contains(target) : false;
    const isInsideToggle = isNodeTarget
      ? Boolean(ui.refs.settingsToggleButton?.contains(target))
      : false;

    if (
      shouldCloseSettingsPanelOnPointerDown({
        isPanelOpen: !panel.hidden,
        isInsidePanel,
        isInsideToggle,
      })
    ) {
      ui.setSettingsPanelOpen(false);
    }
  });

  for (const button of ui.refs.controlNavButtons ?? []) {
    button.addEventListener("click", () => {
      ui.toggleControlSection(button.dataset.controlSection);
    });
  }

  ui.refs.romFileInput?.addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    if (file) {
      ui.setSettingsPanelOpen(false);
    }
    try {
      await loadRom(file);
    } catch (error) {
      console.error(error);
      ui.setStatus(`ROM load error: ${error}`);
    } finally {
      event.target.value = "";
    }
  });

  ui.refs.canvas?.addEventListener("click", () => {
    if (!emulator) {
      ui.refs.romFileInput?.click();
    }
  });

  ui.refs.bootRomFileInput?.addEventListener("change", async (event) => {
    const files = Array.from(event.target.files ?? []);
    try {
      await importBootRoms(files);
    } catch (error) {
      console.error(error);
      ui.setStatus(`Boot ROM import error: ${error}`);
    } finally {
      event.target.value = "";
    }
  });

  ui.refs.paletteOverrideFileInput?.addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    try {
      await importPaletteOverrides(file);
    } catch (error) {
      console.error(error);
      ui.setStatus(`Palette override import error: ${error}`);
    } finally {
      event.target.value = "";
    }
  });

  ui.refs.savFileInput?.addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    try {
      await importSav(file);
    } catch (error) {
      console.error(error);
      ui.setStatus(`SAV import error: ${error}`);
    } finally {
      event.target.value = "";
    }
  });

  ui.refs.rtcFileInput?.addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    try {
      await importRtc(file);
    } catch (error) {
      console.error(error);
      ui.setStatus(`RTC import error: ${error}`);
    } finally {
      event.target.value = "";
    }
  });

  ui.refs.modelSelect?.addEventListener("change", () => {
    refreshBootRomInfo();
  });

  ui.refs.paletteSelect?.addEventListener("change", () => {
    if (!emulator) {
      return;
    }
    try {
      applySelectedPaletteToEmulator();
      ui.drawFrameFromEmulator(emulator);
    } catch (error) {
      console.error(error);
      ui.setStatus(`Palette update error: ${error}`);
    }
  });
  ui.refs.paletteOverrideClearButton?.addEventListener("click", clearPaletteOverrides);

  ui.refs.videoSizeSelect?.addEventListener("change", () => {
    ui.updateScreenScale({ scale: ui.refs.videoSizeSelect?.value });
  });

  ui.refs.testToneCheckbox?.addEventListener("change", () => {
    audioController.handleTestToneChanged();
  });

  ui.refs.audioResamplerSelect?.addEventListener("change", () => {
    audioController.handleResamplerChanged();
  });

  ui.refs.audioEnableButton?.addEventListener("click", async () => {
    await audioController.toggle();
    refreshAudioButtonLabel();
  });
  ui.refs.romResetButton?.addEventListener("click", async () => {
    ui.setSettingsPanelOpen(false);
    try {
      await resetRom();
    } catch (error) {
      console.error(error);
      ui.setStatus(`ROM reset error: ${error}`);
    }
  });
  ui.refs.savDownloadButton?.addEventListener("click", exportSav);
  ui.refs.rtcDownloadButton?.addEventListener("click", exportRtc);

  bindKeyboardInput({ getEmulator: () => emulator });

  window.addEventListener("pagehide", handlePagePersistenceFlush);
  window.addEventListener("blur", handlePagePersistenceFlush);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      handlePagePersistenceFlush();
    }
  });
}

function downloadBytesAsFile(bytes, fileName, mimeType) {
  const blob = new Blob([bytes], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(url);
}

async function bootstrap() {
  bindDomEvents();
  await initWasm();
  wasmReady = true;
  const removedInvalidStoredPaletteOverrides = loadStoredPaletteOverrides();
  refreshAudioButtonLabel();
  refreshBootRomInfo();
  refreshPaletteOverrideInfo();
  refreshRomControls();
  ui.setStatus(
    removedInvalidStoredPaletteOverrides
      ? "WASM ready. Invalid stored palette overrides were removed."
      : "WASM ready. Load a ROM."
  );
  ui.setCartridgeInfoFromEmulator(emulator);
  ui.setPersistenceInfoFromEmulator(emulator);
  audioController.updateTelemetry();

  if (rafId === 0) {
    rafId = requestAnimationFrame(stepFrame);
  }
}

bootstrap().catch((error) => {
  console.error(error);
  ui.setStatus(`Bootstrap error: ${error}`);
});

async function finishRomActivation({
  nextEmulator,
  romBytes,
  fileName,
  model,
  bootRomBytes,
  resetSerialOutput,
  statusMessage,
  autoStart = false,
}) {
  loadedRomState = {
    fileName,
    model,
    romBytes,
    bootRomBytes: bootRomBytes ? bootRomBytes.slice(0, 0x100) : null,
  };
  emulator = nextEmulator;
  isRunning = false;
  lastFrameTimeMs = 0;
  applySelectedPaletteToEmulator();
  applyLoadedPaletteOverridesToEmulator();
  emulator.set_host_rtc_epoch_secs(Math.floor(Date.now() / 1000));
  savePersistence.attachRom({
    romBytes,
    fileName,
    nextEmulator: emulator,
  });
  ui.setPersistenceInfoFromEmulator(emulator);
  refreshRomControls();

  audioController.onEmulatorLoaded();

  const warningCount = emulator.cartridge_warning_count();
  ui.setCartridgeInfoFromEmulator(emulator);
  if (resetSerialOutput) {
    ui.clearSerialOutput();
  }

  const defaultMessage = `${buildRomLoadedStatusMessage({
    fileName,
    romTitle: emulator.rom_title(),
    model,
    warningCount,
  })} ${bootRomBytes ? "Boot ROM active." : "No boot ROM configured for this hardware."}`;

  if (autoStart) {
    if (!audioController.isEnabled()) {
      await audioController.enable();
      refreshAudioButtonLabel();
    }
    isRunning = true;
    lastFrameTimeMs = 0;
    refreshRomControls();
  }

  ui.setStatus(statusMessage ?? `${defaultMessage} Running.`);
}
