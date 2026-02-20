export const DEFAULT_ADAPTIVE_QUEUE_OPTIONS = Object.freeze({
  windowMs: 500,
  minTargetSamples: 2048,
  maxTargetSamples: 16384,
  increaseStepSamples: 1024,
  decreaseStepSamples: 512,
  decreaseStableWindows: 6,
  decreaseQueueHeadroomSamples: 1024,
});

function toNonNegativeInt(value, fallback = 0) {
  if (!Number.isFinite(value)) {
    return fallback;
  }
  return Math.max(0, Math.floor(value));
}

function normalizeOptions(options) {
  const merged = { ...DEFAULT_ADAPTIVE_QUEUE_OPTIONS, ...(options || {}) };
  const minTargetSamples = Math.max(1, toNonNegativeInt(merged.minTargetSamples, 2048));
  const maxTargetSamples = Math.max(minTargetSamples, toNonNegativeInt(merged.maxTargetSamples, 16384));

  return {
    windowMs: Math.max(1, toNonNegativeInt(merged.windowMs, 500)),
    minTargetSamples,
    maxTargetSamples,
    increaseStepSamples: Math.max(1, toNonNegativeInt(merged.increaseStepSamples, 1024)),
    decreaseStepSamples: Math.max(1, toNonNegativeInt(merged.decreaseStepSamples, 512)),
    decreaseStableWindows: Math.max(1, toNonNegativeInt(merged.decreaseStableWindows, 6)),
    decreaseQueueHeadroomSamples: Math.max(0, toNonNegativeInt(merged.decreaseQueueHeadroomSamples, 1024)),
  };
}

function clampTargetSamples(targetSamples, options) {
  const normalizedTarget = toNonNegativeInt(targetSamples, options.minTargetSamples);
  return Math.min(options.maxTargetSamples, Math.max(options.minTargetSamples, normalizedTarget));
}

export function createAdaptiveQueueState(nowMs = 0, totalUnderrunSamples = 0) {
  return {
    lastWindowMs: Number.isFinite(nowMs) ? nowMs : 0,
    lastUnderrunSamples: toNonNegativeInt(totalUnderrunSamples, 0),
    stableWindowCount: 0,
  };
}

export function updateAdaptiveQueueTarget({
  state,
  nowMs,
  queuedSamples,
  targetSamples,
  totalUnderrunSamples,
  blockSamples,
  options,
}) {
  if (!state || typeof state !== "object") {
    throw new TypeError("state is required");
  }

  const tunedOptions = normalizeOptions(options);
  const currentNowMs = Number.isFinite(nowMs) ? nowMs : 0;
  const currentQueuedSamples = toNonNegativeInt(queuedSamples, 0);
  const currentTargetSamples = clampTargetSamples(targetSamples, tunedOptions);
  const currentUnderrunSamples = toNonNegativeInt(totalUnderrunSamples, 0);
  const currentBlockSamples = Math.max(1, toNonNegativeInt(blockSamples, 1));

  if (!Number.isFinite(state.lastWindowMs)) {
    state.lastWindowMs = currentNowMs;
  }
  if (!Number.isFinite(state.lastUnderrunSamples)) {
    state.lastUnderrunSamples = currentUnderrunSamples;
  }
  if (!Number.isFinite(state.stableWindowCount)) {
    state.stableWindowCount = 0;
  }

  const elapsedMs = currentNowMs - state.lastWindowMs;
  if (elapsedMs < tunedOptions.windowMs) {
    return {
      targetSamples: currentTargetSamples,
      changed: false,
      windowUnderrunSamples: 0,
    };
  }

  const windowUnderrunSamples = Math.max(0, currentUnderrunSamples - state.lastUnderrunSamples);
  let nextTargetSamples = currentTargetSamples;

  if (windowUnderrunSamples > 0) {
    const severeUnderrun = windowUnderrunSamples >= currentBlockSamples;
    const increaseStepSamples = severeUnderrun
      ? tunedOptions.increaseStepSamples * 2
      : tunedOptions.increaseStepSamples;
    nextTargetSamples = clampTargetSamples(currentTargetSamples + increaseStepSamples, tunedOptions);
    state.stableWindowCount = 0;
  } else {
    const queueHeadroomSamples = currentQueuedSamples - currentTargetSamples;
    if (queueHeadroomSamples >= tunedOptions.decreaseQueueHeadroomSamples) {
      state.stableWindowCount += 1;
    } else {
      state.stableWindowCount = 0;
    }

    if (state.stableWindowCount >= tunedOptions.decreaseStableWindows) {
      nextTargetSamples = clampTargetSamples(currentTargetSamples - tunedOptions.decreaseStepSamples, tunedOptions);
      state.stableWindowCount = 0;
    }
  }

  state.lastWindowMs = currentNowMs;
  state.lastUnderrunSamples = currentUnderrunSamples;

  return {
    targetSamples: nextTargetSamples,
    changed: nextTargetSamples !== currentTargetSamples,
    windowUnderrunSamples,
  };
}
