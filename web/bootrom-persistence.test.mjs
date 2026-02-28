import test from "node:test";
import assert from "node:assert/strict";

import {
  buildBootRomStorageKey,
  createWebBootRomPersistence,
} from "./bootrom-persistence.mjs";

test("buildBootRomStorageKey normalizes model values", () => {
  assert.equal(buildBootRomStorageKey("DMG"), "gb-emu:web:bootrom:v1:dmg");
  assert.equal(buildBootRomStorageKey("sgb2"), "gb-emu:web:bootrom:v1:sgb2");
});

test("boot ROM persistence stores and loads 256-byte prefix per model", () => {
  const storageMap = new Map();
  const storage = {
    getItem(key) {
      return storageMap.has(key) ? storageMap.get(key) : null;
    },
    setItem(key, value) {
      storageMap.set(key, value);
    },
    removeItem(key) {
      storageMap.delete(key);
    },
  };
  const persistence = createWebBootRomPersistence({ storage });
  const bootRom = new Uint8Array(0x120);
  for (let i = 0; i < bootRom.length; i += 1) {
    bootRom[i] = i & 0xff;
  }

  const result = persistence.storeBootRomBytesForModel("dmg", bootRom);
  assert.equal(result.ok, true);
  assert.equal(result.storedBytes, 0x100);
  assert.equal(persistence.hasBootRomForModel("dmg"), true);
  assert.equal(persistence.hasBootRomForModel("mgb"), false);

  const loaded = persistence.loadBootRomBytesForModel("dmg");
  assert.equal(loaded.length, 0x100);
  assert.equal(loaded[0], 0x00);
  assert.equal(loaded[0xff], 0xff);

  assert.equal(persistence.removeBootRomForModel("dmg"), true);
  assert.equal(persistence.hasBootRomForModel("dmg"), false);
});

test("boot ROM persistence rejects short payloads and storage write failures", () => {
  const failingStorage = {
    getItem() {
      return null;
    },
    setItem() {
      throw new Error("quota exceeded");
    },
    removeItem() {
      throw new Error("blocked");
    },
  };
  const persistence = createWebBootRomPersistence({ storage: failingStorage });

  const shortResult = persistence.storeBootRomBytesForModel("dmg", new Uint8Array(0x80));
  assert.equal(shortResult.ok, false);
  assert.equal(shortResult.error, "Boot ROM must contain at least 256 bytes.");

  const writeFailResult = persistence.storeBootRomBytesForModel("dmg", new Uint8Array(0x100));
  assert.equal(writeFailResult.ok, false);
  assert.equal(writeFailResult.error, "Failed to persist boot ROM in browser storage.");
  assert.equal(persistence.removeBootRomForModel("dmg"), false);
});
