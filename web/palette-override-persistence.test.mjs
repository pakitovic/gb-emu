import test from "node:test";
import assert from "node:assert/strict";

import { createWebPaletteOverridePersistence } from "./palette-override-persistence.mjs";

test("palette override persistence stores and loads named INI state", () => {
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
  const persistence = createWebPaletteOverridePersistence({ storage });

  const stored = persistence.storePaletteOverrideState({
    name: "kirby.ini",
    text: "[gb.override.302017CC]\npal[0]=0x112233\n",
  });
  assert.equal(stored.ok, true);
  assert.equal(stored.name, "kirby.ini");

  assert.deepEqual(persistence.loadPaletteOverrideState(), {
    name: "kirby.ini",
    text: "[gb.override.302017CC]\npal[0]=0x112233\n",
  });
  assert.equal(persistence.removePaletteOverrideState(), true);
  assert.equal(persistence.loadPaletteOverrideState(), null);
});

test("palette override persistence defaults empty names and rejects empty text", () => {
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
  const persistence = createWebPaletteOverridePersistence({ storage });

  const invalid = persistence.storePaletteOverrideState({ name: "x", text: "   " });
  assert.equal(invalid.ok, false);
  assert.equal(invalid.error, "Palette override text must be a non-empty string.");

  const stored = persistence.storePaletteOverrideState({
    name: " ",
    text: "[gb.override.12345678]\npal[0]=0xABCDEF\n",
  });
  assert.equal(stored.ok, true);
  assert.equal(stored.name, "overrides.ini");
  assert.deepEqual(persistence.loadPaletteOverrideState(), {
    name: "overrides.ini",
    text: "[gb.override.12345678]\npal[0]=0xABCDEF\n",
  });
});

test("palette override persistence reports storage write and remove failures", () => {
  const storage = {
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
  const persistence = createWebPaletteOverridePersistence({ storage });

  const stored = persistence.storePaletteOverrideState({
    name: "test.ini",
    text: "[gb.override.12345678]\npal[0]=0xABCDEF\n",
  });
  assert.equal(stored.ok, false);
  assert.equal(stored.error, "Failed to persist palette overrides in browser storage.");
  assert.equal(persistence.removePaletteOverrideState(), false);
});
