import initWasm, { WebEmulator } from "./pkg/gb_emu.js";
import { createWebAudioController } from "./audio-controller.mjs";
import { bindKeyboardInput } from "./input.mjs";
import {
  buildRomLoadedStatusMessage,
  createWebEmulatorFromRomFile,
} from "./rom-loading.mjs";
import { createWebSavePersistence } from "./save-persistence.mjs";
import { createUi } from "./ui.mjs";

const ui = createUi();

let wasmReady = false;
let emulator = null;
let rafId = 0;
let lastFrameTimeMs = 0;
let loadedRomState = null;

const audioController = createWebAudioController({
  getEmulator: () => emulator,
  getResamplerQuality: () => ui.refs.audioResamplerSelect?.value || "cubic",
  getTestToneEnabled: () => ui.refs.testToneCheckbox?.checked || false,
  setStatus: (message) => ui.setStatus(message),
  setAudioTelemetryText: (message) => ui.setAudioTelemetryText(message),
});
const savePersistence = createWebSavePersistence({ debounceMs: 2000 });

function refreshAudioButtonLabel() {
  if (!ui.refs.audioEnableButton) {
    return;
  }
  ui.refs.audioEnableButton.textContent = audioController.isEnabled()
    ? "Disable audio"
    : "Enable audio";
}

function stepFrame(nowMs) {
  if (!emulator) {
    ui.setCartridgeInfoFromEmulator(emulator);
    ui.setPersistenceInfoFromEmulator(emulator);
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

  const model = ui.refs.modelSelect?.value || undefined;
  await closeRom({ keepStatus: true, keepAudio: true });

  const loaded = await createWebEmulatorFromRomFile({
    file,
    WebEmulator,
    model,
  });
  if (!loaded) {
    return;
  }
  finishRomActivation({
    nextEmulator: loaded.emulator,
    romBytes: loaded.romBytes,
    fileName: file.name,
    model,
    resetSerialOutput: true,
    statusMessage: null,
  });
}

async function closeRom({ keepStatus = false, keepAudio = false } = {}) {
  flushCurrentPersistence();
  savePersistence.detachRom();
  emulator = null;
  loadedRomState = null;
  lastFrameTimeMs = 0;

  if (!keepAudio && audioController.isEnabled()) {
    await audioController.disable();
    refreshAudioButtonLabel();
  }

  ui.clearSerialOutput();
  ui.clearScreen();
  ui.setCartridgeInfoFromEmulator(emulator);
  ui.setPersistenceInfoFromEmulator(emulator);

  if (!keepStatus) {
    ui.setStatus("ROM closed. Load a ROM to start emulation.");
  }
}

async function resetRom() {
  if (!loadedRomState) {
    ui.setStatus("Load a ROM before resetting.");
    return;
  }

  const { fileName, model, romBytes } = loadedRomState;
  await closeRom({ keepStatus: true, keepAudio: true });
  const nextEmulator = new WebEmulator(romBytes, model || undefined);
  finishRomActivation({
    nextEmulator,
    romBytes,
    fileName,
    model,
    resetSerialOutput: true,
    statusMessage: `ROM reset: ${fileName}`,
  });
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
    `Imported SAV from ${file.name} (${saveBytes.length} bytes). Use Reset ROM to reload the game state if it is already running.`
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
    `Imported RTC from ${file.name} (${rtcBytes.length} bytes). Use Reset ROM to reload the game state if it is already running.`
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
  ui.refs.romFileInput?.addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    try {
      await loadRom(file);
    } catch (error) {
      console.error(error);
      ui.setStatus(`ROM load error: ${error}`);
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
    try {
      await resetRom();
    } catch (error) {
      console.error(error);
      ui.setStatus(`ROM reset error: ${error}`);
    }
  });
  ui.refs.romCloseButton?.addEventListener("click", async () => {
    await closeRom();
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
  refreshAudioButtonLabel();
  ui.setStatus("WASM ready. Load a ROM to start.");
  ui.clearScreen();
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

function finishRomActivation({
  nextEmulator,
  romBytes,
  fileName,
  model,
  resetSerialOutput,
  statusMessage,
}) {
  loadedRomState = {
    fileName,
    model,
    romBytes,
  };
  emulator = nextEmulator;
  emulator.set_host_rtc_epoch_secs(Math.floor(Date.now() / 1000));
  savePersistence.attachRom({
    romBytes,
    fileName,
    nextEmulator: emulator,
  });
  ui.setPersistenceInfoFromEmulator(emulator);

  audioController.onEmulatorLoaded();

  const warningCount = emulator.cartridge_warning_count();
  ui.drawFrameFromEmulator(emulator);
  ui.setCartridgeInfoFromEmulator(emulator);
  if (resetSerialOutput) {
    ui.clearSerialOutput();
  }
  ui.setStatus(
    statusMessage ??
      buildRomLoadedStatusMessage({
        fileName,
        romTitle: emulator.rom_title(),
        model,
        warningCount,
      })
  );
}
