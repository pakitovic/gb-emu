import test from "node:test";
import assert from "node:assert/strict";

import {
  batteryPowerOnForState,
  canvasPromptForState,
  CONTROL_SECTIONS,
  nextControlPanelState,
  normalizeControlSection,
  normalizeScreenScale,
  shouldCloseSettingsPanelOnPointerDown,
} from "./ui.mjs";

test("normalizeControlSection accepts known sections", () => {
  for (const section of CONTROL_SECTIONS) {
    assert.equal(normalizeControlSection(section), section);
  }
});

test("normalizeControlSection normalizes case and whitespace", () => {
  assert.equal(normalizeControlSection(" DATA "), "data");
  assert.equal(normalizeControlSection("System"), "system");
  assert.equal(normalizeControlSection("aUdIo"), "audio");
});

test("normalizeControlSection falls back to data", () => {
  assert.equal(normalizeControlSection(""), "data");
  assert.equal(normalizeControlSection("unknown"), "data");
  assert.equal(normalizeControlSection(null), "data");
  assert.equal(normalizeControlSection(undefined), "data");
});

test("canvasPromptForState keeps only the no-rom prompt", () => {
  assert.deepEqual(canvasPromptForState({ hasRom: false, isRunning: false }), {
    text: "Load ROM",
    clickable: true,
  });
  assert.deepEqual(canvasPromptForState({ hasRom: true, isRunning: false }), {
    text: null,
    clickable: false,
  });
});

test("canvasPromptForState disables prompt while running", () => {
  assert.deepEqual(canvasPromptForState({ hasRom: true, isRunning: true }), {
    text: null,
    clickable: false,
  });
});

test("nextControlPanelState opens requested section when panel is collapsed", () => {
  assert.deepEqual(
    nextControlPanelState({ currentSection: "data", isOpen: false, requestedSection: "audio" }),
    { section: "audio", isOpen: true }
  );
});

test("nextControlPanelState collapses panel when current section is requested again", () => {
  assert.deepEqual(
    nextControlPanelState({ currentSection: "system", isOpen: true, requestedSection: "system" }),
    { section: "system", isOpen: false }
  );
});

test("nextControlPanelState switches section and keeps panel open", () => {
  assert.deepEqual(
    nextControlPanelState({ currentSection: "system", isOpen: true, requestedSection: "debug" }),
    { section: "debug", isOpen: true }
  );
});

test("normalizeScreenScale keeps valid x1..x4 values", () => {
  assert.equal(normalizeScreenScale({ scale: 1 }), 1);
  assert.equal(normalizeScreenScale({ scale: "2" }), 2);
  assert.equal(normalizeScreenScale({ scale: 3 }), 3);
  assert.equal(normalizeScreenScale({ scale: 4 }), 4);
});

test("normalizeScreenScale clamps to x1..x4 bounds", () => {
  assert.equal(normalizeScreenScale({ scale: 0 }), 1);
  assert.equal(normalizeScreenScale({ scale: -10 }), 1);
  assert.equal(normalizeScreenScale({ scale: 99 }), 4);
});

test("normalizeScreenScale defaults to x4 for invalid values", () => {
  assert.equal(normalizeScreenScale({ scale: undefined }), 4);
  assert.equal(normalizeScreenScale({ scale: null }), 4);
  assert.equal(normalizeScreenScale({ scale: "foo" }), 4);
});

test("shouldCloseSettingsPanelOnPointerDown closes only on outside click while open", () => {
  assert.equal(
    shouldCloseSettingsPanelOnPointerDown({
      isPanelOpen: true,
      isInsidePanel: false,
      isInsideToggle: false,
    }),
    true
  );
  assert.equal(
    shouldCloseSettingsPanelOnPointerDown({
      isPanelOpen: true,
      isInsidePanel: true,
      isInsideToggle: false,
    }),
    false
  );
  assert.equal(
    shouldCloseSettingsPanelOnPointerDown({
      isPanelOpen: true,
      isInsidePanel: false,
      isInsideToggle: true,
    }),
    false
  );
  assert.equal(
    shouldCloseSettingsPanelOnPointerDown({
      isPanelOpen: false,
      isInsidePanel: false,
      isInsideToggle: false,
    }),
    false
  );
});

test("batteryPowerOnForState turns on only when a ROM is loaded", () => {
  assert.equal(batteryPowerOnForState({ hasRom: true }), true);
  assert.equal(batteryPowerOnForState({ hasRom: false }), false);
  assert.equal(batteryPowerOnForState({ hasRom: undefined }), false);
});
