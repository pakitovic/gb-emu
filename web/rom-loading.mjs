export function createWebEmulatorFromRomBytes({
  romBytes,
  WebEmulator,
  model,
  bootRomBytes,
}) {
  if (!romBytes) {
    return null;
  }

  const normalizedModel = model || undefined;
  const normalizedBootRomBytes =
    bootRomBytes instanceof Uint8Array && bootRomBytes.length > 0 ? bootRomBytes : null;

  if (normalizedBootRomBytes && typeof WebEmulator.newWithBootRom === "function") {
    return WebEmulator.newWithBootRom(romBytes, normalizedModel, normalizedBootRomBytes);
  }

  return new WebEmulator(romBytes, normalizedModel);
}

export async function createWebEmulatorFromRomFile({
  file,
  WebEmulator,
  model,
  bootRomBytes,
}) {
  if (!file) {
    return null;
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  return {
    emulator: createWebEmulatorFromRomBytes({
      romBytes: bytes,
      WebEmulator,
      model,
      bootRomBytes,
    }),
    romBytes: bytes,
  };
}

export function buildRomLoadedStatusMessage({ fileName, romTitle, model, warningCount }) {
  const warningText = warningCount > 0 ? ` (${warningCount} header warnings)` : "";
  return `Loaded ${fileName} (${romTitle}) on model ${model}.${warningText}`;
}
