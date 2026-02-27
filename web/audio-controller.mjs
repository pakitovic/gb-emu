import {
  createAdaptiveQueueState,
  DEFAULT_ADAPTIVE_QUEUE_OPTIONS,
  updateAdaptiveQueueTarget,
} from "./audio-adaptive.mjs";

const AUDIO_BLOCK_SAMPLES = 512;
const AUDIO_CHANNELS = 2;
const AUDIO_QUEUE_TARGET_INITIAL_SAMPLES = 4096;
const AUDIO_REFILL_INTERVAL_MS = 8;
const AUDIO_ADAPTIVE_QUEUE_OPTIONS = {
  ...DEFAULT_ADAPTIVE_QUEUE_OPTIONS,
};

export function createWebAudioController({
  getEmulator,
  getResamplerQuality,
  getTestToneEnabled,
  setStatus,
  setAudioTelemetryText,
}) {
  let audioContext = null;
  let audioNode = null;
  let audioRefillTimerId = null;
  let queuedAudioSamples = 0;
  let audioWorkletLoaded = false;
  let audioConsumedSamplesTotal = 0;
  let audioUnderrunSamplesTotal = 0;
  let audioQueueTargetSamples = AUDIO_QUEUE_TARGET_INITIAL_SAMPLES;
  let audioAdaptiveQueueState = createAdaptiveQueueState();

  function resetAudioTelemetryState() {
    queuedAudioSamples = 0;
    audioConsumedSamplesTotal = 0;
    audioUnderrunSamplesTotal = 0;
    audioQueueTargetSamples = AUDIO_QUEUE_TARGET_INITIAL_SAMPLES;
    audioAdaptiveQueueState = createAdaptiveQueueState(performance.now(), 0);
  }

  function updateTelemetry() {
    const emulator = getEmulator?.() ?? null;
    const fallbackQuality = getResamplerQuality?.() || "cubic";
    const resamplerQuality =
      emulator && typeof emulator.audio_resampler_quality === "function"
        ? emulator.audio_resampler_quality()
        : fallbackQuality;

    if (!audioContext || !audioNode) {
      setAudioTelemetryText?.(`Audio: disabled | resampler ${resamplerQuality}`);
      return;
    }

    const sampleRate = Math.max(1, audioContext.sampleRate || 48_000);
    const queuedMs = (queuedAudioSamples * 1000) / sampleRate;
    const targetMs = (audioQueueTargetSamples * 1000) / sampleRate;
    const underrunMs = (audioUnderrunSamplesTotal * 1000) / sampleRate;
    const playedSeconds = audioConsumedSamplesTotal / sampleRate;
    setAudioTelemetryText?.(
      `Audio: ${audioContext.state} | resampler ${resamplerQuality} | queued ${queuedMs.toFixed(1)}ms / target ${targetMs.toFixed(1)}ms | ` +
        `underruns ${audioUnderrunSamplesTotal} samples (${underrunMs.toFixed(2)}ms) | played ${playedSeconds.toFixed(1)}s`
    );
  }

  function applyResamplerQuality() {
    const emulator = getEmulator?.() ?? null;
    if (!emulator) {
      return;
    }

    const quality = getResamplerQuality?.() || "cubic";
    try {
      emulator.set_audio_resampler_quality(quality);
    } catch (error) {
      console.error(error);
      setStatus?.(`Audio resampler error: ${error}`);
    }
  }

  function applyTestToneState() {
    const emulator = getEmulator?.() ?? null;
    if (!emulator) {
      return;
    }
    emulator.set_audio_test_tone_enabled(Boolean(getTestToneEnabled?.()));
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

  function refillAudioQueue() {
    const emulator = getEmulator?.() ?? null;
    if (!emulator || !audioNode) {
      return;
    }

    maybeAdjustAudioQueueTarget(performance.now());

    let guard = 0;
    while (queuedAudioSamples < audioQueueTargetSamples && guard < 16) {
      const samples = emulator.drain_audio_samples(AUDIO_BLOCK_SAMPLES);
      if (!samples || samples.length === 0) {
        break;
      }
      audioNode.port.postMessage({ type: "samples", samples });
      const enqueuedFrames = Math.floor(samples.length / AUDIO_CHANNELS);
      queuedAudioSamples += enqueuedFrames;
      guard += 1;
    }
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
    updateTelemetry();
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

  async function enable() {
    const emulator = getEmulator?.() ?? null;
    if (!emulator) {
      setStatus?.("Load a ROM before enabling audio.");
      return;
    }

    try {
      const ac = await ensureAudioContext();
      if (!ac.audioWorklet || typeof ac.audioWorklet.addModule !== "function") {
        throw new Error("AudioWorklet is not available in this browser");
      }
      emulator.set_audio_sample_rate(ac.sampleRate);
      applyResamplerQuality();
      applyTestToneState();

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

      setStatus?.(`AudioWorklet enabled (${ac.sampleRate} Hz, block ${AUDIO_BLOCK_SAMPLES}).`);
      updateTelemetry();
    } catch (error) {
      console.error(error);
      setStatus?.(`Audio setup error: ${error}`);
      updateTelemetry();
    }
  }

  async function disable() {
    disconnectAudioBackend();
    if (!audioContext) {
      setStatus?.("Audio disabled.");
      updateTelemetry();
      return;
    }

    try {
      if (audioContext.state !== "closed") {
        await audioContext.suspend();
      }
      setStatus?.("Audio disabled.");
    } catch (error) {
      console.error(error);
      setStatus?.(`Audio disable error: ${error}`);
    }
    updateTelemetry();
  }

  async function toggle() {
    if (audioNode) {
      await disable();
    } else {
      await enable();
    }
    return Boolean(audioNode);
  }

  function isEnabled() {
    return Boolean(audioNode);
  }

  function onEmulatorLoaded() {
    const emulator = getEmulator?.() ?? null;
    if (!emulator) {
      updateTelemetry();
      return;
    }

    applyResamplerQuality();
    applyTestToneState();

    if (audioContext) {
      emulator.set_audio_sample_rate(audioContext.sampleRate);
    }
    if (audioNode) {
      audioNode.port.postMessage({ type: "reset" });
      resetAudioTelemetryState();
      refillAudioQueue();
    }

    updateTelemetry();
  }

  function handleResamplerChanged() {
    applyResamplerQuality();
    updateTelemetry();
  }

  function handleTestToneChanged() {
    applyTestToneState();
  }

  return {
    enable,
    disable,
    toggle,
    isEnabled,
    updateTelemetry,
    onEmulatorLoaded,
    handleResamplerChanged,
    handleTestToneChanged,
  };
}
