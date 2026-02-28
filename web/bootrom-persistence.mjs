import { base64ToBytes, bytesToBase64 } from "./save-persistence.mjs";

const STORAGE_KEY_PREFIX = "gb-emu:web:bootrom:v1";
const BOOT_ROM_WINDOW_SIZE = 0x100;

export function buildBootRomStorageKey(model) {
  const normalizedModel = String(model || "dmg").toLowerCase();
  return `${STORAGE_KEY_PREFIX}:${normalizedModel}`;
}

export function createWebBootRomPersistence({
  storage = globalThis.localStorage,
} = {}) {
  function loadBootRomBytesForModel(model) {
    if (!storage) {
      return null;
    }
    const base64 = safeStorageGet(storage, buildBootRomStorageKey(model));
    if (!base64) {
      return null;
    }

    let bytes;
    try {
      bytes = base64ToBytes(base64);
    } catch {
      return null;
    }
    if (bytes.length < BOOT_ROM_WINDOW_SIZE) {
      return null;
    }
    return bytes.slice(0, BOOT_ROM_WINDOW_SIZE);
  }

  function hasBootRomForModel(model) {
    return loadBootRomBytesForModel(model) !== null;
  }

  function storeBootRomBytesForModel(model, bytes) {
    if (!storage) {
      return { ok: false, error: "Browser storage unavailable." };
    }
    if (!(bytes instanceof Uint8Array) || bytes.length < BOOT_ROM_WINDOW_SIZE) {
      return {
        ok: false,
        error: "Boot ROM must contain at least 256 bytes.",
      };
    }

    const bootPrefix = bytes.slice(0, BOOT_ROM_WINDOW_SIZE);
    const key = buildBootRomStorageKey(model);
    const saved = safeStorageSet(storage, key, bytesToBase64(bootPrefix));
    if (!saved) {
      return { ok: false, error: "Failed to persist boot ROM in browser storage." };
    }

    return { ok: true, storedBytes: bootPrefix.length };
  }

  function removeBootRomForModel(model) {
    if (!storage) {
      return false;
    }
    return safeStorageRemove(storage, buildBootRomStorageKey(model));
  }

  return {
    loadBootRomBytesForModel,
    hasBootRomForModel,
    storeBootRomBytesForModel,
    removeBootRomForModel,
  };
}

function safeStorageGet(storage, key) {
  try {
    return storage.getItem(key);
  } catch {
    return null;
  }
}

function safeStorageSet(storage, key, value) {
  try {
    storage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}

function safeStorageRemove(storage, key) {
  try {
    storage.removeItem(key);
    return true;
  } catch {
    return false;
  }
}
