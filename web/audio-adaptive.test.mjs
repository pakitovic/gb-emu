import { test } from "node:test";
import assert from "node:assert/strict";

import {
  createAdaptiveQueueState,
  updateAdaptiveQueueTarget,
} from "./audio-adaptive.mjs";

test("increases queue target when underruns appear in the window", () => {
  const state = createAdaptiveQueueState(0, 0);
  const options = {
    windowMs: 100,
    increaseStepSamples: 256,
  };

  const result = updateAdaptiveQueueTarget({
    state,
    nowMs: 100,
    queuedSamples: 2000,
    targetSamples: 4096,
    totalUnderrunSamples: 10,
    blockSamples: 512,
    options,
  });

  assert.equal(result.targetSamples, 4352);
  assert.equal(result.changed, true);
  assert.equal(result.windowUnderrunSamples, 10);
});

test("uses a larger increase step on severe underrun windows", () => {
  const state = createAdaptiveQueueState(0, 0);
  const options = {
    windowMs: 100,
    increaseStepSamples: 256,
  };

  const result = updateAdaptiveQueueTarget({
    state,
    nowMs: 100,
    queuedSamples: 1000,
    targetSamples: 4096,
    totalUnderrunSamples: 700,
    blockSamples: 512,
    options,
  });

  assert.equal(result.targetSamples, 4608);
  assert.equal(result.changed, true);
  assert.equal(result.windowUnderrunSamples, 700);
});

test("decreases queue target gradually after sustained stable windows", () => {
  const state = createAdaptiveQueueState(0, 0);
  const options = {
    windowMs: 100,
    decreaseStableWindows: 2,
    decreaseStepSamples: 128,
    decreaseQueueHeadroomSamples: 256,
  };

  let targetSamples = 4096;
  let result = updateAdaptiveQueueTarget({
    state,
    nowMs: 100,
    queuedSamples: 4600,
    targetSamples,
    totalUnderrunSamples: 0,
    blockSamples: 512,
    options,
  });
  assert.equal(result.targetSamples, 4096);

  result = updateAdaptiveQueueTarget({
    state,
    nowMs: 200,
    queuedSamples: 4600,
    targetSamples: result.targetSamples,
    totalUnderrunSamples: 0,
    blockSamples: 512,
    options,
  });
  targetSamples = result.targetSamples;
  assert.equal(targetSamples, 3968);

  result = updateAdaptiveQueueTarget({
    state,
    nowMs: 300,
    queuedSamples: 4400,
    targetSamples,
    totalUnderrunSamples: 0,
    blockSamples: 512,
    options,
  });
  assert.equal(result.targetSamples, 3968);

  result = updateAdaptiveQueueTarget({
    state,
    nowMs: 400,
    queuedSamples: 4400,
    targetSamples: result.targetSamples,
    totalUnderrunSamples: 0,
    blockSamples: 512,
    options,
  });
  assert.equal(result.targetSamples, 3840);
});

test("enforces min and max queue target limits", () => {
  const state = createAdaptiveQueueState(0, 0);
  const options = {
    windowMs: 100,
    minTargetSamples: 1024,
    maxTargetSamples: 2048,
    increaseStepSamples: 400,
    decreaseStepSamples: 800,
    decreaseStableWindows: 1,
    decreaseQueueHeadroomSamples: 1,
  };

  let result = updateAdaptiveQueueTarget({
    state,
    nowMs: 100,
    queuedSamples: 100,
    targetSamples: 1900,
    totalUnderrunSamples: 2000,
    blockSamples: 512,
    options,
  });
  assert.equal(result.targetSamples, 2048);

  result = updateAdaptiveQueueTarget({
    state,
    nowMs: 200,
    queuedSamples: 5000,
    targetSamples: result.targetSamples,
    totalUnderrunSamples: 2000,
    blockSamples: 512,
    options,
  });
  assert.equal(result.targetSamples, 1248);

  result = updateAdaptiveQueueTarget({
    state,
    nowMs: 300,
    queuedSamples: 5000,
    targetSamples: result.targetSamples,
    totalUnderrunSamples: 2000,
    blockSamples: 512,
    options,
  });
  assert.equal(result.targetSamples, 1024);
});
