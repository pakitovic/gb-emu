use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

const HEADER_MIN_LEN: usize = 0x150;
const ROM_ONLY: u8 = 0x00;
const MBC1: u8 = 0x01;
const MBC1_RAM: u8 = 0x02;
const MBC1_RAM_BATTERY: u8 = 0x03;
const MBC5: u8 = 0x19;
const MBC5_RAM: u8 = 0x1A;
const MBC5_RAM_BATTERY: u8 = 0x1B;
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
                    "Unsupported cartridge type 0x{code:02X}; supported: ROM-only (0x00), MBC1 (0x01), MBC1+RAM (0x02), MBC1+RAM+BATTERY (0x03), MBC5 (0x19), MBC5+RAM (0x1A), MBC5+RAM+BATTERY (0x1B)"
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
    rom_bank: u16,
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
        if !is_supported_cart_type(cart_type) {
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
        if is_mbc1(self.cart_type) && (0x2000..=0x3FFF).contains(&addr) {
            let bank = value & 0x1F;
            self.rom_bank = if bank == 0 { 1 } else { bank as u16 };
            return;
        }

        if is_mbc5(self.cart_type) {
            match addr {
                0x2000..=0x2FFF => {
                    // MBC5 low 8-bit ROM bank register.
                    self.rom_bank = (self.rom_bank & 0x100) | value as u16;
                }
                0x3000..=0x3FFF => {
                    // MBC5 high 1-bit ROM bank register.
                    self.rom_bank = (self.rom_bank & 0x00FF) | (((value & 0x01) as u16) << 8);
                }
                _ => {}
            }
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

fn is_supported_cart_type(cart_type: u8) -> bool {
    matches!(
        cart_type,
        ROM_ONLY | MBC1 | MBC1_RAM | MBC1_RAM_BATTERY | MBC5 | MBC5_RAM | MBC5_RAM_BATTERY
    )
}

fn is_mbc1(cart_type: u8) -> bool {
    matches!(cart_type, MBC1 | MBC1_RAM | MBC1_RAM_BATTERY)
}

fn is_mbc5(cart_type: u8) -> bool {
    matches!(cart_type, MBC5 | MBC5_RAM | MBC5_RAM_BATTERY)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rom(size: usize, cart_type: u8, rom_size_code: u8) -> Vec<u8> {
        let mut rom = vec![0; size];
        rom[0x0147] = cart_type;
        rom[0x0148] = rom_size_code;
        rom
    }

    #[test]
    fn accepts_rom_only_32kb() {
        let rom = make_rom(ROM_32KB_BYTES, ROM_ONLY, ROM_SIZE_32KB_CODE);
        let cart = Cartridge::from_bytes(rom).expect("valid ROM should load");
        assert_eq!(cart.read_rom_byte(0x1234), 0x00);
    }

    #[test]
    fn mbc1_switches_rom_bank() {
        let mut rom = make_rom(ROM_64KB_BYTES, MBC1, ROM_SIZE_64KB_CODE);
        rom[0x4000] = 0x11; // bank 1 first byte
        rom[0x4000 + 0x4000] = 0x22; // bank 2 first byte

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1 ROM should load");
        assert_eq!(cart.read_rom_byte(0x4000), 0x11);

        cart.write_rom_control(0x2000, 0x02);
        assert_eq!(cart.read_rom_byte(0x4000), 0x22);
    }

    #[test]
    fn mbc1_ram_battery_switches_rom_bank() {
        let mut rom = make_rom(ROM_64KB_BYTES, MBC1_RAM_BATTERY, ROM_SIZE_64KB_CODE);
        rom[0x4000] = 0x11; // bank 1 first byte
        rom[0x4000 + 0x4000] = 0x22; // bank 2 first byte

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1+RAM+BATTERY ROM should load");
        assert_eq!(cart.read_rom_byte(0x4000), 0x11);

        cart.write_rom_control(0x2000, 0x02);
        assert_eq!(cart.read_rom_byte(0x4000), 0x22);
    }

    #[test]
    fn mbc5_switches_rom_bank_and_allows_bank_zero() {
        let mut rom = make_rom(ROM_64KB_BYTES, MBC5_RAM_BATTERY, ROM_SIZE_64KB_CODE);
        rom[0x0000] = 0x10; // bank 0 first byte
        rom[0x4000] = 0x11; // bank 1 first byte
        rom[0x8000] = 0x22; // bank 2 first byte

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 ROM should load");
        assert_eq!(cart.read_rom_byte(0x4000), 0x11);

        cart.write_rom_control(0x2000, 0x02);
        assert_eq!(cart.read_rom_byte(0x4000), 0x22);

        cart.write_rom_control(0x2000, 0x00);
        assert_eq!(cart.read_rom_byte(0x4000), 0x10);
    }

    #[test]
    fn rejects_invalid_rom_length_for_header_code() {
        let rom = make_rom(ROM_32KB_BYTES - 1, ROM_ONLY, ROM_SIZE_32KB_CODE);
        match Cartridge::from_bytes(rom) {
            Err(CartridgeError::UnsupportedRomLength(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected ROM loading to fail"),
        }
    }
}
