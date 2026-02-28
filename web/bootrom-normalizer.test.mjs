import test from "node:test";
import assert from "node:assert/strict";

import {
  canonicalBootRomFileNameToModel,
  classifyBootRomForWebHardware,
  expectedCanonicalBootRomFileNameForModel,
  isValidStoredBootRomForModel,
} from "./bootrom-normalizer.mjs";

test("normalizer maps model names to canonical boot ROM file names", () => {
  assert.equal(expectedCanonicalBootRomFileNameForModel("dmg"), "dmg_boot.bin");
  assert.equal(expectedCanonicalBootRomFileNameForModel("mgb"), "mgb_boot.bin");
  assert.equal(expectedCanonicalBootRomFileNameForModel("cgb"), null);
});

test("normalizer maps canonical DMG-family file names to web hardware models", () => {
  assert.equal(canonicalBootRomFileNameToModel("dmg0_boot.bin"), "dmg0");
  assert.equal(canonicalBootRomFileNameToModel("sgb2_boot.bin"), "sgb2");
  assert.equal(canonicalBootRomFileNameToModel("cgb_boot.bin"), null);
});

test("normalizer classifies supported, unsupported, and invalid boot ROM payloads", () => {
  const classify = (bytes) => {
    const marker = bytes[0];
    if (marker === 0xd0) {
      return "dmg_boot.bin";
    }
    if (marker === 0xc0) {
      return "cgb_boot.bin";
    }
    return null;
  };

  assert.deepEqual(
    classifyBootRomForWebHardware(new Uint8Array([0xd0]), classify),
    { kind: "supported", canonicalFileName: "dmg_boot.bin", model: "dmg" }
  );
  assert.deepEqual(
    classifyBootRomForWebHardware(new Uint8Array([0xc0]), classify),
    { kind: "known_unsupported", canonicalFileName: "cgb_boot.bin" }
  );
  assert.deepEqual(classifyBootRomForWebHardware(new Uint8Array([0x00]), classify), {
    kind: "invalid",
  });
});

test("normalizer validates a stored boot ROM against the selected model", () => {
  const classify = (bytes) => {
    if (bytes[0] === 0xd0) {
      return "dmg_boot.bin";
    }
    if (bytes[0] === 0xb0) {
      return "mgb_boot.bin";
    }
    return null;
  };

  assert.equal(
    isValidStoredBootRomForModel({
      model: "dmg",
      bootRomBytes: new Uint8Array(0x100).fill(0xd0),
      classifyBootRomFileName: classify,
    }),
    true
  );

  assert.equal(
    isValidStoredBootRomForModel({
      model: "mgb",
      bootRomBytes: new Uint8Array(0x100).fill(0xd0),
      classifyBootRomFileName: classify,
    }),
    false
  );

  assert.equal(
    isValidStoredBootRomForModel({
      model: "dmg",
      bootRomBytes: new Uint8Array(0x80).fill(0xd0),
      classifyBootRomFileName: classify,
    }),
    false
  );
});
