export const SCREEN_WIDTH = 160;
export const SCREEN_HEIGHT = 144;

export function createUi(doc = document) {
  const romFileInput = doc.getElementById("rom-file");
  const modelSelect = doc.getElementById("model");
  const statusLabel = doc.getElementById("status");
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

    const frame = emulator.grayscale_frame();
    for (let i = 0; i < frame.length; i += 1) {
      const shade = frame[i];
      const rgbaIndex = i * 4;
      frameRgba[rgbaIndex] = shade;
      frameRgba[rgbaIndex + 1] = shade;
      frameRgba[rgbaIndex + 2] = shade;
      frameRgba[rgbaIndex + 3] = 255;
    }
    ctx.putImageData(frameImage, 0, 0);
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
      modelSelect,
      statusLabel,
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
    setCartridgeInfoFromEmulator,
    drawFrameFromEmulator,
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
