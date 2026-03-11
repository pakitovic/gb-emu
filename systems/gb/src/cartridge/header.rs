use super::{
    CartridgeHeaderWarning, GLOBAL_CHECKSUM_HIGH_OFFSET, GLOBAL_CHECKSUM_LOW_OFFSET,
    HEADER_CHECKSUM_END, HEADER_CHECKSUM_OFFSET, HEADER_CHECKSUM_START, HEADER_LOGO_END,
    HEADER_LOGO_START, NINTENDO_LOGO_BYTES,
};

pub(crate) use self::mode_flags::parse_header_mode_flags;
pub use self::mode_flags::{CartridgeCgbSupport, CartridgeSgbSupport};

pub(crate) mod mode_flags {
    const HEADER_CGB_FLAG_OFFSET: usize = 0x0143;
    const HEADER_SGB_FLAG_OFFSET: usize = 0x0146;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CartridgeCgbSupport {
        None,
        Supported,
        Required,
    }

    impl CartridgeCgbSupport {
        #[inline]
        pub(crate) fn from_header_flag_raw(flag: u8) -> Self {
            match flag {
                0x80 => Self::Supported,
                0xC0 => Self::Required,
                _ => Self::None,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CartridgeSgbSupport {
        None,
        Supported,
    }

    impl CartridgeSgbSupport {
        #[inline]
        pub(crate) fn from_header_flag_raw(flag: u8) -> Self {
            if flag == 0x03 {
                Self::Supported
            } else {
                Self::None
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct CartridgeHeaderModeFlags {
        pub(crate) cgb_header_flag_raw: u8,
        pub(crate) cgb_support: CartridgeCgbSupport,
        pub(crate) supports_cgb: bool,
        pub(crate) cgb_only: bool,
        pub(crate) sgb_header_flag_raw: u8,
        pub(crate) sgb_support: CartridgeSgbSupport,
        pub(crate) supports_sgb: bool,
    }

    pub(crate) fn parse_header_mode_flags(rom: &[u8]) -> CartridgeHeaderModeFlags {
        let cgb_header_flag_raw = rom.get(HEADER_CGB_FLAG_OFFSET).copied().unwrap_or(0x00);
        let cgb_support = CartridgeCgbSupport::from_header_flag_raw(cgb_header_flag_raw);
        let sgb_header_flag_raw = rom.get(HEADER_SGB_FLAG_OFFSET).copied().unwrap_or(0x00);
        let sgb_support = CartridgeSgbSupport::from_header_flag_raw(sgb_header_flag_raw);

        CartridgeHeaderModeFlags {
            cgb_header_flag_raw,
            cgb_support,
            supports_cgb: matches!(
                cgb_support,
                CartridgeCgbSupport::Supported | CartridgeCgbSupport::Required
            ),
            cgb_only: matches!(cgb_support, CartridgeCgbSupport::Required),
            sgb_header_flag_raw,
            sgb_support,
            supports_sgb: matches!(sgb_support, CartridgeSgbSupport::Supported),
        }
    }
}

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

pub(super) fn compute_header_crc32(rom: &[u8]) -> u32 {
    let Some(header) = rom.get(0x0100..0x0150) else {
        return 0;
    };

    let mut crc = !0u32;
    for byte in header {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg() & 0xEDB88320;
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
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

#[cfg(test)]
mod tests {
    use super::compute_header_crc32;

    #[test]
    fn header_crc32_matches_standard_crc32_over_0x100_0x14f() {
        let mut rom = vec![0u8; 0x150];
        for (index, byte) in rom[0x0100..0x0150].iter_mut().enumerate() {
            *byte = index as u8;
        }

        assert_eq!(compute_header_crc32(&rom), 0xCA26_C3E1);
    }
}
