use super::{
    CartridgeHeaderWarning, GLOBAL_CHECKSUM_HIGH_OFFSET, GLOBAL_CHECKSUM_LOW_OFFSET,
    HEADER_CHECKSUM_END, HEADER_CHECKSUM_OFFSET, HEADER_CHECKSUM_START, HEADER_LOGO_END,
    HEADER_LOGO_START, NINTENDO_LOGO_BYTES,
};

pub(super) fn diagnose_header(rom: &[u8]) -> Vec<CartridgeHeaderWarning> {
    let mut warnings = Vec::new();

    if !has_valid_nintendo_logo(rom) {
        warnings.push(CartridgeHeaderWarning::NintendoLogoMismatch);
    }

    let header_checksum = read_header_checksum(rom);
    let computed_header_checksum = compute_header_checksum(rom);
    if header_checksum != computed_header_checksum {
        warnings.push(CartridgeHeaderWarning::HeaderChecksumMismatch {
            header_value: header_checksum,
            computed_value: computed_header_checksum,
        });
    }

    let global_checksum = read_global_checksum(rom);
    let computed_global_checksum = compute_global_checksum(rom);
    if global_checksum != computed_global_checksum {
        warnings.push(CartridgeHeaderWarning::GlobalChecksumMismatch {
            header_value: global_checksum,
            computed_value: computed_global_checksum,
        });
    }

    warnings
}

fn has_valid_nintendo_logo(rom: &[u8]) -> bool {
    let Some(logo_bytes) = rom.get(HEADER_LOGO_START..=HEADER_LOGO_END) else {
        return false;
    };
    logo_bytes == NINTENDO_LOGO_BYTES
}

fn read_header_checksum(rom: &[u8]) -> u8 {
    rom.get(HEADER_CHECKSUM_OFFSET).copied().unwrap_or(0)
}

pub(super) fn compute_header_checksum(rom: &[u8]) -> u8 {
    let Some(checksum_slice) = rom.get(HEADER_CHECKSUM_START..=HEADER_CHECKSUM_END) else {
        return 0;
    };
    checksum_slice
        .iter()
        .fold(0u8, |acc, byte| acc.wrapping_sub(*byte).wrapping_sub(1))
}

fn read_global_checksum(rom: &[u8]) -> u16 {
    let high = rom.get(GLOBAL_CHECKSUM_HIGH_OFFSET).copied().unwrap_or(0) as u16;
    let low = rom.get(GLOBAL_CHECKSUM_LOW_OFFSET).copied().unwrap_or(0) as u16;
    (high << 8) | low
}

pub(super) fn compute_global_checksum(rom: &[u8]) -> u16 {
    rom.iter().enumerate().fold(0u16, |acc, (index, byte)| {
        if index == GLOBAL_CHECKSUM_HIGH_OFFSET || index == GLOBAL_CHECKSUM_LOW_OFFSET {
            acc
        } else {
            acc.wrapping_add(*byte as u16)
        }
    })
}

pub(super) fn parse_title(rom: &[u8]) -> String {
    let title_bytes = &rom[0x0134..=0x0143];
    let end = title_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(title_bytes.len());
    title_bytes[..end]
        .iter()
        .map(|&b| b as char)
        .collect::<String>()
        .trim()
        .to_string()
}
