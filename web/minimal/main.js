import initWasm, { WebEmulator } from "./pkg/gb_emu.js";

const SCREEN_WIDTH = 160;
const SCREEN_HEIGHT = 144;
const AUDIO_BLOCK_SAMPLES = 512;
const AUDIO_QUEUE_TARGET_SAMPLES = 4096;
const AUDIO_REFILL_INTERVAL_MS = 8;

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
const serialPre = document.getElementById("serial");
const testToneCheckbox = document.getElementById("test-tone");
const audioEnableButton = document.getElementById("audio-enable");

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

function setStatus(message) {
  statusLabel.textContent = message;
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

function stepFrame(nowMs) {
  if (!emulator) {
    rafId = requestAnimationFrame(stepFrame);
    return;
  }

  const elapsedMs = lastFrameTimeMs > 0 ? nowMs - lastFrameTimeMs : 16.0;
  lastFrameTimeMs = nowMs;
  const clampedMs = Math.max(0, Math.min(elapsedMs, 100));

  try {
    emulator.run_for_elapsed_micros(Math.floor(clampedMs * 1000));
    drawFrame();
    serialPre.textContent = emulator.serial_output();
  } catch (error) {
    console.error(error);
    setStatus(`Runtime error: ${error}`);
  }

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
  queuedAudioSamples = 0;
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
    emulator.set_audio_test_tone_enabled(testToneCheckbox.checked);

    disconnectAudioBackend();
    if (!audioWorkletLoaded) {
      await ac.audioWorklet.addModule("./audio-worklet.js");
      audioWorkletLoaded = true;
    }
    audioNode = new AudioWorkletNode(ac, "gb-audio-processor", {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [1],
      channelCount: 1,
      channelCountMode: "explicit",
      channelInterpretation: "speakers",
    });
    audioNode.port.onmessage = (event) => {
      const data = event.data;
      if (!data || data.type !== "consumed") {
        return;
      }
      queuedAudioSamples = Math.max(0, queuedAudioSamples - (data.samples | 0));
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
  } catch (error) {
    console.error(error);
    setStatus(`Audio setup error: ${error}`);
  }
}

function refillAudioQueue() {
  if (!emulator || !audioNode) {
    return;
  }

  let guard = 0;
  while (queuedAudioSamples < AUDIO_QUEUE_TARGET_SAMPLES && guard < 16) {
    const samples = emulator.drain_audio_samples_realtime(AUDIO_BLOCK_SAMPLES);
    if (!samples || samples.length === 0) {
      break;
    }
    audioNode.port.postMessage({ type: "samples", samples });
    queuedAudioSamples += samples.length;
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
  emulator.set_audio_test_tone_enabled(testToneCheckbox.checked);

  if (audioContext) {
    emulator.set_audio_sample_rate(audioContext.sampleRate);
  }
  if (audioNode) {
    audioNode.port.postMessage({ type: "reset" });
    queuedAudioSamples = 0;
    refillAudioQueue();
  }

  drawFrame();
  serialPre.textContent = "";
  setStatus(`Loaded ${file.name} (${emulator.rom_title()}) on model ${model}.`);
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

  if (rafId === 0) {
    rafId = requestAnimationFrame(stepFrame);
  }
}

bootstrap().catch((error) => {
  console.error(error);
  setStatus(`Bootstrap error: ${error}`);
});
