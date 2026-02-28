use gb_emu::bootrom::{BootRomData, parse_boot_rom_prefix};
use gb_emu::hardware::HardwareModel;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_BOOT_ROM_DIR: &str = "roms/bootrom";
const BOOT_ROM_DIR_ENV: &str = "GB_BOOTROM_DIR";
const INVALID_BOOT_ROM_PREFIX: &str = "invalid_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnownBootRom {
    Dmg0,
    Dmg,
    Mgb,
    Sgb,
    Sgb2,
    Cgb,
    Cgb0,
    CgbE,
    CgbAgb,
    CgbAgb0,
}

impl KnownBootRom {
    const fn canonical_file_name(self) -> &'static str {
        match self {
            Self::Dmg0 => "dmg0_boot.bin",
            Self::Dmg => "dmg_boot.bin",
            Self::Mgb => "mgb_boot.bin",
            Self::Sgb => "sgb_boot.bin",
            Self::Sgb2 => "sgb2_boot.bin",
            Self::Cgb => "cgb_boot.bin",
            Self::Cgb0 => "cgb0_boot.bin",
            Self::CgbE => "cgbE_boot.bin",
            Self::CgbAgb => "cgb_agb_boot.bin",
            Self::CgbAgb0 => "cgb_agb0_boot.bin",
        }
    }
}

pub fn load_boot_rom_for_model(model: HardwareModel) -> Option<BootRomData> {
    load_boot_rom_for_model_from_dir(model, resolve_boot_rom_dir())
}

pub fn load_boot_rom_for_model_from_dir(
    model: HardwareModel,
    directory: impl AsRef<Path>,
) -> Option<BootRomData> {
    load_boot_rom_for_model_from_dir_with_classifier(
        model,
        directory.as_ref(),
        classify_known_boot_rom_file_name,
    )
}

fn load_boot_rom_for_model_from_dir_with_classifier(
    model: HardwareModel,
    directory: &Path,
    classifier: fn(&[u8]) -> Option<&'static str>,
) -> Option<BootRomData> {
    normalize_boot_rom_directory_with_classifier(directory, classifier);

    let path = directory.join(model.boot_rom_file_name());
    let bytes = fs::read(path).ok()?;
    parse_boot_rom_prefix(&bytes)
}

pub fn normalize_boot_rom_directory(directory: impl AsRef<Path>) {
    normalize_boot_rom_directory_with_classifier(
        directory.as_ref(),
        classify_known_boot_rom_file_name,
    );
}

fn normalize_boot_rom_directory_with_classifier(
    directory: &Path,
    classifier: fn(&[u8]) -> Option<&'static str>,
) {
    if !directory.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry_result in entries {
        let Ok(entry) = entry_result else {
            continue;
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == ".gitkeep"
            || file_name.starts_with('.')
            || file_name.starts_with(INVALID_BOOT_ROM_PREFIX)
        {
            continue;
        }

        let Ok(bytes) = fs::read(&path) else {
            continue;
        };

        let Some(expected_name) = classifier(&bytes) else {
            rename_with_invalid_prefix(directory, &path);
            continue;
        };

        if file_name == expected_name {
            continue;
        }

        let expected_path = directory.join(expected_name);
        if expected_path.exists() {
            let expected_valid = fs::read(&expected_path)
                .ok()
                .and_then(|expected_bytes| classifier(&expected_bytes))
                .map(|classified_name| classified_name == expected_name)
                .unwrap_or(false);

            if expected_valid {
                rename_with_invalid_prefix(directory, &path);
                continue;
            }

            rename_with_invalid_prefix(directory, &expected_path);
            if expected_path.exists() {
                rename_with_invalid_prefix(directory, &path);
                continue;
            }
        }

        let _ = fs::rename(&path, &expected_path);
    }
}

fn rename_with_invalid_prefix(directory: &Path, path: &Path) {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if file_name.starts_with(INVALID_BOOT_ROM_PREFIX) {
        return;
    }

    let mut candidate = format!("{INVALID_BOOT_ROM_PREFIX}{file_name}");
    let mut target = directory.join(&candidate);
    let mut suffix = 1usize;
    while target.exists() {
        candidate = format!("{INVALID_BOOT_ROM_PREFIX}{suffix}_{file_name}");
        target = directory.join(&candidate);
        suffix = suffix.saturating_add(1);
    }

    let _ = fs::rename(path, target);
}

pub fn classify_known_boot_rom_file_name(bytes: &[u8]) -> Option<&'static str> {
    let boot_rom = parse_boot_rom_prefix(bytes)?;
    let hash = sha256_hex(&boot_rom);
    known_boot_rom_from_hash(&hash).map(KnownBootRom::canonical_file_name)
}

fn known_boot_rom_from_hash(hash: &str) -> Option<KnownBootRom> {
    match hash {
        "26e71cf01e301e5dc40e987cd2ecbf6d0276245890ac829db2a25323da86818e" => {
            Some(KnownBootRom::Dmg0)
        }
        "cf053eccb4ccafff9e67339d4e78e98dce7d1ed59be819d2a1ba2232c6fce1c7" => {
            Some(KnownBootRom::Dmg)
        }
        "a8cb5f4f1f16f2573ed2ecd8daedb9c5d1dd2c30a481f9b179b5d725d95eafe2" => {
            Some(KnownBootRom::Mgb)
        }
        "0e4ddff32fc9d1eeaae812a157dd246459b00c9e14f2f61751f661f32361e360" => {
            Some(KnownBootRom::Sgb)
        }
        "fd243c4fb27008986316ce3df29e9cfbcdc0cd52704970555a8bb76edbec3988" => {
            Some(KnownBootRom::Sgb2)
        }
        "b4f2e416a35eef52cba161b159c7c8523a92594facb924b3ede0d722867c50c7" => {
            Some(KnownBootRom::Cgb)
        }
        "3a307a41689bee99a9a32ea021bf45136906c86b2e4f06c806738398e4f92e45" => {
            Some(KnownBootRom::Cgb0)
        }
        "c56299bedd56debdbf36442238636bf5887a65c5173b33995682052353804da9" => {
            Some(KnownBootRom::CgbE)
        }
        "fe3cceb79930c4cb6c6f62f742c2562fd4c96b827584ef8ea89d49b387bd6860" => {
            Some(KnownBootRom::CgbAgb)
        }
        "fe2d45405531756d87622abde6127c804bd675cb968081b2c052497a470ffeb2" => {
            Some(KnownBootRom::CgbAgb0)
        }
        _ => None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn resolve_boot_rom_dir() -> PathBuf {
    match std::env::var_os(BOOT_ROM_DIR_ENV) {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => PathBuf::from(DEFAULT_BOOT_ROM_DIR),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KnownBootRom, known_boot_rom_from_hash, load_boot_rom_for_model_from_dir_with_classifier,
        normalize_boot_rom_directory_with_classifier,
    };
    use gb_emu::hardware::HardwareModel;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_boot_rom_returns_none_when_file_is_missing() {
        let dir = unique_temp_dir("missing");
        assert!(
            load_boot_rom_for_model_from_dir_with_classifier(
                HardwareModel::Dmg,
                &dir,
                classify_for_test,
            )
            .is_none()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_boot_rom_uses_first_256_bytes() {
        let dir = unique_temp_dir("prefix");
        let path = dir.join(HardwareModel::Dmg.boot_rom_file_name());
        let mut bytes = vec![0xA5; 0x108];
        bytes[0] = 0xD0;
        bytes[0x100] = 0x00;
        fs::write(&path, bytes).expect("boot ROM write should succeed");

        let boot_rom = load_boot_rom_for_model_from_dir_with_classifier(
            HardwareModel::Dmg,
            &dir,
            classify_for_test,
        )
        .expect("should load");
        assert_eq!(boot_rom.len(), 0x100);
        assert_eq!(boot_rom[0], 0xD0);
        assert!(boot_rom.iter().skip(1).all(|byte| *byte == 0xA5));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_boot_rom_rejects_short_files() {
        let dir = unique_temp_dir("short");
        let path = dir.join(HardwareModel::Dmg.boot_rom_file_name());
        fs::write(&path, vec![0xFF; 0x80]).expect("boot ROM write should succeed");

        assert!(
            load_boot_rom_for_model_from_dir_with_classifier(
                HardwareModel::Dmg,
                &dir,
                classify_for_test,
            )
            .is_none()
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn normalization_renames_known_misnamed_file() {
        let dir = unique_temp_dir("normalize_known");
        let original = dir.join("GB.bin");
        fs::write(&original, vec![0xD0; 0x100]).expect("boot ROM write should succeed");

        normalize_boot_rom_directory_with_classifier(&dir, classify_for_test);

        assert!(!original.exists());
        assert!(dir.join("dmg_boot.bin").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn normalization_prefixes_invalid_files() {
        let dir = unique_temp_dir("normalize_invalid");
        let original = dir.join("unknown.bin");
        fs::write(&original, vec![0x11; 0x100]).expect("boot ROM write should succeed");

        normalize_boot_rom_directory_with_classifier(&dir, classify_for_test);

        assert!(!original.exists());
        assert!(dir.join("invalid_unknown.bin").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn normalization_replaces_invalid_canonical_target_with_valid_known_file() {
        let dir = unique_temp_dir("normalize_replace_target");
        let source = dir.join("GB.bin");
        let target = dir.join("dmg_boot.bin");
        fs::write(&source, vec![0xD0; 0x100]).expect("source write should succeed");
        fs::write(&target, vec![0x11; 0x100]).expect("target write should succeed");

        normalize_boot_rom_directory_with_classifier(&dir, classify_for_test);

        assert!(target.exists());
        assert_eq!(fs::read(&target).expect("read should succeed")[0], 0xD0);
        assert!(dir.join("invalid_dmg_boot.bin").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn normalization_prefixes_duplicate_misnamed_when_canonical_file_is_already_valid() {
        let dir = unique_temp_dir("normalize_duplicate");
        let source = dir.join("GB.bin");
        let target = dir.join("dmg_boot.bin");
        fs::write(&source, vec![0xD0; 0x100]).expect("source write should succeed");
        fs::write(&target, vec![0xD0; 0x100]).expect("target write should succeed");

        normalize_boot_rom_directory_with_classifier(&dir, classify_for_test);

        assert!(target.exists());
        assert!(dir.join("invalid_GB.bin").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn known_hash_table_contains_expected_dmg_hash() {
        assert_eq!(
            known_boot_rom_from_hash(
                "cf053eccb4ccafff9e67339d4e78e98dce7d1ed59be819d2a1ba2232c6fce1c7"
            ),
            Some(KnownBootRom::Dmg)
        );
    }

    #[test]
    fn known_hash_table_rejects_unknown_hashes() {
        assert!(known_boot_rom_from_hash("deadbeef").is_none());
    }

    fn classify_for_test(bytes: &[u8]) -> Option<&'static str> {
        match bytes.first().copied() {
            Some(0xD0) => Some("dmg_boot.bin"),
            Some(0xB0) => Some("sgb_boot.bin"),
            _ => None,
        }
    }

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("gb_runtime_bootrom_{name}_{pid}_{nanos}"));
        fs::create_dir_all(&dir).expect("temp dir creation should succeed");
        dir
    }
}
