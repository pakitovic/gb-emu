export const SCREEN_WIDTH = 160;
export const SCREEN_HEIGHT = 144;

export function createUi(doc = document) {
  const romFileInput = doc.getElementById("rom-file");
  const bootRomFileInput = doc.getElementById("bootrom-file");
  const bootRomFileButton = doc.getElementById("bootrom-file-button");
  const savFileInput = doc.getElementById("sav-file");
  const savFileButton = doc.getElementById("sav-file-button");
  const savDownloadButton = doc.getElementById("sav-download");
  const rtcFileInput = doc.getElementById("rtc-file");
  const rtcFileButton = doc.getElementById("rtc-file-button");
  const rtcDownloadButton = doc.getElementById("rtc-download");
  const romStartButton = doc.getElementById("rom-start");
  const romResetButton = doc.getElementById("rom-reset");
  const romCloseButton = doc.getElementById("rom-close");
  const modelSelect = doc.getElementById("model");
  const paletteSelect = doc.getElementById("palette");
  const bootRomModelCheck = doc.getElementById("bootrom-model-check");
  const statusLabel = doc.getElementById("status");
  const persistenceInfoLabel = doc.getElementById("persistence-info");
  const bootRomInfoLabel = doc.getElementById("bootrom-info");
  const audioTelemetryLabel = doc.getElementById("audio-telemetry");
  const cartInfoPre = doc.getElementById("cart-info");
  const serialPre = doc.getElementById("serial");
  const testToneCheckbox = doc.getElementById("test-tone");
  const audioEnableButton = doc.getElementById("audio-enable");
  const audioResamplerSelect = doc.getElementById("audio-resampler");
  const canvas = doc.getElementById("screen");
  const ctx = canvas?.getContext("2d");

  if (!canvas || !ctx) {
    throw new Error("Web demo UI is missing the screen canvas or 2D context");
  }

  const frameRgba = new Uint8ClampedArray(SCREEN_WIDTH * SCREEN_HEIGHT * 4);
  const frameImage = new ImageData(frameRgba, SCREEN_WIDTH, SCREEN_HEIGHT);

  function setStatus(message) {
    if (statusLabel) {
      statusLabel.textContent = message;
    }
  }

  function setAudioTelemetryText(message) {
    if (audioTelemetryLabel) {
      audioTelemetryLabel.textContent = message;
    }
  }

  function setPersistenceInfoText(message) {
    if (persistenceInfoLabel) {
      persistenceInfoLabel.textContent = message;
    }
  }

  function setBootRomInfoText(message) {
    if (bootRomInfoLabel) {
      bootRomInfoLabel.textContent = message;
    }
  }

  function setBootRomModelCheck(isValid) {
    if (!bootRomModelCheck) {
      return;
    }

    bootRomModelCheck.classList.toggle("is-valid", Boolean(isValid));
    bootRomModelCheck.textContent = isValid ? "✓" : "✗";
  }

  function setPersistenceControlsEnabled({
    hasRom = false,
    batterySave = false,
    rtc = false,
  } = {}) {
    if (savFileInput) {
      savFileInput.disabled = !(hasRom && batterySave);
    }
    if (savFileButton) {
      savFileButton.classList.toggle("is-disabled", !(hasRom && batterySave));
    }
    if (savDownloadButton) {
      savDownloadButton.disabled = !(hasRom && batterySave);
    }

    if (rtcFileInput) {
      rtcFileInput.disabled = !(hasRom && rtc);
    }
    if (rtcFileButton) {
      rtcFileButton.classList.toggle("is-disabled", !(hasRom && rtc));
    }
    if (rtcDownloadButton) {
      rtcDownloadButton.disabled = !(hasRom && rtc);
    }
  }

  function setRomControlsEnabled({ hasRom = false, isRunning = false } = {}) {
    if (romStartButton) {
      romStartButton.disabled = !hasRom || isRunning;
    }
    if (romCloseButton) {
      romCloseButton.disabled = !hasRom;
    }
    if (romResetButton) {
      romResetButton.disabled = !hasRom;
    }
  }

  function setPersistenceInfoFromEmulator(emulator) {
    if (!persistenceInfoLabel) {
      return;
    }
    if (!emulator) {
      setPersistenceInfoText("Persistence: no ROM loaded.");
      setPersistenceControlsEnabled();
      return;
    }

    const batterySave =
      typeof emulator.cartridge_has_battery_save === "function"
        ? Boolean(emulator.cartridge_has_battery_save())
        : false;
    const rtc =
      typeof emulator.cartridge_has_rtc_persistence === "function"
        ? Boolean(emulator.cartridge_has_rtc_persistence())
        : false;
    setPersistenceControlsEnabled({ hasRom: true, batterySave, rtc });
    setPersistenceInfoText(
      `Persistence: battery-save=${batterySave ? "yes" : "no"} | rtc=${rtc ? "yes" : "no"}`
    );
  }

  function setCartridgeInfoFromEmulator(emulator) {
    if (!cartInfoPre) {
      return;
    }
    if (!emulator) {
      cartInfoPre.textContent = "Cartridge: not loaded.";
      return;
    }
    cartInfoPre.textContent = emulator.cartridge_debug_report();
  }

  function drawFrameFromEmulator(emulator) {
    if (!emulator) {
      return;
    }

    const frame = emulator.rgba_frame();
    if (frame.length !== frameRgba.length) {
      throw new Error(
        `RGBA frame length mismatch (expected ${frameRgba.length}, got ${frame.length})`
      );
    }
    frameRgba.set(frame);
    ctx.putImageData(frameImage, 0, 0);
  }

  function clearScreen() {
    ctx.fillStyle = "#000000";
    ctx.fillRect(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT);
    ctx.fillStyle = "#b8c8d2";
    ctx.font = "10px IBM Plex Mono, monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText("No ROM loaded", SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2);
  }

  function setSerialOutputRaw(rawSerial) {
    if (!serialPre) {
      return;
    }
    serialPre.textContent = formatSerialOutputForDisplay(rawSerial);
  }

  function clearSerialOutput() {
    if (serialPre) {
      serialPre.textContent = "";
    }
  }

  return {
    refs: {
      romFileInput,
      bootRomFileInput,
      bootRomFileButton,
      savFileInput,
      savFileButton,
      savDownloadButton,
      rtcFileInput,
      rtcFileButton,
      rtcDownloadButton,
      romStartButton,
      romResetButton,
      romCloseButton,
      modelSelect,
      paletteSelect,
      bootRomModelCheck,
      statusLabel,
      persistenceInfoLabel,
      bootRomInfoLabel,
      audioTelemetryLabel,
      cartInfoPre,
      serialPre,
      testToneCheckbox,
      audioEnableButton,
      audioResamplerSelect,
      canvas,
    },
    setStatus,
    setAudioTelemetryText,
    setPersistenceInfoText,
    setBootRomInfoText,
    setBootRomModelCheck,
    setPersistenceControlsEnabled,
    setRomControlsEnabled,
    setPersistenceInfoFromEmulator,
    setCartridgeInfoFromEmulator,
    drawFrameFromEmulator,
    clearScreen,
    setSerialOutputRaw,
    clearSerialOutput,
  };
}

export function formatSerialOutputForDisplay(rawSerial) {
  if (!rawSerial) {
    return "";
  }

  const MAX_VISIBLE_CHARS = 4096;
  let nonTextCount = 0;
  let formatted = "";

  for (const ch of rawSerial) {
    const code = ch.codePointAt(0);
    if (code === undefined) {
      continue;
    }
    if (ch === "\n" || ch === "\r" || ch === "\t" || (code >= 0x20 && code <= 0x7e)) {
      formatted += ch;
      continue;
    }
    nonTextCount += 1;
  }

  if (formatted.length === 0 && nonTextCount > 0) {
    return `[serial debug text hidden: received ${nonTextCount} non-text byte(s)]`;
  }

  if (formatted.length > MAX_VISIBLE_CHARS) {
    formatted = formatted.slice(-MAX_VISIBLE_CHARS);
    formatted = `[serial output truncated, showing last ${MAX_VISIBLE_CHARS} chars]\n${formatted}`;
  }

  if (nonTextCount > 0) {
    formatted += `\n[filtered ${nonTextCount} non-text byte(s)]`;
  }

  return formatted;
}
