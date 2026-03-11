use std::fmt::{Display, Formatter};

use super::HEADER_MIN_LEN;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CartridgeMapper {
    RomOnly,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
}

impl Display for CartridgeMapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::RomOnly => "ROM-only",
            Self::Mbc1 => "MBC1",
            Self::Mbc2 => "MBC2",
            Self::Mbc3 => "MBC3",
            Self::Mbc5 => "MBC5",
        };
        write!(f, "{label}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CartridgeMetadata {
    pub title: String,
    pub header_crc32: u32,
    pub cart_type_code: u8,
    pub mapper: CartridgeMapper,
    pub rom_size_code: u8,
    pub ram_size_code: u8,
    pub rom_size_bytes: usize,
    pub rom_bank_count: usize,
    pub declared_ram_size_bytes: usize,
    pub effective_ram_size_bytes: usize,
    pub ram_bank_count: usize,
    pub compatibility_ram_mode: bool,
    pub has_battery: bool,
    pub has_timer: bool,
    pub has_rumble: bool,
    pub has_battery_save: bool,
    pub rumble_active: bool,
    pub header_warnings: Vec<CartridgeHeaderWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CartridgeHeaderWarning {
    NintendoLogoMismatch,
    HeaderChecksumMismatch {
        header_value: u8,
        computed_value: u8,
    },
    GlobalChecksumMismatch {
        header_value: u16,
        computed_value: u16,
    },
}

impl Display for CartridgeHeaderWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NintendoLogoMismatch => write!(f, "Nintendo logo mismatch"),
            Self::HeaderChecksumMismatch {
                header_value,
                computed_value,
            } => write!(
                f,
                "Header checksum mismatch (header 0x{header_value:02X}, computed 0x{computed_value:02X})"
            ),
            Self::GlobalChecksumMismatch {
                header_value,
                computed_value,
            } => write!(
                f,
                "Global checksum mismatch (header 0x{header_value:04X}, computed 0x{computed_value:04X})"
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MapperType {
    RomOnly,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CartridgeSpec {
    pub(super) mapper: MapperType,
    pub(super) has_ram: bool,
    pub(super) has_battery: bool,
    pub(super) has_timer: bool,
}

#[derive(Debug)]
pub enum CartridgeError {
    Io(std::io::Error),
    SaveIo(std::io::Error),
    RomTooSmall { actual: usize },
    UnsupportedCartridgeType(u8),
    UnsupportedRomSizeCode(u8),
    UnsupportedRamSizeCode(u8),
    UnsupportedRomSizeForCartridge { cart_type: u8, rom_size_code: u8 },
    UnsupportedRamSizeForCartridge { cart_type: u8, ram_size_code: u8 },
    UnsupportedRomLength { expected: usize, actual: usize },
}

impl Display for CartridgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error loading ROM: {err}"),
            Self::SaveIo(err) => write!(f, "I/O error loading/saving SRAM: {err}"),
            Self::RomTooSmall { actual } => {
                write!(
                    f,
                    "ROM too small ({actual} bytes), expected at least {HEADER_MIN_LEN} bytes"
                )
            }
            Self::UnsupportedCartridgeType(code) => {
                write!(
                    f,
                    "Unsupported cartridge type 0x{code:02X}; supported: ROM-only/RAM (0x00/0x08/0x09), MBC1 family (0x01/0x02/0x03), MBC2 (0x05/0x06), MBC3 family (0x0F/0x10/0x11/0x12/0x13), MBC5 family (0x19..0x1E)"
                )
            }
            Self::UnsupportedRomSizeCode(code) => {
                write!(f, "Unsupported ROM size code 0x{code:02X}")
            }
            Self::UnsupportedRamSizeCode(code) => {
                write!(f, "Unsupported RAM size code 0x{code:02X}")
            }
            Self::UnsupportedRomSizeForCartridge {
                cart_type,
                rom_size_code,
            } => {
                write!(
                    f,
                    "Unsupported ROM size code 0x{rom_size_code:02X} for cartridge type 0x{cart_type:02X}"
                )
            }
            Self::UnsupportedRamSizeForCartridge {
                cart_type,
                ram_size_code,
            } => {
                write!(
                    f,
                    "Unsupported RAM size code 0x{ram_size_code:02X} for cartridge type 0x{cart_type:02X}"
                )
            }
            Self::UnsupportedRomLength { expected, actual } => {
                write!(
                    f,
                    "Unsupported ROM file length {actual}; expected {expected} bytes for header ROM size code"
                )
            }
        }
    }
}

impl std::error::Error for CartridgeError {}
