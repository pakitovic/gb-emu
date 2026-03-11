const STORAGE_KEY = "gb-emu:web:palette-overrides:v1";
const DEFAULT_FILE_NAME = "overrides.ini";

export function createWebPaletteOverridePersistence({
  storage = globalThis.localStorage,
} = {}) {
  function loadPaletteOverrideState() {
    if (!storage) {
      return null;
    }
    const raw = safeStorageGet(storage, STORAGE_KEY);
    if (!raw) {
      return null;
    }

    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch {
      return null;
    }

    if (!parsed || typeof parsed !== "object") {
      return null;
    }
    if (typeof parsed.text !== "string" || parsed.text.trim() === "") {
      return null;
    }

    return {
      name:
        typeof parsed.name === "string" && parsed.name.trim() !== ""
          ? parsed.name.trim()
          : DEFAULT_FILE_NAME,
      text: parsed.text,
    };
  }

  function storePaletteOverrideState({ name, text }) {
    if (!storage) {
      return { ok: false, error: "Browser storage unavailable." };
    }
    if (typeof text !== "string" || text.trim() === "") {
      return {
        ok: false,
        error: "Palette override text must be a non-empty string.",
      };
    }

    const normalizedName =
      typeof name === "string" && name.trim() !== "" ? name.trim() : DEFAULT_FILE_NAME;
    const saved = safeStorageSet(
      storage,
      STORAGE_KEY,
      JSON.stringify({ name: normalizedName, text })
    );
    if (!saved) {
      return {
        ok: false,
        error: "Failed to persist palette overrides in browser storage.",
      };
    }

    return {
      ok: true,
      name: normalizedName,
      storedChars: text.length,
    };
  }

  function removePaletteOverrideState() {
    if (!storage) {
      return false;
    }
    return safeStorageRemove(storage, STORAGE_KEY);
  }

  return {
    loadPaletteOverrideState,
    storePaletteOverrideState,
    removePaletteOverrideState,
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
