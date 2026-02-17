use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

const HEADER_MIN_LEN: usize = 0x150;
const ROM_ONLY: u8 = 0x00;
const ROM_SIZE_32KB_CODE: u8 = 0x00;
const ROM_32KB_BYTES: usize = 32 * 1024;

#[derive(Debug)]
pub enum CartridgeError {
    Io(std::io::Error),
    RomTooSmall { actual: usize },
    UnsupportedCartridgeType(u8),
    UnsupportedRomSizeCode(u8),
    UnsupportedRomLength(usize),
}

impl Display for CartridgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error loading ROM: {err}"),
            Self::RomTooSmall { actual } => {
                write!(
                    f,
                    "ROM too small ({actual} bytes), expected at least {HEADER_MIN_LEN} bytes"
                )
            }
            Self::UnsupportedCartridgeType(code) => {
                write!(
                    f,
                    "Unsupported cartridge type 0x{code:02X}; only ROM-only (0x00) is supported"
                )
            }
            Self::UnsupportedRomSizeCode(code) => {
                write!(
                    f,
                    "Unsupported ROM size code 0x{code:02X}; only 32KB (0x00) is supported"
                )
            }
            Self::UnsupportedRomLength(len) => {
                write!(
                    f,
                    "Unsupported ROM file length {len}; expected exactly {ROM_32KB_BYTES} bytes for ROM-only"
                )
            }
        }
    }
}

impl std::error::Error for CartridgeError {}

pub struct Cartridge {
    rom: Vec<u8>,
    title: String,
}

impl Cartridge {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CartridgeError> {
        let rom = fs::read(path).map_err(CartridgeError::Io)?;
        Self::from_bytes(rom)
    }

    pub fn from_bytes(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        if rom.len() < HEADER_MIN_LEN {
            return Err(CartridgeError::RomTooSmall { actual: rom.len() });
        }

        let cart_type = rom[0x0147];
        if cart_type != ROM_ONLY {
            return Err(CartridgeError::UnsupportedCartridgeType(cart_type));
        }

        let rom_size_code = rom[0x0148];
        if rom_size_code != ROM_SIZE_32KB_CODE {
            return Err(CartridgeError::UnsupportedRomSizeCode(rom_size_code));
        }

        if rom.len() != ROM_32KB_BYTES {
            return Err(CartridgeError::UnsupportedRomLength(rom.len()));
        }

        let title = parse_title(&rom);
        Ok(Self { rom, title })
    }

    pub fn read_rom_byte(&self, addr: u16) -> u8 {
        self.rom.get(addr as usize).copied().unwrap_or(0xFF)
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

fn parse_title(rom: &[u8]) -> String {
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
