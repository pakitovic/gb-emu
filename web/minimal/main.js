import initWasm, { WebEmulator } from "./pkg/gb_emu.js";
import {
  createAdaptiveQueueState,
  DEFAULT_ADAPTIVE_QUEUE_OPTIONS,
  updateAdaptiveQueueTarget,
} from "./audio-adaptive.mjs";

const SCREEN_WIDTH = 160;
const SCREEN_HEIGHT = 144;
const AUDIO_BLOCK_SAMPLES = 512;
const AUDIO_CHANNELS = 2;
const AUDIO_QUEUE_TARGET_INITIAL_SAMPLES = 4096;
const AUDIO_REFILL_INTERVAL_MS = 8;
const AUDIO_ADAPTIVE_QUEUE_OPTIONS = {
  ...DEFAULT_ADAPTIVE_QUEUE_OPTIONS,
};

const KEY_TO_BUTTON = {
  ArrowRight: 0,
  ArrowLeft: 1,
  ArrowUp: 2,
  ArrowDown: 3,
  KeyZ: 4,
  KeyX: 5,
  Backspace: 6,
  Enter: 7,
};

const romFileInput = document.getElementById("rom-file");
const modelSelect = document.getElementById("model");
const statusLabel = document.getElementById("status");
const audioTelemetryLabel = document.getElementById("audio-telemetry");
const cartInfoPre = document.getElementById("cart-info");
const serialPre = document.getElementById("serial");
const testToneCheckbox = document.getElementById("test-tone");
const audioEnableButton = document.getElementById("audio-enable");
const audioResamplerSelect = document.getElementById("audio-resampler");

const canvas = document.getElementById("screen");
const ctx = canvas.getContext("2d");
const frameRgba = new Uint8ClampedArray(SCREEN_WIDTH * SCREEN_HEIGHT * 4);
const frameImage = new ImageData(frameRgba, SCREEN_WIDTH, SCREEN_HEIGHT);

let wasmReady = false;
let emulator = null;
let rafId = 0;
let lastFrameTimeMs = 0;

let audioContext = null;
let audioNode = null;
let audioRefillTimerId = null;
let queuedAudioSamples = 0;
let audioWorkletLoaded = false;
let audioConsumedSamplesTotal = 0;
let audioUnderrunSamplesTotal = 0;
let audioQueueTargetSamples = AUDIO_QUEUE_TARGET_INITIAL_SAMPLES;
let audioAdaptiveQueueState = createAdaptiveQueueState();

function formatSerialOutputForDisplay(rawSerial) {
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

function setStatus(message) {
  statusLabel.textContent = message;
}

function updateCartridgeInfoPanel() {
  if (!cartInfoPre) {
    return;
  }
  if (!emulator) {
    cartInfoPre.textContent = "Cartridge: not loaded.";
    return;
  }
  cartInfoPre.textContent = emulator.cartridge_debug_report();
}

function drawFrame() {
  if (!emulator) {
    return;
  }

  const frame = emulator.grayscale_frame();
  for (let i = 0; i < frame.length; i += 1) {
    const shade = frame[i];
    const rgbaIndex = i * 4;
    frameRgba[rgbaIndex] = shade;
    frameRgba[rgbaIndex + 1] = shade;
    frameRgba[rgbaIndex + 2] = shade;
    frameRgba[rgbaIndex + 3] = 255;
  }
  ctx.putImageData(frameImage, 0, 0);
}

function resetAudioTelemetryState() {
  queuedAudioSamples = 0;
  audioConsumedSamplesTotal = 0;
  audioUnderrunSamplesTotal = 0;
  audioQueueTargetSamples = AUDIO_QUEUE_TARGET_INITIAL_SAMPLES;
  audioAdaptiveQueueState = createAdaptiveQueueState(performance.now(), 0);
}

function updateAudioTelemetry() {
  if (!audioTelemetryLabel) {
    return;
  }
  const resamplerQuality =
    emulator && typeof emulator.audio_resampler_quality === "function"
      ? emulator.audio_resampler_quality()
      : audioResamplerSelect?.value || "cubic";
  if (!audioContext || !audioNode) {
    audioTelemetryLabel.textContent = `Audio: disabled | resampler ${resamplerQuality}`;
    return;
  }

  const sampleRate = Math.max(1, audioContext.sampleRate || 48_000);
  const queuedMs = (queuedAudioSamples * 1000) / sampleRate;
  const targetMs = (audioQueueTargetSamples * 1000) / sampleRate;
  const underrunMs = (audioUnderrunSamplesTotal * 1000) / sampleRate;
  const playedSeconds = audioConsumedSamplesTotal / sampleRate;
  audioTelemetryLabel.textContent =
    `Audio: ${audioContext.state} | resampler ${resamplerQuality} | queued ${queuedMs.toFixed(1)}ms / target ${targetMs.toFixed(1)}ms | ` +
    `underruns ${audioUnderrunSamplesTotal} samples (${underrunMs.toFixed(2)}ms) | played ${playedSeconds.toFixed(1)}s`;
}

function applyAudioResamplerQuality() {
  if (!emulator || !audioResamplerSelect) {
    return;
  }
  const quality = audioResamplerSelect.value || "cubic";
  try {
    emulator.set_audio_resampler_quality(quality);
  } catch (error) {
    console.error(error);
    setStatus(`Audio resampler error: ${error}`);
  }
}

function maybeAdjustAudioQueueTarget(nowMs) {
  const result = updateAdaptiveQueueTarget({
    state: audioAdaptiveQueueState,
    nowMs,
    queuedSamples: queuedAudioSamples,
    targetSamples: audioQueueTargetSamples,
    totalUnderrunSamples: audioUnderrunSamplesTotal,
    blockSamples: AUDIO_BLOCK_SAMPLES,
    options: AUDIO_ADAPTIVE_QUEUE_OPTIONS,
  });
  audioQueueTargetSamples = result.targetSamples;
}

function stepFrame(nowMs) {
  if (!emulator) {
    updateCartridgeInfoPanel();
    updateAudioTelemetry();
    rafId = requestAnimationFrame(stepFrame);
    return;
  }

  const elapsedMs = lastFrameTimeMs > 0 ? nowMs - lastFrameTimeMs : 16.0;
  lastFrameTimeMs = nowMs;
  const clampedMs = Math.max(0, Math.min(elapsedMs, 100));

  try {
    emulator.run_for_elapsed_micros(Math.floor(clampedMs * 1000));
    drawFrame();
    serialPre.textContent = formatSerialOutputForDisplay(emulator.serial_output());
  } catch (error) {
    console.error(error);
    setStatus(`Runtime error: ${error}`);
  }

  updateAudioTelemetry();
  rafId = requestAnimationFrame(stepFrame);
}

function setButtonFromKeyboardEvent(event, pressed) {
  if (!emulator) {
    return;
  }
  const buttonIndex = KEY_TO_BUTTON[event.code];
  if (buttonIndex === undefined) {
    return;
  }
  event.preventDefault();
  emulator.set_button(buttonIndex, pressed);
}

function disconnectAudioBackend() {
  if (audioRefillTimerId !== null) {
    window.clearInterval(audioRefillTimerId);
    audioRefillTimerId = null;
  }
  if (audioNode) {
    audioNode.port.onmessage = null;
    audioNode.disconnect();
    audioNode = null;
  }
  resetAudioTelemetryState();
  updateAudioTelemetry();
}

async function ensureAudioContext() {
  if (audioContext) {
    if (audioContext.state === "suspended") {
      await audioContext.resume();
    }
    return audioContext;
  }

  const AudioContextCtor = window.AudioContext || window.webkitAudioContext;
  if (!AudioContextCtor) {
    throw new Error("WebAudio is not available in this browser");
  }

  audioContext = new AudioContextCtor({ sampleRate: 48_000 });
  if (audioContext.state === "suspended") {
    await audioContext.resume();
  }
  return audioContext;
}

async function enableAudio() {
  if (!emulator) {
    setStatus("Load a ROM before enabling audio.");
    return;
  }

  try {
    const ac = await ensureAudioContext();
    if (!ac.audioWorklet || typeof ac.audioWorklet.addModule !== "function") {
      throw new Error("AudioWorklet is not available in this browser");
    }
    emulator.set_audio_sample_rate(ac.sampleRate);
    applyAudioResamplerQuality();
    emulator.set_audio_test_tone_enabled(testToneCheckbox.checked);

    disconnectAudioBackend();
    if (!audioWorkletLoaded) {
      await ac.audioWorklet.addModule("./audio-worklet.js");
      audioWorkletLoaded = true;
    }
    audioNode = new AudioWorkletNode(ac, "gb-audio-processor", {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [AUDIO_CHANNELS],
      channelCount: AUDIO_CHANNELS,
      channelCountMode: "explicit",
      channelInterpretation: "speakers",
    });
    audioNode.port.onmessage = (event) => {
      const data = event.data;
      if (!data || data.type !== "consumed") {
        return;
      }
      const consumedSamples = data.samples | 0;
      const underrunSamples = data.underruns | 0;
      queuedAudioSamples = Math.max(0, queuedAudioSamples - consumedSamples);
      audioConsumedSamplesTotal += consumedSamples;
      audioUnderrunSamplesTotal += underrunSamples;
    };
    audioNode.connect(ac.destination);

    refillAudioQueue();
    audioRefillTimerId = window.setInterval(() => {
      if (ac.state !== "running") {
        return;
      }
      refillAudioQueue();
    }, AUDIO_REFILL_INTERVAL_MS);

    setStatus(`AudioWorklet enabled (${ac.sampleRate} Hz, block ${AUDIO_BLOCK_SAMPLES}).`);
    updateAudioTelemetry();
  } catch (error) {
    console.error(error);
    setStatus(`Audio setup error: ${error}`);
    updateAudioTelemetry();
  }
}

function refillAudioQueue() {
  if (!emulator || !audioNode) {
    return;
  }

  maybeAdjustAudioQueueTarget(performance.now());

  let guard = 0;
  while (queuedAudioSamples < audioQueueTargetSamples && guard < 16) {
    const samples = emulator.drain_audio_samples_realtime(AUDIO_BLOCK_SAMPLES);
    if (!samples || samples.length === 0) {
      break;
    }
    audioNode.port.postMessage({ type: "samples", samples });
    const enqueuedFrames = Math.floor(samples.length / AUDIO_CHANNELS);
    queuedAudioSamples += enqueuedFrames;
    guard += 1;
  }
}

async function loadRom(file) {
  if (!wasmReady) {
    setStatus("WASM is still loading.");
    return;
  }
  if (!file) {
    return;
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  const model = modelSelect.value || undefined;
  emulator = new WebEmulator(bytes, model);
  applyAudioResamplerQuality();
  emulator.set_audio_test_tone_enabled(testToneCheckbox.checked);

  if (audioContext) {
    emulator.set_audio_sample_rate(audioContext.sampleRate);
  }
  if (audioNode) {
    audioNode.port.postMessage({ type: "reset" });
    resetAudioTelemetryState();
    refillAudioQueue();
  }

  const warningCount = emulator.cartridge_warning_count();
  const warningText = warningCount > 0 ? ` (${warningCount} header warnings)` : "";
  drawFrame();
  updateCartridgeInfoPanel();
  updateAudioTelemetry();
  serialPre.textContent = "";
  setStatus(
    `Loaded ${file.name} (${emulator.rom_title()}) on model ${model}.${warningText}`
  );
}

function bindDomEvents() {
  romFileInput.addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    try {
      await loadRom(file);
    } catch (error) {
      console.error(error);
      setStatus(`ROM load error: ${error}`);
    }
  });

  testToneCheckbox.addEventListener("change", () => {
    if (emulator) {
      emulator.set_audio_test_tone_enabled(testToneCheckbox.checked);
    }
  });

  audioResamplerSelect?.addEventListener("change", () => {
    applyAudioResamplerQuality();
    updateAudioTelemetry();
  });

  audioEnableButton.addEventListener("click", () => {
    void enableAudio();
  });

  window.addEventListener("keydown", (event) => setButtonFromKeyboardEvent(event, true));
  window.addEventListener("keyup", (event) => setButtonFromKeyboardEvent(event, false));
}

async function bootstrap() {
  bindDomEvents();
  await initWasm();
  wasmReady = true;
  setStatus("WASM ready. Load a ROM to start.");
  updateCartridgeInfoPanel();
  updateAudioTelemetry();

  if (rafId === 0) {
    rafId = requestAnimationFrame(stepFrame);
  }
}

bootstrap().catch((error) => {
  console.error(error);
  setStatus(`Bootstrap error: ${error}`);
});
