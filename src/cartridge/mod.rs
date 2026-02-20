use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HEADER_MIN_LEN: usize = 0x150;
const ROM_ONLY: u8 = 0x00;
const ROM_RAM: u8 = 0x08;
const ROM_RAM_BATTERY: u8 = 0x09;
const MBC1: u8 = 0x01;
const MBC1_RAM: u8 = 0x02;
const MBC1_RAM_BATTERY: u8 = 0x03;
const MBC2: u8 = 0x05;
const MBC2_BATTERY: u8 = 0x06;
const MBC3_TIMER_BATTERY: u8 = 0x0F;
const MBC3_TIMER_RAM_BATTERY: u8 = 0x10;
const MBC3: u8 = 0x11;
const MBC3_RAM: u8 = 0x12;
const MBC3_RAM_BATTERY: u8 = 0x13;
const MBC5: u8 = 0x19;
const MBC5_RAM: u8 = 0x1A;
const MBC5_RAM_BATTERY: u8 = 0x1B;
const MBC5_RUMBLE: u8 = 0x1C;
const MBC5_RUMBLE_RAM: u8 = 0x1D;
const MBC5_RUMBLE_RAM_BATTERY: u8 = 0x1E;
const ROM_BANK_BYTES: usize = 0x4000;
const RAM_BANK_BYTES: usize = 0x2000;
const MBC2_RAM_BYTES: usize = 512;
const ROM_ONLY_ROM_BANK_COUNT: usize = 2;
const SAVE_FILE_EXTENSION: &str = "sav";
const RTC_FILE_EXTENSION: &str = "rtc";

trait RtcClock {
    fn now_epoch_secs(&self) -> u64;
}

struct SystemRtcClock;

impl RtcClock for SystemRtcClock {
    fn now_epoch_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapperType {
    RomOnly,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CartridgeSpec {
    mapper: MapperType,
    has_ram: bool,
    has_battery: bool,
    has_timer: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mbc3Rtc {
    seconds: u8,
    minutes: u8,
    hours: u8,
    day_counter: u16,
    carry: bool,
    halted: bool,
    latched_registers: [u8; 5],
    has_latched_snapshot: bool,
    latch_armed: bool,
    last_update_epoch_secs: u64,
}

impl Mbc3Rtc {
    fn new(now_epoch_secs: u64) -> Self {
        Self {
            seconds: 0,
            minutes: 0,
            hours: 0,
            day_counter: 0,
            carry: false,
            halted: false,
            latched_registers: [0; 5],
            has_latched_snapshot: false,
            latch_armed: false,
            last_update_epoch_secs: now_epoch_secs,
        }
    }

    fn tick_to_epoch(&mut self, now_epoch_secs: u64) {
        if now_epoch_secs <= self.last_update_epoch_secs {
            return;
        }
        let elapsed = now_epoch_secs.saturating_sub(self.last_update_epoch_secs);
        self.last_update_epoch_secs = now_epoch_secs;
        if self.halted {
            return;
        }
        self.add_elapsed_seconds(elapsed);
    }

    fn add_elapsed_seconds(&mut self, elapsed_secs: u64) {
        if elapsed_secs == 0 {
            return;
        }

        let mut total = (self.seconds as u64)
            + (self.minutes as u64) * 60
            + (self.hours as u64) * 3600
            + elapsed_secs;

        let total_days = total / 86_400;
        total %= 86_400;

        self.hours = (total / 3600) as u8;
        total %= 3600;
        self.minutes = (total / 60) as u8;
        self.seconds = (total % 60) as u8;

        if total_days > 0 {
            let new_days = (self.day_counter as u64).saturating_add(total_days);
            if new_days > 0x01FF {
                self.carry = true;
            }
            self.day_counter = (new_days & 0x01FF) as u16;
        }
    }

    fn latch_command(&mut self, value: u8) {
        if value == 0 {
            self.latch_armed = true;
            return;
        }
        if value == 1 && self.latch_armed {
            self.latch();
        }
        self.latch_armed = false;
    }

    fn latch(&mut self) {
        self.latched_registers = self.live_registers();
        self.has_latched_snapshot = true;
    }

    fn live_registers(&self) -> [u8; 5] {
        let day_low = (self.day_counter & 0x00FF) as u8;
        let day_high = ((self.day_counter >> 8) as u8) & 0x01;
        let halt_bit = if self.halted { 0x40 } else { 0x00 };
        let carry_bit = if self.carry { 0x80 } else { 0x00 };
        [
            self.seconds % 60,
            self.minutes % 60,
            self.hours % 24,
            day_low,
            day_high | halt_bit | carry_bit,
        ]
    }

    fn live_registers_at_epoch(&self, now_epoch_secs: u64) -> [u8; 5] {
        let mut snapshot = *self;
        snapshot.tick_to_epoch(now_epoch_secs);
        snapshot.live_registers()
    }

    fn read_register(&self, register_select: u8, use_latched: bool) -> u8 {
        let index = (register_select.saturating_sub(0x08)) as usize;
        if index >= 5 {
            return 0xFF;
        }
        if use_latched {
            self.latched_registers[index]
        } else {
            self.live_registers()[index]
        }
    }

    fn write_register(&mut self, register_select: u8, value: u8, now_epoch_secs: u64) {
        self.tick_to_epoch(now_epoch_secs);
        match register_select {
            0x08 => self.seconds = value % 60,
            0x09 => self.minutes = value % 60,
            0x0A => self.hours = value % 24,
            0x0B => {
                self.day_counter = (self.day_counter & 0x0100) | value as u16;
            }
            0x0C => {
                self.day_counter = (self.day_counter & 0x00FF) | (((value & 0x01) as u16) << 8);
                self.halted = (value & 0x40) != 0;
                self.carry = (value & 0x80) != 0;
                self.last_update_epoch_secs = now_epoch_secs;
            }
            _ => {}
        }
    }

    fn serialize(&mut self, now_epoch_secs: u64) -> [u8; 13] {
        self.tick_to_epoch(now_epoch_secs);
        let regs = self.live_registers();
        let mut out = [0u8; 13];
        out[0..5].copy_from_slice(&regs);
        out[5..13].copy_from_slice(&self.last_update_epoch_secs.to_le_bytes());
        out
    }

    fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 13 {
            return None;
        }
        let mut last_update_raw = [0u8; 8];
        last_update_raw.copy_from_slice(&bytes[5..13]);
        let rtc = Self {
            seconds: bytes[0] % 60,
            minutes: bytes[1] % 60,
            hours: bytes[2] % 24,
            day_counter: ((bytes[4] as u16 & 0x01) << 8) | bytes[3] as u16,
            carry: (bytes[4] & 0x80) != 0,
            halted: (bytes[4] & 0x40) != 0,
            latched_registers: [0; 5],
            has_latched_snapshot: false,
            latch_armed: false,
            last_update_epoch_secs: u64::from_le_bytes(last_update_raw),
        };
        Some(rtc)
    }
}

pub struct Cartridge {
    rom: Vec<u8>,
    title: String,
    mapper: MapperType,
    rom_bank_count: usize,
    ram: Vec<u8>,
    ram_bank_count: usize,
    has_battery: bool,
    has_timer: bool,
    has_rumble: bool,
    rumble_active: bool,
    clock: Box<dyn RtcClock>,
    save_path: Option<PathBuf>,
    rtc_path: Option<PathBuf>,
    save_dirty: bool,
    ram_enable_required: bool,
    ram_enabled: bool,
    mbc1_rom_bank_low5: u8,
    mbc1_bank_high2: u8,
    mbc1_mode: u8,
    mbc2_rom_bank_low4: u8,
    mbc3_rom_bank_low7: u8,
    mbc3_ram_bank_or_rtc: u8,
    rtc: Option<Mbc3Rtc>,
    mbc5_rom_bank: u16,
    mbc5_ram_bank: u8,
}

impl Cartridge {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CartridgeError> {
        Self::from_file_with_clock(path, Box::new(SystemRtcClock))
    }

    fn from_file_with_clock(
        path: impl AsRef<Path>,
        clock: Box<dyn RtcClock>,
    ) -> Result<Self, CartridgeError> {
        let path_ref = path.as_ref();
        let rom = fs::read(path_ref).map_err(CartridgeError::Io)?;
        let mut cartridge = Self::from_bytes_with_clock(rom, clock)?;
        cartridge.attach_save_from_rom_path(path_ref)?;
        Ok(cartridge)
    }

    pub fn from_bytes(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        Self::from_bytes_with_clock(rom, Box::new(SystemRtcClock))
    }

    fn from_bytes_with_clock(
        rom: Vec<u8>,
        clock: Box<dyn RtcClock>,
    ) -> Result<Self, CartridgeError> {
        if rom.len() < HEADER_MIN_LEN {
            return Err(CartridgeError::RomTooSmall { actual: rom.len() });
        }

        let cart_type = rom[0x0147];
        let Some(spec) = cartridge_spec(cart_type) else {
            return Err(CartridgeError::UnsupportedCartridgeType(cart_type));
        };
        let has_rumble = is_mbc5_rumble_type(cart_type);

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
        if spec.mapper == MapperType::Mbc2 && ram_size_code != 0x00 {
            return Err(CartridgeError::UnsupportedRamSizeForCartridge {
                cart_type,
                ram_size_code,
            });
        }
        if !spec.has_ram && ram_size_bytes != 0 {
            return Err(CartridgeError::UnsupportedRamSizeForCartridge {
                cart_type,
                ram_size_code,
            });
        }

        let title = parse_title(&rom);
        // Keep a transient 8KB RAM window for ROM-only homebrew/test ROM protocols
        // that write pass/fail signatures into A000-BFFF without declaring cartridge RAM.
        let effective_ram_size = if spec.mapper == MapperType::Mbc2 {
            MBC2_RAM_BYTES
        } else if spec.has_ram || spec.mapper == MapperType::RomOnly {
            if ram_size_bytes > 0 {
                ram_size_bytes
            } else {
                RAM_BANK_BYTES
            }
        } else {
            0
        };
        let compatibility_ram =
            effective_ram_size > 0 && ram_size_bytes == 0 && spec.mapper != MapperType::Mbc2;
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
        let now_epoch_secs = clock.now_epoch_secs();
        let rtc = if spec.has_timer {
            Some(Mbc3Rtc::new(now_epoch_secs))
        } else {
            None
        };

        Ok(Self {
            rom,
            title,
            mapper: spec.mapper,
            rom_bank_count,
            ram,
            ram_bank_count,
            has_battery: spec.has_battery,
            has_timer: spec.has_timer,
            has_rumble,
            rumble_active: false,
            clock,
            save_path: None,
            rtc_path: None,
            save_dirty: false,
            ram_enable_required: mapper_uses_ram_gate(spec.mapper) && !compatibility_ram,
            ram_enabled: !mapper_uses_ram_gate(spec.mapper) || compatibility_ram,
            mbc1_rom_bank_low5: 1,
            mbc1_bank_high2: 0,
            mbc1_mode: 0,
            mbc2_rom_bank_low4: 1,
            mbc3_rom_bank_low7: 1,
            mbc3_ram_bank_or_rtc: 0,
            rtc,
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
            MapperType::Mbc2 => {
                if (0x0000..=0x3FFF).contains(&addr) {
                    if (addr & 0x0100) == 0 {
                        self.ram_enabled = (value & 0x0F) == 0x0A;
                    } else {
                        self.mbc2_rom_bank_low4 = value & 0x0F;
                    }
                }
            }
            MapperType::Mbc3 => match addr {
                0x0000..=0x1FFF => {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                }
                0x2000..=0x3FFF => {
                    self.mbc3_rom_bank_low7 = value & 0x7F;
                }
                0x4000..=0x5FFF => {
                    self.mbc3_ram_bank_or_rtc = value;
                }
                0x6000..=0x7FFF => {
                    let now_epoch_secs = self.clock.now_epoch_secs();
                    if let Some(rtc) = self.rtc.as_mut() {
                        rtc.tick_to_epoch(now_epoch_secs);
                        rtc.latch_command(value);
                    }
                }
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
                    if self.has_rumble {
                        self.rumble_active = (value & 0x08) != 0;
                        self.mbc5_ram_bank = value & 0x07;
                    } else {
                        self.mbc5_ram_bank = value & 0x0F;
                    }
                }
                _ => {}
            },
        }
    }

    pub fn read_ram_byte(&self, addr: u16) -> u8 {
        if !(0xA000..=0xBFFF).contains(&addr) {
            return 0xFF;
        }
        if self.ram_enable_required && !self.ram_enabled {
            return 0xFF;
        }

        match self.mapper {
            MapperType::Mbc2 => {
                if self.ram.is_empty() {
                    return 0xFF;
                }
                let index = ((addr as usize).saturating_sub(0xA000)) & 0x01FF;
                let value = self.ram.get(index).copied().unwrap_or(0x0F) & 0x0F;
                value | 0xF0
            }
            MapperType::Mbc3 if self.mbc3_ram_bank_or_rtc >= 0x08 => {
                let now_epoch_secs = self.clock.now_epoch_secs();
                match self.rtc.as_ref() {
                    Some(rtc) if rtc.has_latched_snapshot => {
                        rtc.read_register(self.mbc3_ram_bank_or_rtc, true)
                    }
                    Some(rtc) => {
                        let live = rtc.live_registers_at_epoch(now_epoch_secs);
                        let index = (self.mbc3_ram_bank_or_rtc.saturating_sub(0x08)) as usize;
                        live.get(index).copied().unwrap_or(0xFF)
                    }
                    None => 0xFF,
                }
            }
            _ => {
                if self.ram.is_empty() {
                    return 0xFF;
                }
                let Some(index) = self.ram_index(addr) else {
                    return 0xFF;
                };
                self.ram.get(index).copied().unwrap_or(0xFF)
            }
        }
    }

    pub fn write_ram_byte(&mut self, addr: u16, value: u8) {
        if !(0xA000..=0xBFFF).contains(&addr) {
            return;
        }
        if self.ram_enable_required && !self.ram_enabled {
            return;
        }

        match self.mapper {
            MapperType::Mbc2 => {
                if self.ram.is_empty() {
                    return;
                }
                let index = ((addr as usize).saturating_sub(0xA000)) & 0x01FF;
                if let Some(slot) = self.ram.get_mut(index) {
                    let next = value & 0x0F;
                    if *slot != next {
                        *slot = next;
                        self.save_dirty = true;
                    }
                }
            }
            MapperType::Mbc3 if self.mbc3_ram_bank_or_rtc >= 0x08 => {
                let now_epoch_secs = self.clock.now_epoch_secs();
                if let Some(rtc) = self.rtc.as_mut() {
                    rtc.write_register(self.mbc3_ram_bank_or_rtc, value, now_epoch_secs);
                    self.save_dirty = true;
                }
            }
            _ => {
                if self.ram.is_empty() {
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
        }
    }

    pub fn flush_save(&mut self) -> Result<(), CartridgeError> {
        if !self.has_battery {
            return Ok(());
        }

        if !self.ram.is_empty()
            && self.save_dirty
            && let Some(path) = self.save_path.as_ref()
        {
            write_file_atomic(path, &self.ram).map_err(CartridgeError::SaveIo)?;
            self.save_dirty = false;
        }

        if self.has_timer
            && let (Some(rtc), Some(path)) = (self.rtc.as_mut(), self.rtc_path.as_ref())
        {
            let now_epoch_secs = self.clock.now_epoch_secs();
            let rtc_bytes = rtc.serialize(now_epoch_secs);
            write_file_atomic(path, &rtc_bytes).map_err(CartridgeError::SaveIo)?;
        }

        Ok(())
    }

    pub fn has_battery_save(&self) -> bool {
        self.has_battery && (!self.ram.is_empty() || self.has_timer)
    }

    pub fn has_rumble(&self) -> bool {
        self.has_rumble
    }

    pub fn rumble_active(&self) -> bool {
        self.has_rumble && self.rumble_active
    }

    fn attach_save_from_rom_path(&mut self, rom_path: &Path) -> Result<(), CartridgeError> {
        if !self.has_battery_save() {
            return Ok(());
        }

        if !self.ram.is_empty() {
            let save_path = rom_path.with_extension(SAVE_FILE_EXTENSION);
            match fs::read(&save_path) {
                Ok(data) => self.load_save_data(&data),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(CartridgeError::SaveIo(err)),
            }
            self.save_path = Some(save_path);
        }

        if self.has_timer {
            let rtc_path = rom_path.with_extension(RTC_FILE_EXTENSION);
            match fs::read(&rtc_path) {
                Ok(data) => self.load_rtc_data(&data),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(CartridgeError::SaveIo(err)),
            }
            self.rtc_path = Some(rtc_path);
        }

        self.save_dirty = false;
        Ok(())
    }

    fn load_save_data(&mut self, data: &[u8]) {
        let copy_len = self.ram.len().min(data.len());
        if copy_len > 0 {
            self.ram[..copy_len].copy_from_slice(&data[..copy_len]);
        }
    }

    fn load_rtc_data(&mut self, data: &[u8]) {
        if let Some(rtc) = Mbc3Rtc::deserialize(data) {
            self.rtc = Some(rtc);
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
            MapperType::RomOnly | MapperType::Mbc2 | MapperType::Mbc3 | MapperType::Mbc5 => 0,
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
            MapperType::Mbc2 => {
                let low = self.mbc2_rom_bank_low4 & 0x0F;
                let low_nonzero = if low == 0 { 1 } else { low };
                (low_nonzero as usize) % self.rom_bank_count.max(1)
            }
            MapperType::Mbc3 => {
                let low = self.mbc3_rom_bank_low7 & 0x7F;
                let low_nonzero = if low == 0 { 1 } else { low };
                (low_nonzero as usize) % self.rom_bank_count.max(1)
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
            MapperType::Mbc2 => 0,
            MapperType::Mbc3 => {
                if self.mbc3_ram_bank_or_rtc <= 0x03 {
                    (self.mbc3_ram_bank_or_rtc as usize) % self.ram_bank_count
                } else {
                    0
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
            has_timer: false,
        },
        ROM_RAM => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        ROM_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC1 => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: false,
            has_battery: false,
            has_timer: false,
        },
        MBC1_RAM => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC1_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC2 => CartridgeSpec {
            mapper: MapperType::Mbc2,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC2_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc2,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC3 => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: false,
            has_battery: false,
            has_timer: false,
        },
        MBC3_RAM => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC3_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC3_TIMER_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: false,
            has_battery: true,
            has_timer: true,
        },
        MBC3_TIMER_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: true,
            has_battery: true,
            has_timer: true,
        },
        MBC5 | MBC5_RUMBLE => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: false,
            has_battery: false,
            has_timer: false,
        },
        MBC5_RAM | MBC5_RUMBLE_RAM => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC5_RAM_BATTERY | MBC5_RUMBLE_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        _ => return None,
    };
    Some(spec)
}

fn mapper_uses_ram_gate(mapper: MapperType) -> bool {
    matches!(
        mapper,
        MapperType::Mbc1 | MapperType::Mbc2 | MapperType::Mbc3 | MapperType::Mbc5
    )
}

fn is_mbc5_rumble_type(cart_type: u8) -> bool {
    matches!(
        cart_type,
        MBC5_RUMBLE | MBC5_RUMBLE_RAM | MBC5_RUMBLE_RAM_BATTERY
    )
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

fn write_file_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut attempt = 0u32;
    loop {
        let temp_path = atomic_temp_path(path, attempt);
        attempt = attempt.saturating_add(1);

        let open_result = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path);
        let mut file = match open_result {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        };

        let write_result = (|| {
            file.write_all(data)?;
            file.sync_all()?;
            drop(file);
            match fs::rename(&temp_path, path) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    fs::remove_file(path)?;
                    fs::rename(&temp_path, path)
                }
                Err(err) => Err(err),
            }
        })();

        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }

        return write_result;
    }
}

fn atomic_temp_path(path: &Path, attempt: u32) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let base_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("save");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    parent.join(format!(".{base_name}.tmp.{pid}.{nanos}.{attempt}"))
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
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
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

    #[derive(Clone)]
    struct TestClock {
        now_epoch_secs: Arc<AtomicU64>,
    }

    impl TestClock {
        fn new(now_epoch_secs: u64) -> Self {
            Self {
                now_epoch_secs: Arc::new(AtomicU64::new(now_epoch_secs)),
            }
        }

        fn set_now_epoch_secs(&self, now_epoch_secs: u64) {
            self.now_epoch_secs.store(now_epoch_secs, Ordering::Relaxed);
        }
    }

    impl RtcClock for TestClock {
        fn now_epoch_secs(&self) -> u64 {
            self.now_epoch_secs.load(Ordering::Relaxed)
        }
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
    fn mbc5_non_rumble_uses_full_4bit_ram_bank_register() {
        let rom = make_rom(64 * 1024, MBC5_RAM, 0x01, 0x04);
        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 ROM should load");
        cart.write_rom_control(0x0000, 0x0A);

        cart.write_rom_control(0x4000, 0x00);
        cart.write_ram_byte(0xA000, 0x11);

        cart.write_rom_control(0x4000, 0x08);
        cart.write_ram_byte(0xA000, 0x88);

        cart.write_rom_control(0x4000, 0x00);
        assert_eq!(cart.read_ram_byte(0xA000), 0x11);
        cart.write_rom_control(0x4000, 0x08);
        assert_eq!(cart.read_ram_byte(0xA000), 0x88);
        assert!(!cart.has_rumble());
        assert!(!cart.rumble_active());
    }

    #[test]
    fn mbc5_rumble_masks_ram_bank_bit3_and_tracks_motor_state() {
        let rom = make_rom(64 * 1024, MBC5_RUMBLE_RAM_BATTERY, 0x01, 0x04);
        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 RUMBLE ROM should load");
        cart.write_rom_control(0x0000, 0x0A);

        cart.write_rom_control(0x4000, 0x00);
        cart.write_ram_byte(0xA000, 0x11);

        cart.write_rom_control(0x4000, 0x08);
        assert!(cart.rumble_active());
        cart.write_ram_byte(0xA000, 0x22);

        cart.write_rom_control(0x4000, 0x00);
        assert!(!cart.rumble_active());
        assert_eq!(cart.read_ram_byte(0xA000), 0x22);

        cart.write_rom_control(0x4000, 0x01);
        cart.write_ram_byte(0xA000, 0x33);

        cart.write_rom_control(0x4000, 0x09);
        assert!(cart.rumble_active());
        assert_eq!(cart.read_ram_byte(0xA000), 0x33);

        cart.write_rom_control(0x4000, 0x00);
        assert_eq!(cart.read_ram_byte(0xA000), 0x22);
        assert!(cart.has_rumble());
    }

    #[test]
    fn mbc2_switches_rom_bank_and_uses_4bit_ram_cells() {
        let mut rom = make_rom(64 * 1024, MBC2_BATTERY, 0x01, 0x00);
        rom[0x4000] = 0x11;
        rom[0x8000] = 0x22;

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC2 ROM should load");
        assert_eq!(cart.read_rom_byte(0x4000), 0x11);

        // MBC2 ROM banking: use an address with A8=1.
        cart.write_rom_control(0x2100, 0x02);
        assert_eq!(cart.read_rom_byte(0x4000), 0x22);

        // RAM is disabled until A8=0 write with 0x0A.
        cart.write_ram_byte(0xA000, 0xAB);
        assert_eq!(cart.read_ram_byte(0xA000), 0xFF);

        cart.write_rom_control(0x0000, 0x0A);
        cart.write_ram_byte(0xA000, 0xAB);
        assert_eq!(cart.read_ram_byte(0xA000), 0xFB);

        // A000 and A200 alias because MBC2 RAM is 512 x 4-bit.
        assert_eq!(cart.read_ram_byte(0xA200), 0xFB);
    }

    #[test]
    fn mbc3_switches_rom_bank_and_maps_zero_to_one() {
        let mut rom = make_rom(64 * 1024, MBC3, 0x01, 0x00);
        rom[0x0000] = 0x10;
        rom[0x4000] = 0x11;
        rom[0x8000] = 0x22;

        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC3 ROM should load");
        assert_eq!(cart.read_rom_byte(0x4000), 0x11);

        cart.write_rom_control(0x2000, 0x02);
        assert_eq!(cart.read_rom_byte(0x4000), 0x22);

        cart.write_rom_control(0x2000, 0x00);
        assert_eq!(cart.read_rom_byte(0x4000), 0x11);
    }

    #[test]
    fn mbc3_banks_ram_and_latches_rtc_registers() {
        let rom = make_rom(256 * 1024, MBC3_TIMER_RAM_BATTERY, 0x03, 0x03);
        let mut cart = Cartridge::from_bytes(rom).expect("valid MBC3 ROM should load");

        cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable

        cart.write_rom_control(0x4000, 0x00);
        cart.write_ram_byte(0xA000, 0x11);
        cart.write_rom_control(0x4000, 0x01);
        cart.write_ram_byte(0xA000, 0x22);
        cart.write_rom_control(0x4000, 0x00);
        assert_eq!(cart.read_ram_byte(0xA000), 0x11);
        cart.write_rom_control(0x4000, 0x01);
        assert_eq!(cart.read_ram_byte(0xA000), 0x22);

        // RTC seconds register select.
        cart.write_rom_control(0x4000, 0x08);
        cart.write_ram_byte(0xA000, 10);

        // Latch 0->1 captures snapshot.
        cart.write_rom_control(0x6000, 0x00);
        cart.write_rom_control(0x6000, 0x01);
        cart.write_ram_byte(0xA000, 20);

        // Reads use latched snapshot until next latch.
        assert_eq!(cart.read_ram_byte(0xA000), 10);
    }

    #[test]
    fn mbc3_rtc_halt_stops_elapsed_time_with_test_clock() {
        let clock = TestClock::new(100);
        let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
        let mut cart = Cartridge::from_bytes_with_clock(rom, Box::new(clock.clone()))
            .expect("valid MBC3 ROM should load");

        cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
        cart.write_rom_control(0x4000, 0x08); // seconds register
        cart.write_ram_byte(0xA000, 10);

        cart.write_rom_control(0x4000, 0x0C); // day high
        cart.write_ram_byte(0xA000, 0x40); // halt

        clock.set_now_epoch_secs(160);
        cart.write_rom_control(0x4000, 0x08);
        assert_eq!(cart.read_ram_byte(0xA000), 10);

        cart.write_rom_control(0x4000, 0x0C);
        cart.write_ram_byte(0xA000, 0x00); // resume

        clock.set_now_epoch_secs(165);
        cart.write_rom_control(0x4000, 0x08);
        assert_eq!(cart.read_ram_byte(0xA000), 15);
    }

    #[test]
    fn mbc3_rtc_latch_snapshot_is_stable_until_next_latch_with_test_clock() {
        let clock = TestClock::new(10);
        let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
        let mut cart = Cartridge::from_bytes_with_clock(rom, Box::new(clock.clone()))
            .expect("valid MBC3 ROM should load");

        cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
        cart.write_rom_control(0x4000, 0x08); // seconds register
        cart.write_ram_byte(0xA000, 0);

        clock.set_now_epoch_secs(15);
        cart.write_rom_control(0x6000, 0x00);
        cart.write_rom_control(0x6000, 0x01);

        clock.set_now_epoch_secs(20);
        cart.write_rom_control(0x4000, 0x08);
        assert_eq!(cart.read_ram_byte(0xA000), 5);

        cart.write_rom_control(0x6000, 0x00);
        cart.write_rom_control(0x6000, 0x01);
        assert_eq!(cart.read_ram_byte(0xA000), 10);
    }

    #[test]
    fn mbc3_rtc_day_counter_sets_carry_after_overflow_with_test_clock() {
        let clock = TestClock::new(0);
        let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
        let mut cart = Cartridge::from_bytes_with_clock(rom, Box::new(clock.clone()))
            .expect("valid MBC3 ROM should load");

        cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
        cart.write_rom_control(0x4000, 0x0B); // day low
        cart.write_ram_byte(0xA000, 0xFF);
        cart.write_rom_control(0x4000, 0x0C); // day high
        cart.write_ram_byte(0xA000, 0x01); // day bit 8 = 1 => 511 days

        clock.set_now_epoch_secs(86_400);
        cart.write_rom_control(0x4000, 0x0B);
        assert_eq!(cart.read_ram_byte(0xA000), 0x00);
        cart.write_rom_control(0x4000, 0x0C);
        let day_high = cart.read_ram_byte(0xA000);
        assert_eq!(day_high & 0x01, 0x00);
        assert_eq!(day_high & 0x80, 0x80);
    }

    #[test]
    fn mbc3_timer_battery_persists_rtc_sidecar() {
        let rom_path = unique_temp_file_path("mbc3_timer_rtc", "gb");
        let save_path = rom_path.with_extension("sav");
        let rtc_path = rom_path.with_extension("rtc");
        let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let mut first_load = Cartridge::from_file(&rom_path).expect("cartridge should load");
        first_load.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
        first_load.write_rom_control(0x4000, 0x0C); // RTC day high
        first_load.write_ram_byte(0xA000, 0x40); // halt
        first_load.write_rom_control(0x4000, 0x08); // RTC seconds
        first_load.write_ram_byte(0xA000, 33);
        first_load.flush_save().expect("flush should persist rtc");
        assert!(!save_path.exists());
        assert!(rtc_path.exists());

        let mut second_load = Cartridge::from_file(&rom_path).expect("cartridge should reload");
        second_load.write_rom_control(0x0000, 0x0A);
        second_load.write_rom_control(0x4000, 0x0C);
        assert_eq!(second_load.read_ram_byte(0xA000) & 0x40, 0x40);
        second_load.write_rom_control(0x4000, 0x08);
        assert_eq!(second_load.read_ram_byte(0xA000), 33);

        let _ = fs::remove_file(rtc_path);
        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(rom_path);
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

    #[test]
    fn persists_mbc2_battery_ram_to_sav_file() {
        let rom_path = unique_temp_file_path("mbc2_save", "gb");
        let save_path = rom_path.with_extension("sav");
        let rom = make_rom(64 * 1024, MBC2_BATTERY, 0x01, 0x00);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let mut first = Cartridge::from_file(&rom_path).expect("cartridge should load");
        first.write_rom_control(0x0000, 0x0A);
        first.write_ram_byte(0xA123, 0xA5);
        first.flush_save().expect("flush should persist save");
        assert!(save_path.exists());

        let mut second = Cartridge::from_file(&rom_path).expect("cartridge should reload");
        second.write_rom_control(0x0000, 0x0A);
        assert_eq!(second.read_ram_byte(0xA123), 0xF5);

        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(rom_path);
    }

    #[test]
    fn atomic_save_writer_replaces_existing_file_without_temp_leaks() {
        let save_path = unique_temp_file_path("atomic_save_replace", "sav");
        fs::write(&save_path, [0xAA, 0xBB]).expect("initial write should work");

        write_file_atomic(&save_path, &[0x11, 0x22, 0x33]).expect("atomic write should work");
        let data = fs::read(&save_path).expect("read should work");
        assert_eq!(data, vec![0x11, 0x22, 0x33]);

        let parent = save_path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = save_path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("temp save path should have a utf8 name");
        let tmp_prefix = format!(".{file_name}.tmp.");
        let has_temp_files = fs::read_dir(parent)
            .expect("read_dir should work")
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .any(|name| name.starts_with(&tmp_prefix));
        assert!(!has_temp_files);

        let _ = fs::remove_file(save_path);
    }
}
