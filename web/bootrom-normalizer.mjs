const CANONICAL_FILE_NAME_BY_MODEL = Object.freeze({
  dmg0: "dmg0_boot.bin",
  dmg: "dmg_boot.bin",
  mgb: "mgb_boot.bin",
  sgb: "sgb_boot.bin",
  sgb2: "sgb2_boot.bin",
});

const MODEL_BY_CANONICAL_FILE_NAME = Object.freeze(
  Object.fromEntries(
    Object.entries(CANONICAL_FILE_NAME_BY_MODEL).map(([model, fileName]) => [fileName, model])
  )
);

export function expectedCanonicalBootRomFileNameForModel(model) {
  return CANONICAL_FILE_NAME_BY_MODEL[String(model || "").toLowerCase()] || null;
}

export function canonicalBootRomFileNameToModel(fileName) {
  return MODEL_BY_CANONICAL_FILE_NAME[String(fileName || "")] || null;
}

export function classifyBootRomForWebHardware(bootRomBytes, classifyBootRomFileName) {
  const canonicalFileName = classifyBootRomFileName(bootRomBytes);
  if (!canonicalFileName) {
    return { kind: "invalid" };
  }

  const model = canonicalBootRomFileNameToModel(canonicalFileName);
  if (!model) {
    return {
      kind: "known_unsupported",
      canonicalFileName,
    };
  }

  return {
    kind: "supported",
    canonicalFileName,
    model,
  };
}

export function isValidStoredBootRomForModel({
  model,
  bootRomBytes,
  classifyBootRomFileName,
}) {
  if (!(bootRomBytes instanceof Uint8Array) || bootRomBytes.length < 0x100) {
    return false;
  }

  const expectedCanonicalFileName = expectedCanonicalBootRomFileNameForModel(model);
  if (!expectedCanonicalFileName) {
    return false;
  }

  return classifyBootRomFileName(bootRomBytes) === expectedCanonicalFileName;
}
