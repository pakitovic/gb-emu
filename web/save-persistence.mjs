const STORAGE_KEY_PREFIX = "gb-emu:web:persistence:v1";

export class SaveAutosaveDebouncer {
  constructor(debounceMs) {
    this.debounceMs = Math.max(0, Number.isFinite(debounceMs) ? debounceMs : 0);
    this.dirtySinceMs = null;
  }

  updateAndShouldFlush(isDirty, nowMs) {
    if (!isDirty) {
      this.dirtySinceMs = null;
      return false;
    }

    if (this.dirtySinceMs === null) {
      this.dirtySinceMs = nowMs;
      return false;
    }

    return nowMs - this.dirtySinceMs >= this.debounceMs;
  }

  markFlushed() {
    this.dirtySinceMs = null;
  }
}

export function buildRomPersistenceKey({ fileName, romBytes }) {
  const romHashHex = fnv1a32Hex(romBytes);
  return `${STORAGE_KEY_PREFIX}:${sanitizeKeyPart(fileName)}:${romBytes.length}:${romHashHex}`;
}

export function createWebSavePersistence({
  storage = globalThis.localStorage,
  debounceMs = 2000,
  now = () => globalThis.performance?.now?.() ?? Date.now(),
} = {}) {
  const debouncer = new SaveAutosaveDebouncer(debounceMs);
  let emulator = null;
  let romKey = null;
  let romFileName = null;

  function loadForCurrentRom() {
    if (!storage || !emulator || !romKey) {
      return;
    }

    const saveBase64 = safeStorageGet(storage, `${romKey}:sav`);
    if (saveBase64) {
      emulator.import_cartridge_save_ram_bytes(base64ToBytes(saveBase64));
    }

    const rtcBase64 = safeStorageGet(storage, `${romKey}:rtc`);
    if (rtcBase64) {
      emulator.import_cartridge_rtc_persistence_bytes(base64ToBytes(rtcBase64));
    }

    emulator.mark_cartridge_persistence_clean();
    debouncer.markFlushed();
  }

  function attachRom({ romBytes, fileName, nextEmulator }) {
    emulator = nextEmulator;
    romKey = nextEmulator ? buildRomPersistenceKey({ fileName, romBytes }) : null;
    romFileName = nextEmulator ? String(fileName || "game.gb") : null;
    debouncer.markFlushed();
    loadForCurrentRom();
  }

  function detachRom() {
    emulator = null;
    romKey = null;
    romFileName = null;
    debouncer.markFlushed();
  }

  function flushNow() {
    if (!storage || !emulator || !romKey) {
      return false;
    }

    let wroteAny = false;
    let writeFailed = false;
    const saveBytes = emulator.export_cartridge_save_ram_bytes();
    if (saveBytes) {
      if (safeStorageSet(storage, `${romKey}:sav`, bytesToBase64(saveBytes))) {
        wroteAny = true;
      } else {
        writeFailed = true;
      }
    }

    const rtcBytes = emulator.export_cartridge_rtc_persistence_bytes();
    if (rtcBytes) {
      if (safeStorageSet(storage, `${romKey}:rtc`, bytesToBase64(rtcBytes))) {
        wroteAny = true;
      } else {
        writeFailed = true;
      }
    }

    if (!writeFailed) {
      emulator.mark_cartridge_persistence_clean();
      debouncer.markFlushed();
    }
    return wroteAny;
  }

  function tick() {
    if (!emulator) {
      return false;
    }

    const dirty = emulator.cartridge_battery_save_dirty();
    if (!debouncer.updateAndShouldFlush(dirty, now())) {
      return false;
    }

    return flushNow();
  }

  function importSavBytes(saveBytes) {
    if (!emulator || !romKey) {
      return false;
    }

    emulator.import_cartridge_save_ram_bytes(saveBytes);
    const saved = persistSaveBytes(saveBytes);
    if (saved) {
      emulator.mark_cartridge_persistence_clean();
      debouncer.markFlushed();
    }
    return saved;
  }

  function importRtcBytes(rtcBytes) {
    if (!emulator || !romKey) {
      return false;
    }

    const accepted = emulator.import_cartridge_rtc_persistence_bytes(rtcBytes);
    if (!accepted) {
      return false;
    }

    const saved = persistRtcBytes(rtcBytes);
    if (saved) {
      emulator.mark_cartridge_persistence_clean();
      debouncer.markFlushed();
    }
    return saved;
  }

  function exportSavBytes() {
    if (!emulator) {
      return null;
    }
    if (storage && romKey) {
      const saveBase64 = safeStorageGet(storage, `${romKey}:sav`);
      if (saveBase64) {
        return base64ToBytes(saveBase64);
      }
    }
    const saveBytes = emulator.export_cartridge_save_ram_bytes();
    if (!saveBytes) {
      return null;
    }
    return saveBytes;
  }

  function exportSavFileName() {
    if (!romFileName) {
      return "game.sav";
    }
    return replaceFileExtension(romFileName, "sav");
  }

  function exportRtcBytes() {
    if (!emulator) {
      return null;
    }
    if (storage && romKey) {
      const rtcBase64 = safeStorageGet(storage, `${romKey}:rtc`);
      if (rtcBase64) {
        return base64ToBytes(rtcBase64);
      }
    }
    const rtcBytes = emulator.export_cartridge_rtc_persistence_bytes();
    if (!rtcBytes) {
      return null;
    }
    return rtcBytes;
  }

  function exportRtcFileName() {
    if (!romFileName) {
      return "game.rtc";
    }
    return replaceFileExtension(romFileName, "rtc");
  }

  return {
    attachRom,
    detachRom,
    importSavBytes,
    importRtcBytes,
    exportSavBytes,
    exportSavFileName,
    exportRtcBytes,
    exportRtcFileName,
    flushNow,
    tick,
  };

  function persistSaveBytes(saveBytes) {
    if (!storage || !romKey) {
      return false;
    }
    return safeStorageSet(storage, `${romKey}:sav`, bytesToBase64(saveBytes));
  }

  function persistRtcBytes(rtcBytes) {
    if (!storage || !romKey) {
      return false;
    }
    return safeStorageSet(storage, `${romKey}:rtc`, bytesToBase64(rtcBytes));
  }
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
    // Ignore storage quota/security failures in the demo host.
    return false;
  }
}

function sanitizeKeyPart(value) {
  return String(value || "rom").replaceAll(":", "_");
}

function replaceFileExtension(fileName, nextExtension) {
  const value = String(fileName || "game");
  const lastDot = value.lastIndexOf(".");
  const stem = lastDot > 0 ? value.slice(0, lastDot) : value;
  return `${stem}.${nextExtension}`;
}

export function fnv1a32Hex(bytes) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i += 1) {
    hash ^= bytes[i];
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

export function bytesToBase64(bytes) {
  if (typeof Buffer !== "undefined") {
    return Buffer.from(bytes).toString("base64");
  }

  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

export function base64ToBytes(base64) {
  if (typeof Buffer !== "undefined") {
    return new Uint8Array(Buffer.from(base64, "base64"));
  }

  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}
