use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

const HEADER_MIN_LEN: usize = 0x150;
const ROM_ONLY: u8 = 0x00;
const ROM_RAM: u8 = 0x08;
const ROM_RAM_BATTERY: u8 = 0x09;
const MBC1: u8 = 0x01;
const MBC1_RAM: u8 = 0x02;
const MBC1_RAM_BATTERY: u8 = 0x03;
const MBC5: u8 = 0x19;
const MBC5_RAM: u8 = 0x1A;
const MBC5_RAM_BATTERY: u8 = 0x1B;
const MBC5_RUMBLE: u8 = 0x1C;
const MBC5_RUMBLE_RAM: u8 = 0x1D;
const MBC5_RUMBLE_RAM_BATTERY: u8 = 0x1E;
const ROM_BANK_BYTES: usize = 0x4000;
const RAM_BANK_BYTES: usize = 0x2000;
const ROM_ONLY_ROM_BANK_COUNT: usize = 2;
const SAVE_FILE_EXTENSION: &str = "sav";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapperType {
    RomOnly,
    Mbc1,
    Mbc5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CartridgeSpec {
    mapper: MapperType,
    has_ram: bool,
    has_battery: bool,
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
                    "Unsupported cartridge type 0x{code:02X}; supported: ROM-only (0x00), ROM+RAM (0x08), ROM+RAM+BATTERY (0x09), MBC1 family (0x01/0x02/0x03), MBC5 family (0x19..0x1E)"
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

pub struct Cartridge {
    rom: Vec<u8>,
    title: String,
    mapper: MapperType,
    rom_bank_count: usize,
    ram: Vec<u8>,
    ram_bank_count: usize,
    has_battery: bool,
    save_path: Option<PathBuf>,
    save_dirty: bool,
    ram_enable_required: bool,
    ram_enabled: bool,
    mbc1_rom_bank_low5: u8,
    mbc1_bank_high2: u8,
    mbc1_mode: u8,
    mbc5_rom_bank: u16,
    mbc5_ram_bank: u8,
}

impl Cartridge {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CartridgeError> {
        let path_ref = path.as_ref();
        let rom = fs::read(path_ref).map_err(CartridgeError::Io)?;
        let mut cartridge = Self::from_bytes(rom)?;
        cartridge.attach_save_from_rom_path(path_ref)?;
        Ok(cartridge)
    }

    pub fn from_bytes(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        if rom.len() < HEADER_MIN_LEN {
            return Err(CartridgeError::RomTooSmall { actual: rom.len() });
        }

        let cart_type = rom[0x0147];
        let Some(spec) = cartridge_spec(cart_type) else {
            return Err(CartridgeError::UnsupportedCartridgeType(cart_type));
        };

        let rom_size_code = rom[0x0148];
        let Some(expected_len) = rom_size_bytes_from_code(rom_size_code) else {
            return Err(CartridgeError::UnsupportedRomSizeCode(rom_size_code));
        };

        if rom.len() != expected_len {
            return Err(CartridgeError::UnsupportedRomLength {
                expected: expected_len,
                actual: rom.len(),
            });
        }

        let rom_bank_count = (rom.len() / ROM_BANK_BYTES).max(1);
        if spec.mapper == MapperType::RomOnly && rom_bank_count != ROM_ONLY_ROM_BANK_COUNT {
            return Err(CartridgeError::UnsupportedRomSizeForCartridge {
                cart_type,
                rom_size_code,
            });
        }

        let ram_size_code = rom[0x0149];
        let Some(ram_size_bytes) = ram_size_bytes_from_code(ram_size_code) else {
            return Err(CartridgeError::UnsupportedRamSizeCode(ram_size_code));
        };
        if !spec.has_ram && ram_size_bytes != 0 {
            return Err(CartridgeError::UnsupportedRamSizeForCartridge {
                cart_type,
                ram_size_code,
            });
        }

        let title = parse_title(&rom);
        // Keep a transient 8KB RAM window for ROM-only homebrew/test ROM protocols
        // that write pass/fail signatures into A000-BFFF without declaring cartridge RAM.
        let effective_ram_size = if spec.has_ram || spec.mapper == MapperType::RomOnly {
            if ram_size_bytes > 0 {
                ram_size_bytes
            } else {
                RAM_BANK_BYTES
            }
        } else {
            0
        };
        let compatibility_ram = effective_ram_size > 0 && ram_size_bytes == 0;
        let ram = if effective_ram_size > 0 {
            vec![0; effective_ram_size]
        } else {
            Vec::new()
        };
        let ram_bank_count = if ram.is_empty() {
            0
        } else {
            ram.len().div_ceil(RAM_BANK_BYTES)
        };

        Ok(Self {
            rom,
            title,
            mapper: spec.mapper,
            rom_bank_count,
            ram,
            ram_bank_count,
            has_battery: spec.has_battery,
            save_path: None,
            save_dirty: false,
            ram_enable_required: mapper_uses_ram_gate(spec.mapper) && !compatibility_ram,
            ram_enabled: !mapper_uses_ram_gate(spec.mapper) || compatibility_ram,
            mbc1_rom_bank_low5: 1,
            mbc1_bank_high2: 0,
            mbc1_mode: 0,
            mbc5_rom_bank: 1,
            mbc5_ram_bank: 0,
        })
    }

    pub fn read_rom_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.read_from_rom_bank(self.rom_bank_zero_index(), addr as usize),
            0x4000..=0x7FFF => self.read_from_rom_bank(
                self.rom_bank_switchable_index(),
                (addr as usize).saturating_sub(ROM_BANK_BYTES),
            ),
            _ => 0xFF,
        }
    }

    pub fn write_rom_control(&mut self, addr: u16, value: u8) {
        match self.mapper {
            MapperType::RomOnly => {}
            MapperType::Mbc1 => match addr {
                0x0000..=0x1FFF => {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                }
                0x2000..=0x3FFF => {
                    self.mbc1_rom_bank_low5 = value & 0x1F;
                }
                0x4000..=0x5FFF => self.mbc1_bank_high2 = value & 0x03,
                0x6000..=0x7FFF => self.mbc1_mode = value & 0x01,
                _ => {}
            },
            MapperType::Mbc5 => match addr {
                0x0000..=0x1FFF => {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                }
                0x2000..=0x2FFF => {
                    self.mbc5_rom_bank = (self.mbc5_rom_bank & 0x100) | value as u16;
                }
                0x3000..=0x3FFF => {
                    self.mbc5_rom_bank =
                        (self.mbc5_rom_bank & 0x00FF) | (((value & 0x01) as u16) << 8);
                }
                0x4000..=0x5FFF => {
                    self.mbc5_ram_bank = value & 0x0F;
                }
                _ => {}
            },
        }
    }

    pub fn read_ram_byte(&self, addr: u16) -> u8 {
        if !(0xA000..=0xBFFF).contains(&addr) {
            return 0xFF;
        }
        if self.ram.is_empty() {
            return 0xFF;
        }
        if self.ram_enable_required && !self.ram_enabled {
            return 0xFF;
        }
        let Some(index) = self.ram_index(addr) else {
            return 0xFF;
        };
        self.ram.get(index).copied().unwrap_or(0xFF)
    }

    pub fn write_ram_byte(&mut self, addr: u16, value: u8) {
        if !(0xA000..=0xBFFF).contains(&addr) {
            return;
        }
        if self.ram.is_empty() {
            return;
        }
        if self.ram_enable_required && !self.ram_enabled {
            return;
        }
        let Some(index) = self.ram_index(addr) else {
            return;
        };
        if let Some(slot) = self.ram.get_mut(index)
            && *slot != value
        {
            *slot = value;
            self.save_dirty = true;
        }
    }

    pub fn flush_save(&mut self) -> Result<(), CartridgeError> {
        if !self.has_battery || self.ram.is_empty() || !self.save_dirty {
            return Ok(());
        }
        let Some(path) = self.save_path.as_ref() else {
            return Ok(());
        };
        fs::write(path, &self.ram).map_err(CartridgeError::SaveIo)?;
        self.save_dirty = false;
        Ok(())
    }

    pub fn has_battery_save(&self) -> bool {
        self.has_battery && !self.ram.is_empty()
    }

    fn attach_save_from_rom_path(&mut self, rom_path: &Path) -> Result<(), CartridgeError> {
        if !self.has_battery_save() {
            return Ok(());
        }
        let save_path = rom_path.with_extension(SAVE_FILE_EXTENSION);
        match fs::read(&save_path) {
            Ok(data) => self.load_save_data(&data),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(CartridgeError::SaveIo(err)),
        }
        self.save_path = Some(save_path);
        self.save_dirty = false;
        Ok(())
    }

    fn load_save_data(&mut self, data: &[u8]) {
        let copy_len = self.ram.len().min(data.len());
        if copy_len > 0 {
            self.ram[..copy_len].copy_from_slice(&data[..copy_len]);
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    fn read_from_rom_bank(&self, bank_index: usize, bank_offset: usize) -> u8 {
        let bank = if self.rom_bank_count == 0 {
            0
        } else {
            bank_index % self.rom_bank_count
        };
        let offset = bank
            .saturating_mul(ROM_BANK_BYTES)
            .saturating_add(bank_offset % ROM_BANK_BYTES);
        self.rom.get(offset).copied().unwrap_or(0xFF)
    }

    fn rom_bank_zero_index(&self) -> usize {
        match self.mapper {
            MapperType::RomOnly | MapperType::Mbc5 => 0,
            MapperType::Mbc1 => {
                if self.mbc1_mode == 0 {
                    0
                } else {
                    ((self.mbc1_bank_high2 as usize) << 5) % self.rom_bank_count.max(1)
                }
            }
        }
    }

    fn rom_bank_switchable_index(&self) -> usize {
        match self.mapper {
            MapperType::RomOnly => 1 % self.rom_bank_count.max(1),
            MapperType::Mbc1 => {
                let low = self.mbc1_rom_bank_low5 & 0x1F;
                let low_nonzero = if low == 0 { 1 } else { low };
                (((self.mbc1_bank_high2 as usize) << 5) | (low_nonzero as usize))
                    % self.rom_bank_count.max(1)
            }
            MapperType::Mbc5 => (self.mbc5_rom_bank as usize) % self.rom_bank_count.max(1),
        }
    }

    fn ram_bank_index(&self) -> usize {
        if self.ram_bank_count <= 1 {
            return 0;
        }
        match self.mapper {
            MapperType::RomOnly => 0,
            MapperType::Mbc1 => {
                if self.mbc1_mode == 0 {
                    0
                } else {
                    (self.mbc1_bank_high2 as usize) % self.ram_bank_count
                }
            }
            MapperType::Mbc5 => (self.mbc5_ram_bank as usize) % self.ram_bank_count,
        }
    }

    fn ram_index(&self, addr: u16) -> Option<usize> {
        if self.ram.is_empty() {
            return None;
        }
        let bank_offset = ((addr as usize).saturating_sub(0xA000)) % RAM_BANK_BYTES;
        let index = self
            .ram_bank_index()
            .saturating_mul(RAM_BANK_BYTES)
            .saturating_add(bank_offset)
            % self.ram.len();
        Some(index)
    }
}

fn cartridge_spec(cart_type: u8) -> Option<CartridgeSpec> {
    let spec = match cart_type {
        ROM_ONLY => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: false,
            has_battery: false,
        },
        ROM_RAM => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: true,
            has_battery: false,
        },
        ROM_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: true,
            has_battery: true,
        },
        MBC1 => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: false,
            has_battery: false,
        },
        MBC1_RAM => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: true,
            has_battery: false,
        },
        MBC1_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: true,
            has_battery: true,
        },
        MBC5 | MBC5_RUMBLE => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: false,
            has_battery: false,
        },
        MBC5_RAM | MBC5_RUMBLE_RAM => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: true,
            has_battery: false,
        },
        MBC5_RAM_BATTERY | MBC5_RUMBLE_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: true,
            has_battery: true,
        },
        _ => return None,
    };
    Some(spec)
}

fn mapper_uses_ram_gate(mapper: MapperType) -> bool {
    matches!(mapper, MapperType::Mbc1 | MapperType::Mbc5)
}

fn rom_size_bytes_from_code(code: u8) -> Option<usize> {
    let bytes = match code {
        0x00 => 2 * ROM_BANK_BYTES,
        0x01 => 4 * ROM_BANK_BYTES,
        0x02 => 8 * ROM_BANK_BYTES,
        0x03 => 16 * ROM_BANK_BYTES,
        0x04 => 32 * ROM_BANK_BYTES,
        0x05 => 64 * ROM_BANK_BYTES,
        0x06 => 128 * ROM_BANK_BYTES,
        0x07 => 256 * ROM_BANK_BYTES,
        0x08 => 512 * ROM_BANK_BYTES,
        0x52 => 72 * ROM_BANK_BYTES,
        0x53 => 80 * ROM_BANK_BYTES,
        0x54 => 96 * ROM_BANK_BYTES,
        _ => return None,
    };
    Some(bytes)
}

fn ram_size_bytes_from_code(code: u8) -> Option<usize> {
    let bytes = match code {
        0x00 => 0,
        0x01 => 2 * 1024,
        0x02 => 8 * 1024,
        0x03 => 32 * 1024,
        0x04 => 128 * 1024,
        0x05 => 64 * 1024,
        _ => return None,
    };
    Some(bytes)
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
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_rom(size: usize, cart_type: u8, rom_size_code: u8, ram_size_code: u8) -> Vec<u8> {
        let mut rom = vec![0; size];
        rom[0x0147] = cart_type;
        rom[0x0148] = rom_size_code;
        rom[0x0149] = ram_size_code;
        rom
    }

    fn fill_each_rom_bank_first_byte(rom: &mut [u8]) {
        let bank_count = rom.len() / ROM_BANK_BYTES;
        for bank in 0..bank_count {
            rom[bank * ROM_BANK_BYTES] = bank as u8;
        }
    }

    fn unique_temp_file_path(name: &str, ext: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("gb_emu_{name}_{pid}_{nanos}.{ext}"))
    }

    #[test]
    fn accepts_rom_only_32kb() {
        let rom = make_rom(32 * 1024, ROM_ONLY, 0x00, 0x00);
        let cart = Cartridge::from_bytes(rom).expect("valid ROM should load");
        assert_eq!(cart.read_rom_byte(0x1234), 0x00);
    }

    #[test]
    fn supports_extended_rom_size_codes() {
        assert_eq!(rom_size_bytes_from_code(0x00), Some(32 * 1024));
        assert_eq!(rom_size_bytes_from_code(0x06), Some(2 * 1024 * 1024));
        assert_eq!(rom_size_bytes_from_code(0x08), Some(8 * 1024 * 1024));
        assert_eq!(rom_size_bytes_from_code(0x52), Some(72 * ROM_BANK_BYTES));
        assert_eq!(rom_size_bytes_from_code(0x53), Some(80 * ROM_BANK_BYTES));
        assert_eq!(rom_size_bytes_from_code(0x54), Some(96 * ROM_BANK_BYTES));
    }

    #[test]
    fn mbc1_switches_rom_bank_low_bits() {
        let mut rom = make_rom(64 * 1024, MBC1, 0x01, 0x00);
        rom[0x4000] = 0x11; // bank 1 first byte
        rom[0x4000 + 0x4000] = 0x22; // bank 2 first byte

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1 ROM should load");
        assert_eq!(cart.read_rom_byte(0x4000), 0x11);

        cart.write_rom_control(0x2000, 0x02);
        assert_eq!(cart.read_rom_byte(0x4000), 0x22);
    }

    #[test]
    fn mbc1_modes_and_banks_external_ram() {
        let rom = make_rom(256 * 1024, MBC1_RAM_BATTERY, 0x03, 0x03);
        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1+RAM+BATTERY ROM should load");

        // Disabled RAM reads as open bus and ignores writes.
        cart.write_ram_byte(0xA000, 0x66);
        assert_eq!(cart.read_ram_byte(0xA000), 0xFF);

        cart.write_rom_control(0x0000, 0x0A);
        cart.write_ram_byte(0xA000, 0x11);
        assert_eq!(cart.read_ram_byte(0xA000), 0x11);

        // Enter RAM banking mode and switch RAM bank.
        cart.write_rom_control(0x6000, 0x01);
        cart.write_rom_control(0x4000, 0x01);
        cart.write_ram_byte(0xA000, 0x22);
        assert_eq!(cart.read_ram_byte(0xA000), 0x22);

        cart.write_rom_control(0x4000, 0x00);
        assert_eq!(cart.read_ram_byte(0xA000), 0x11);

        cart.write_rom_control(0x4000, 0x01);
        assert_eq!(cart.read_ram_byte(0xA000), 0x22);

        cart.write_rom_control(0x0000, 0x00);
        assert_eq!(cart.read_ram_byte(0xA000), 0xFF);
    }

    #[test]
    fn mbc1_mode_switch_changes_fixed_rom_region() {
        let mut rom = make_rom(2 * 1024 * 1024, MBC1, 0x06, 0x00);
        fill_each_rom_bank_first_byte(&mut rom);
        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1 ROM should load");

        cart.write_rom_control(0x2000, 0x01);
        cart.write_rom_control(0x4000, 0x02);
        assert_eq!(cart.read_rom_byte(0x0000), 0x00);
        assert_eq!(cart.read_rom_byte(0x4000), 0x41);

        cart.write_rom_control(0x6000, 0x01);
        assert_eq!(cart.read_rom_byte(0x0000), 0x40);
        assert_eq!(cart.read_rom_byte(0x4000), 0x41);
    }

    #[test]
    fn mbc1_ram_battery_switches_rom_bank() {
        let mut rom = make_rom(64 * 1024, MBC1_RAM_BATTERY, 0x01, 0x02);
        rom[0x4000] = 0x11;
        rom[0x4000 + 0x4000] = 0x22;

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1+RAM+BATTERY ROM should load");
        cart.write_rom_control(0x2000, 0x02);
        assert_eq!(cart.read_rom_byte(0x4000), 0x22);
    }

    #[test]
    fn mbc5_switches_rom_bank_and_allows_bank_zero() {
        let mut rom = make_rom(64 * 1024, MBC5_RAM_BATTERY, 0x01, 0x02);
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
    fn mbc5_supports_rom_high_bit_and_ram_banks() {
        let mut rom = make_rom(8 * 1024 * 1024, MBC5_RAM_BATTERY, 0x08, 0x03);
        fill_each_rom_bank_first_byte(&mut rom);

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 ROM should load");
        assert_eq!(cart.read_rom_byte(0x4000), 0x01);

        cart.write_rom_control(0x2000, 0x01);
        cart.write_rom_control(0x3000, 0x01);
        assert_eq!(cart.read_rom_byte(0x4000), 0x01); // bank 257 wraps byte value to 0x01

        cart.write_rom_control(0x0000, 0x0A);
        cart.write_ram_byte(0xA000, 0x11);
        cart.write_rom_control(0x4000, 0x01);
        cart.write_ram_byte(0xA000, 0x22);
        cart.write_rom_control(0x4000, 0x00);
        assert_eq!(cart.read_ram_byte(0xA000), 0x11);
        cart.write_rom_control(0x4000, 0x01);
        assert_eq!(cart.read_ram_byte(0xA000), 0x22);
    }

    #[test]
    fn rejects_invalid_rom_length_for_header_code() {
        let rom = make_rom(32 * 1024 - 1, ROM_ONLY, 0x00, 0x00);
        match Cartridge::from_bytes(rom) {
            Err(CartridgeError::UnsupportedRomLength { .. }) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected ROM loading to fail"),
        }
    }

    #[test]
    fn rejects_unsupported_rom_size_code() {
        let rom = make_rom(32 * 1024, ROM_ONLY, 0x7E, 0x00);
        match Cartridge::from_bytes(rom) {
            Err(CartridgeError::UnsupportedRomSizeCode(0x7E)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected ROM loading to fail"),
        }
    }

    #[test]
    fn rejects_unsupported_ram_size_code() {
        let rom = make_rom(64 * 1024, MBC1_RAM, 0x01, 0x7E);
        match Cartridge::from_bytes(rom) {
            Err(CartridgeError::UnsupportedRamSizeCode(0x7E)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected ROM loading to fail"),
        }
    }

    #[test]
    fn rejects_rom_only_with_64kb_size_code() {
        let rom = make_rom(64 * 1024, ROM_ONLY, 0x01, 0x00);
        match Cartridge::from_bytes(rom) {
            Err(CartridgeError::UnsupportedRomSizeForCartridge {
                cart_type,
                rom_size_code,
            }) => {
                assert_eq!(cart_type, ROM_ONLY);
                assert_eq!(rom_size_code, 0x01);
            }
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected ROM loading to fail"),
        }
    }

    #[test]
    fn persists_battery_backed_ram_to_sav_file() {
        let rom_path = unique_temp_file_path("save_roundtrip", "gb");
        let save_path = rom_path.with_extension("sav");
        let rom = make_rom(64 * 1024, MBC1_RAM_BATTERY, 0x01, 0x02);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let mut first_load = Cartridge::from_file(&rom_path).expect("cartridge should load");
        first_load.write_rom_control(0x0000, 0x0A);
        first_load.write_ram_byte(0xA000, 0x5A);
        first_load.flush_save().expect("flush should persist save");
        assert!(save_path.exists());

        let mut second_load = Cartridge::from_file(&rom_path).expect("cartridge should reload");
        second_load.write_rom_control(0x0000, 0x0A);
        assert_eq!(second_load.read_ram_byte(0xA000), 0x5A);

        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(rom_path);
    }

    #[test]
    fn non_battery_carts_do_not_write_save_files() {
        let rom_path = unique_temp_file_path("save_non_battery", "gb");
        let save_path = rom_path.with_extension("sav");
        let rom = make_rom(64 * 1024, MBC1_RAM, 0x01, 0x02);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let mut cart = Cartridge::from_file(&rom_path).expect("cartridge should load");
        cart.write_rom_control(0x0000, 0x0A);
        cart.write_ram_byte(0xA000, 0x33);
        cart.flush_save().expect("flush should not fail");
        assert!(!save_path.exists());

        let _ = fs::remove_file(rom_path);
    }
}
