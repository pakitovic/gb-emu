import test from "node:test";
import assert from "node:assert/strict";

import {
  SaveAutosaveDebouncer,
  base64ToBytes,
  buildRomPersistenceKey,
  bytesToBase64,
  createWebSavePersistence,
} from "./save-persistence.mjs";

test("SaveAutosaveDebouncer flushes only after debounce window", () => {
  const debouncer = new SaveAutosaveDebouncer(2000);

  assert.equal(debouncer.updateAndShouldFlush(true, 1000), false);
  assert.equal(debouncer.updateAndShouldFlush(true, 2999), false);
  assert.equal(debouncer.updateAndShouldFlush(true, 3000), true);
});

test("SaveAutosaveDebouncer resets when save becomes clean", () => {
  const debouncer = new SaveAutosaveDebouncer(2000);

  assert.equal(debouncer.updateAndShouldFlush(true, 1000), false);
  assert.equal(debouncer.updateAndShouldFlush(false, 1500), false);
  assert.equal(debouncer.updateAndShouldFlush(true, 1501), false);
  assert.equal(debouncer.updateAndShouldFlush(true, 3499), false);
  assert.equal(debouncer.updateAndShouldFlush(true, 3501), true);
});

test("base64 helpers roundtrip Uint8Array payloads", () => {
  const payload = new Uint8Array([0, 1, 2, 3, 127, 128, 254, 255]);
  const encoded = bytesToBase64(payload);
  const decoded = base64ToBytes(encoded);

  assert.deepEqual(Array.from(decoded), Array.from(payload));
});

test("rom persistence key is stable and includes sanitized file name", () => {
  const key = buildRomPersistenceKey({
    fileName: "Legend: Zelda.gb",
    romBytes: new Uint8Array([1, 2, 3, 4]),
  });

  assert.match(key, /^gb-emu:web:persistence:v1:Legend_ Zelda\.gb:4:[0-9a-f]{8}$/);
});

test("web persistence loads existing save, debounces dirty flush, and stores rtc", () => {
  let nowMs = 0;
  const storageMap = new Map();
  const storage = {
    getItem(key) {
      return storageMap.has(key) ? storageMap.get(key) : null;
    },
    setItem(key, value) {
      storageMap.set(key, value);
    },
  };
  const persistence = createWebSavePersistence({
    storage,
    debounceMs: 2000,
    now: () => nowMs,
  });
  const romBytes = new Uint8Array([1, 2, 3, 4]);
  const fileName = "test.gb";
  const romKey = buildRomPersistenceKey({ fileName, romBytes });
  storage.setItem(`${romKey}:sav`, bytesToBase64(new Uint8Array([9, 8, 7])));
  storage.setItem(`${romKey}:rtc`, bytesToBase64(new Uint8Array([6, 5])));

  const emulator = makeMockEmulator();
  persistence.attachRom({ romBytes, fileName, nextEmulator: emulator });

  assert.deepEqual(emulator.loadedSave, [9, 8, 7]);
  assert.deepEqual(emulator.loadedRtc, [6, 5]);
  assert.equal(emulator.markCleanCalls, 1);

  emulator.dirty = true;
  nowMs = 1000;
  assert.equal(persistence.tick(), false);
  nowMs = 3001;
  assert.equal(persistence.tick(), true);

  assert.equal(emulator.markCleanCalls, 2);
  assert.equal(storage.getItem(`${romKey}:sav`), bytesToBase64(new Uint8Array([1, 2, 3])));
  assert.equal(storage.getItem(`${romKey}:rtc`), bytesToBase64(new Uint8Array([4, 5])));
});

test("web persistence keeps dirty state when storage writes fail", () => {
  let nowMs = 0;
  const storage = {
    getItem() {
      return null;
    },
    setItem() {
      throw new Error("quota exceeded");
    },
  };
  const persistence = createWebSavePersistence({
    storage,
    debounceMs: 1,
    now: () => nowMs,
  });
  const emulator = makeMockEmulator();
  persistence.attachRom({
    romBytes: new Uint8Array([1]),
    fileName: "fail.gb",
    nextEmulator: emulator,
  });
  emulator.dirty = true;

  nowMs = 10;
  assert.equal(persistence.tick(), false);
  nowMs = 12;
  assert.equal(persistence.tick(), false);
  assert.equal(emulator.markCleanCalls, 1, "only initial load should mark clean");
  assert.equal(emulator.dirty, true);
});

test("web persistence supports manual SAV import/export with ROM-specific filename", () => {
  const storageMap = new Map();
  const storage = {
    getItem(key) {
      return storageMap.has(key) ? storageMap.get(key) : null;
    },
    setItem(key, value) {
      storageMap.set(key, value);
    },
  };
  const persistence = createWebSavePersistence({ storage });
  const emulator = makeMockEmulator();
  const romBytes = new Uint8Array([1, 2, 3]);
  const fileName = "Legend of Zelda.gb";
  const romKey = buildRomPersistenceKey({ fileName, romBytes });

  persistence.attachRom({ romBytes, fileName, nextEmulator: emulator });

  const imported = persistence.importSavBytes(new Uint8Array([0xaa, 0xbb]));
  assert.equal(imported, true);
  assert.deepEqual(emulator.loadedSave, [0xaa, 0xbb]);
  assert.equal(storage.getItem(`${romKey}:sav`), bytesToBase64(new Uint8Array([0xaa, 0xbb])));
  assert.equal(persistence.exportSavFileName(), "Legend of Zelda.sav");
  assert.deepEqual(Array.from(persistence.exportSavBytes()), [0xaa, 0xbb]);
});

test("web persistence supports manual RTC import/export and rejects invalid RTC blobs", () => {
  const storageMap = new Map();
  const storage = {
    getItem(key) {
      return storageMap.has(key) ? storageMap.get(key) : null;
    },
    setItem(key, value) {
      storageMap.set(key, value);
    },
  };
  const persistence = createWebSavePersistence({ storage });
  const emulator = makeMockEmulator();
  persistence.attachRom({
    romBytes: new Uint8Array([7, 8, 9]),
    fileName: "Pokemon Gold.gb",
    nextEmulator: emulator,
  });

  emulator.acceptRtcImport = false;
  assert.equal(persistence.importRtcBytes(new Uint8Array([1, 2])), false);

  emulator.acceptRtcImport = true;
  assert.equal(persistence.importRtcBytes(new Uint8Array([3, 4, 5])), true);
  assert.deepEqual(emulator.loadedRtc, [3, 4, 5]);
  assert.equal(persistence.exportRtcFileName(), "Pokemon Gold.rtc");
  assert.deepEqual(Array.from(persistence.exportRtcBytes()), [3, 4, 5]);
});

function makeMockEmulator() {
  return {
    dirty: false,
    loadedSave: null,
    loadedRtc: null,
    markCleanCalls: 0,
    acceptRtcImport: true,
    cartridge_battery_save_dirty() {
      return this.dirty;
    },
    export_cartridge_save_ram_bytes() {
      return new Uint8Array([1, 2, 3]);
    },
    import_cartridge_save_ram_bytes(bytes) {
      this.loadedSave = Array.from(bytes);
    },
    export_cartridge_rtc_persistence_bytes() {
      return new Uint8Array([4, 5]);
    },
    import_cartridge_rtc_persistence_bytes(bytes) {
      if (!this.acceptRtcImport) {
        return false;
      }
      this.loadedRtc = Array.from(bytes);
      return true;
    },
    mark_cartridge_persistence_clean() {
      this.dirty = false;
      this.markCleanCalls += 1;
    },
  };
}
