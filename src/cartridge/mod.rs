use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

const HEADER_MIN_LEN: usize = 0x150;
const ROM_ONLY: u8 = 0x00;
const MBC1: u8 = 0x01;
const MBC1_RAM: u8 = 0x02;
const ROM_SIZE_32KB_CODE: u8 = 0x00;
const ROM_SIZE_64KB_CODE: u8 = 0x01;
const ROM_32KB_BYTES: usize = 32 * 1024;
const ROM_64KB_BYTES: usize = 64 * 1024;

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
                    "Unsupported cartridge type 0x{code:02X}; supported: ROM-only (0x00), MBC1 (0x01), MBC1+RAM (0x02)"
                )
            }
            Self::UnsupportedRomSizeCode(code) => {
                write!(
                    f,
                    "Unsupported ROM size code 0x{code:02X}; supported: 32KB (0x00), 64KB (0x01)"
                )
            }
            Self::UnsupportedRomLength(len) => {
                write!(
                    f,
                    "Unsupported ROM file length {len}; expected {ROM_32KB_BYTES} or {ROM_64KB_BYTES} bytes"
                )
            }
        }
    }
}

impl std::error::Error for CartridgeError {}

pub struct Cartridge {
    rom: Vec<u8>,
    title: String,
    cart_type: u8,
    rom_bank: u8,
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
        if cart_type != ROM_ONLY && cart_type != MBC1 && cart_type != MBC1_RAM {
            return Err(CartridgeError::UnsupportedCartridgeType(cart_type));
        }

        let rom_size_code = rom[0x0148];
        if rom_size_code != ROM_SIZE_32KB_CODE && rom_size_code != ROM_SIZE_64KB_CODE {
            return Err(CartridgeError::UnsupportedRomSizeCode(rom_size_code));
        }

        let expected_len = match rom_size_code {
            ROM_SIZE_32KB_CODE => ROM_32KB_BYTES,
            ROM_SIZE_64KB_CODE => ROM_64KB_BYTES,
            _ => unreachable!(),
        };

        if rom.len() != expected_len {
            return Err(CartridgeError::UnsupportedRomLength(rom.len()));
        }

        let title = parse_title(&rom);
        Ok(Self {
            rom,
            title,
            cart_type,
            rom_bank: 1,
        })
    }

    pub fn read_rom_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                if self.cart_type == ROM_ONLY {
                    self.rom.get(addr as usize).copied().unwrap_or(0xFF)
                } else {
                    let bank_count = (self.rom.len() / 0x4000).max(1);
                    let bank = (self.rom_bank as usize) % bank_count;
                    let offset = bank * 0x4000 + (addr as usize - 0x4000);
                    self.rom.get(offset).copied().unwrap_or(0xFF)
                }
            }
            _ => 0xFF,
        }
    }

    pub fn write_rom_control(&mut self, addr: u16, value: u8) {
        if (self.cart_type == MBC1 || self.cart_type == MBC1_RAM)
            && (0x2000..=0x3FFF).contains(&addr)
        {
            let bank = value & 0x1F;
            self.rom_bank = if bank == 0 { 1 } else { bank };
        }
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
