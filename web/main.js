import initWasm, { WebEmulator } from "./pkg/gb_emu.js";
import { createWebAudioController } from "./audio-controller.mjs";
import { bindKeyboardInput } from "./input.mjs";
import {
  buildRomLoadedStatusMessage,
  createWebEmulatorFromRomFile,
} from "./rom-loading.mjs";
import { createUi } from "./ui.mjs";

const ui = createUi();

let wasmReady = false;
let emulator = null;
let rafId = 0;
let lastFrameTimeMs = 0;

const audioController = createWebAudioController({
  getEmulator: () => emulator,
  getResamplerQuality: () => ui.refs.audioResamplerSelect?.value || "cubic",
  getTestToneEnabled: () => ui.refs.testToneCheckbox?.checked || false,
  setStatus: (message) => ui.setStatus(message),
  setAudioTelemetryText: (message) => ui.setAudioTelemetryText(message),
});

function stepFrame(nowMs) {
  if (!emulator) {
    ui.setCartridgeInfoFromEmulator(emulator);
    audioController.updateTelemetry();
    rafId = requestAnimationFrame(stepFrame);
    return;
  }

  const elapsedMs = lastFrameTimeMs > 0 ? nowMs - lastFrameTimeMs : 16.0;
  lastFrameTimeMs = nowMs;
  const clampedMs = Math.max(0, Math.min(elapsedMs, 100));

  try {
    emulator.run_for_elapsed_micros(Math.floor(clampedMs * 1000));
    ui.drawFrameFromEmulator(emulator);
    ui.setSerialOutputRaw(emulator.serial_output());
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
  emulator = await createWebEmulatorFromRomFile({
    file,
    WebEmulator,
    model,
  });
  if (!emulator) {
    return;
  }

  audioController.onEmulatorLoaded();

  const warningCount = emulator.cartridge_warning_count();
  ui.drawFrameFromEmulator(emulator);
  ui.setCartridgeInfoFromEmulator(emulator);
  ui.clearSerialOutput();
  ui.setStatus(
    buildRomLoadedStatusMessage({
      fileName: file.name,
      romTitle: emulator.rom_title(),
      model,
      warningCount,
    })
  );
}

function bindDomEvents() {
  ui.refs.romFileInput?.addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    try {
      await loadRom(file);
    } catch (error) {
      console.error(error);
      ui.setStatus(`ROM load error: ${error}`);
    }
  });

  ui.refs.testToneCheckbox?.addEventListener("change", () => {
    audioController.handleTestToneChanged();
  });

  ui.refs.audioResamplerSelect?.addEventListener("change", () => {
    audioController.handleResamplerChanged();
  });

  ui.refs.audioEnableButton?.addEventListener("click", () => {
    void audioController.enable();
  });

  bindKeyboardInput({ getEmulator: () => emulator });
}

async function bootstrap() {
  bindDomEvents();
  await initWasm();
  wasmReady = true;
  ui.setStatus("WASM ready. Load a ROM to start.");
  ui.setCartridgeInfoFromEmulator(emulator);
  audioController.updateTelemetry();

  if (rafId === 0) {
    rafId = requestAnimationFrame(stepFrame);
  }
}

bootstrap().catch((error) => {
  console.error(error);
  ui.setStatus(`Bootstrap error: ${error}`);
});
