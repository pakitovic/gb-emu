export async function createWebEmulatorFromRomFile({ file, WebEmulator, model }) {
  if (!file) {
    return null;
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  return new WebEmulator(bytes, model || undefined);
}

export function buildRomLoadedStatusMessage({ fileName, romTitle, model, warningCount }) {
  const warningText = warningCount > 0 ? ` (${warningCount} header warnings)` : "";
  return `Loaded ${fileName} (${romTitle}) on model ${model}.${warningText}`;
}
