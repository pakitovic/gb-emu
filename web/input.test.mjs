import test from "node:test";
import assert from "node:assert/strict";

import { bindKeyboardInput } from "./input.mjs";

test("keyboard mapping uses player 1 keys when only set_button is available", () => {
  const target = createMockTarget();
  const calls = [];
  const emulator = {
    set_button(index, pressed) {
      calls.push({ index, pressed });
    },
  };
  bindKeyboardInput({
    target,
    getEmulator: () => emulator,
  });

  const zKeyDownEvent = keyEvent("KeyZ");
  const zKeyUpEvent = keyEvent("KeyZ");
  const xKeyDownEvent = keyEvent("KeyX");
  const xKeyUpEvent = keyEvent("KeyX");

  target.dispatch("keydown", zKeyDownEvent);
  target.dispatch("keyup", zKeyUpEvent);
  target.dispatch("keydown", xKeyDownEvent);
  target.dispatch("keyup", xKeyUpEvent);

  assert.deepEqual(calls, [
    { index: 5, pressed: true },
    { index: 5, pressed: false },
    { index: 4, pressed: true },
    { index: 4, pressed: false },
  ]);
  assert.equal(zKeyDownEvent.defaultPrevented, true);
  assert.equal(zKeyUpEvent.defaultPrevented, true);
  assert.equal(xKeyDownEvent.defaultPrevented, true);
  assert.equal(xKeyUpEvent.defaultPrevented, true);
});

test("keyboard mapping routes player 2 keys through set_player_button", () => {
  const target = createMockTarget();
  const calls = [];
  const emulator = {
    set_player_button(player, index, pressed) {
      calls.push({ player, index, pressed });
    },
  };
  bindKeyboardInput({
    target,
    getEmulator: () => emulator,
  });

  const rightDownEvent = keyEvent("KeyD");
  const aDownEvent = keyEvent("KeyG");
  target.dispatch("keydown", rightDownEvent);
  target.dispatch("keydown", aDownEvent);

  assert.deepEqual(calls, [
    { player: 1, index: 0, pressed: true },
    { player: 1, index: 4, pressed: true },
  ]);
  assert.equal(rightDownEvent.defaultPrevented, true);
  assert.equal(aDownEvent.defaultPrevented, true);
});

test("legacy web bindings ignore non-player-1 keys when set_player_button is unavailable", () => {
  const target = createMockTarget();
  const calls = [];
  const emulator = {
    set_button(index, pressed) {
      calls.push({ index, pressed });
    },
  };
  bindKeyboardInput({
    target,
    getEmulator: () => emulator,
  });

  const playerTwoEvent = keyEvent("KeyD");
  target.dispatch("keydown", playerTwoEvent);

  assert.deepEqual(calls, []);
  assert.equal(playerTwoEvent.defaultPrevented, true);
});

test("keyboard handler ignores unmapped keys", () => {
  const target = createMockTarget();
  const calls = [];
  const emulator = {
    set_button(index, pressed) {
      calls.push({ index, pressed });
    },
  };
  bindKeyboardInput({
    target,
    getEmulator: () => emulator,
  });

  const spaceDownEvent = keyEvent("Space");
  target.dispatch("keydown", spaceDownEvent);

  assert.deepEqual(calls, []);
  assert.equal(spaceDownEvent.defaultPrevented, false);
});

function createMockTarget() {
  const listeners = new Map();

  return {
    addEventListener(type, handler) {
      listeners.set(type, handler);
    },
    removeEventListener(type, handler) {
      if (listeners.get(type) === handler) {
        listeners.delete(type);
      }
    },
    dispatch(type, event) {
      listeners.get(type)?.(event);
    },
  };
}

function keyEvent(code) {
  return {
    code,
    defaultPrevented: false,
    preventDefault() {
      this.defaultPrevented = true;
    },
  };
}
