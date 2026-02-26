use std::fmt::{Display, Formatter};

mod capabilities;
mod clock;
mod persistence;

pub(crate) use self::capabilities::CartridgeCapabilities;
use self::clock::{FixedRtcClock, RtcClock, SystemRtcClock};

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
const HEADER_LOGO_START: usize = 0x0104;
const HEADER_LOGO_END: usize = 0x0133;
const HEADER_CHECKSUM_START: usize = 0x0134;
const HEADER_CHECKSUM_END: usize = 0x014C;
const HEADER_CHECKSUM_OFFSET: usize = 0x014D;
const GLOBAL_CHECKSUM_HIGH_OFFSET: usize = 0x014E;
const GLOBAL_CHECKSUM_LOW_OFFSET: usize = 0x014F;
const NINTENDO_LOGO_BYTES: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

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

impl CartridgeMetadata {
    pub fn debug_report(&self) -> String {
        let mut lines = Vec::with_capacity(12 + self.header_warnings.len());
        let title = if self.title.trim().is_empty() {
            "<empty title>".to_string()
        } else {
            self.title.clone()
        };
        lines.push("Cartridge Metadata".to_string());
        lines.push(format!("Title: {title}"));
        lines.push(format!(
            "Type: 0x{:02X} ({})",
            self.cart_type_code, self.mapper
        ));
        lines.push(format!(
            "ROM: code 0x{:02X}, {} bytes, {} banks",
            self.rom_size_code, self.rom_size_bytes, self.rom_bank_count
        ));
        lines.push(format!(
            "RAM: code 0x{:02X}, declared {} bytes, effective {} bytes, {} banks",
            self.ram_size_code,
            self.declared_ram_size_bytes,
            self.effective_ram_size_bytes,
            self.ram_bank_count
        ));
        lines.push(format!(
            "Compatibility RAM mode: {}",
            yes_no(self.compatibility_ram_mode)
        ));
        lines.push(format!(
            "Capabilities: battery={}, timer={}, rumble={} (active={}), battery-save={}",
            yes_no(self.has_battery),
            yes_no(self.has_timer),
            yes_no(self.has_rumble),
            yes_no(self.rumble_active),
            yes_no(self.has_battery_save)
        ));
        lines.push(format!("Header warnings ({}):", self.header_warnings.len()));
        if self.header_warnings.is_empty() {
            lines.push("- none".to_string());
        } else {
            for warning in &self.header_warnings {
                lines.push(format!("- {warning}"));
            }
        }
        lines.join("\n")
    }
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

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
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
    cart_type_code: u8,
    rom_size_code: u8,
    ram_size_code: u8,
    declared_ram_size_bytes: usize,
    compatibility_ram_mode: bool,
    header_warnings: Vec<CartridgeHeaderWarning>,
    mapper: MapperType,
    rom_bank_count: usize,
    ram: Vec<u8>,
    ram_bank_count: usize,
    has_battery: bool,
    has_timer: bool,
    has_rumble: bool,
    rumble_active: bool,
    clock: Box<dyn RtcClock>,
    host_rtc_epoch_secs: Option<u64>,
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
    pub fn from_bytes(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        Self::from_bytes_with_clock(rom, Box::new(SystemRtcClock))
    }

    pub fn from_bytes_with_initial_rtc_epoch(
        rom: Vec<u8>,
        rtc_epoch_secs: u64,
    ) -> Result<Self, CartridgeError> {
        Self::from_bytes_with_clock(rom, Box::new(FixedRtcClock::new(rtc_epoch_secs)))
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
        let header_warnings = diagnose_header(&rom);
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
        let rtc = if spec.has_timer {
            Some(Mbc3Rtc::new(clock.now_epoch_secs()))
        } else {
            None
        };

        Ok(Self {
            rom,
            title,
            cart_type_code: cart_type,
            rom_size_code,
            ram_size_code,
            declared_ram_size_bytes: ram_size_bytes,
            compatibility_ram_mode: compatibility_ram,
            header_warnings,
            mapper: spec.mapper,
            rom_bank_count,
            ram,
            ram_bank_count,
            has_battery: spec.has_battery,
            has_timer: spec.has_timer,
            has_rumble,
            rumble_active: false,
            clock,
            host_rtc_epoch_secs: None,
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

    pub fn set_host_rtc_epoch_secs(&mut self, epoch_secs: Option<u64>) {
        self.host_rtc_epoch_secs = epoch_secs;
    }

    fn current_rtc_epoch_secs(&self) -> u64 {
        self.host_rtc_epoch_secs
            .unwrap_or_else(|| self.clock.now_epoch_secs())
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
                    let now_epoch_secs = self.current_rtc_epoch_secs();
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
                let now_epoch_secs = self.current_rtc_epoch_secs();
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
                let now_epoch_secs = self.current_rtc_epoch_secs();
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

    pub fn has_battery_save(&self) -> bool {
        self.has_battery && (!self.ram.is_empty() || self.has_timer)
    }

    pub fn metadata(&self) -> CartridgeMetadata {
        let capabilities = self.capabilities();
        CartridgeMetadata {
            title: self.title.clone(),
            cart_type_code: self.cart_type_code,
            mapper: capabilities.mapper,
            rom_size_code: self.rom_size_code,
            ram_size_code: self.ram_size_code,
            rom_size_bytes: self.rom.len(),
            rom_bank_count: self.rom_bank_count,
            declared_ram_size_bytes: self.declared_ram_size_bytes,
            effective_ram_size_bytes: self.ram.len(),
            ram_bank_count: self.ram_bank_count,
            compatibility_ram_mode: capabilities.compatibility_ram_mode,
            has_battery: capabilities.has_battery,
            has_timer: capabilities.has_timer,
            has_rumble: capabilities.has_rumble,
            has_battery_save: capabilities.has_battery_save,
            rumble_active: self.rumble_active(),
            header_warnings: self.header_warnings.clone(),
        }
    }

    pub fn header_warnings(&self) -> &[CartridgeHeaderWarning] {
        &self.header_warnings
    }

    pub fn has_rumble(&self) -> bool {
        self.has_rumble
    }

    pub fn rumble_active(&self) -> bool {
        self.has_rumble && self.rumble_active
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

fn public_mapper(mapper: MapperType) -> CartridgeMapper {
    match mapper {
        MapperType::RomOnly => CartridgeMapper::RomOnly,
        MapperType::Mbc1 => CartridgeMapper::Mbc1,
        MapperType::Mbc2 => CartridgeMapper::Mbc2,
        MapperType::Mbc3 => CartridgeMapper::Mbc3,
        MapperType::Mbc5 => CartridgeMapper::Mbc5,
    }
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

fn diagnose_header(rom: &[u8]) -> Vec<CartridgeHeaderWarning> {
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

fn compute_header_checksum(rom: &[u8]) -> u8 {
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

fn compute_global_checksum(rom: &[u8]) -> u16 {
    rom.iter().enumerate().fold(0u16, |acc, (index, byte)| {
        if index == GLOBAL_CHECKSUM_HIGH_OFFSET || index == GLOBAL_CHECKSUM_LOW_OFFSET {
            acc
        } else {
            acc.wrapping_add(*byte as u16)
        }
    })
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

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

    fn apply_valid_header_signature(rom: &mut [u8]) {
        rom[HEADER_LOGO_START..=HEADER_LOGO_END].copy_from_slice(&NINTENDO_LOGO_BYTES);
        let header_checksum = compute_header_checksum(rom);
        rom[HEADER_CHECKSUM_OFFSET] = header_checksum;
        let global_checksum = compute_global_checksum(rom);
        rom[GLOBAL_CHECKSUM_HIGH_OFFSET] = (global_checksum >> 8) as u8;
        rom[GLOBAL_CHECKSUM_LOW_OFFSET] = global_checksum as u8;
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

    struct PanicClock;

    impl RtcClock for PanicClock {
        fn now_epoch_secs(&self) -> u64 {
            panic!("ROM loading without RTC support should not query the clock");
        }
    }

    #[derive(Clone, Copy)]
    struct MapperConformanceCase {
        name: &'static str,
        cart_type: u8,
        rom_size_code: u8,
        ram_size_code: u8,
        expected_mapper: MapperType,
        expected_ram_bytes: usize,
        has_battery: bool,
        has_timer: bool,
        has_rumble: bool,
    }

    fn mapper_conformance_cases() -> [MapperConformanceCase; 19] {
        [
            MapperConformanceCase {
                name: "ROM_ONLY",
                cart_type: ROM_ONLY,
                rom_size_code: 0x00,
                ram_size_code: 0x00,
                expected_mapper: MapperType::RomOnly,
                expected_ram_bytes: RAM_BANK_BYTES,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "ROM_RAM",
                cart_type: ROM_RAM,
                rom_size_code: 0x00,
                ram_size_code: 0x02,
                expected_mapper: MapperType::RomOnly,
                expected_ram_bytes: 8 * 1024,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "ROM_RAM_BATTERY",
                cart_type: ROM_RAM_BATTERY,
                rom_size_code: 0x00,
                ram_size_code: 0x03,
                expected_mapper: MapperType::RomOnly,
                expected_ram_bytes: 32 * 1024,
                has_battery: true,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC1",
                cart_type: MBC1,
                rom_size_code: 0x01,
                ram_size_code: 0x00,
                expected_mapper: MapperType::Mbc1,
                expected_ram_bytes: 0,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC1_RAM",
                cart_type: MBC1_RAM,
                rom_size_code: 0x01,
                ram_size_code: 0x02,
                expected_mapper: MapperType::Mbc1,
                expected_ram_bytes: 8 * 1024,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC1_RAM_BATTERY",
                cart_type: MBC1_RAM_BATTERY,
                rom_size_code: 0x01,
                ram_size_code: 0x03,
                expected_mapper: MapperType::Mbc1,
                expected_ram_bytes: 32 * 1024,
                has_battery: true,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC2",
                cart_type: MBC2,
                rom_size_code: 0x01,
                ram_size_code: 0x00,
                expected_mapper: MapperType::Mbc2,
                expected_ram_bytes: MBC2_RAM_BYTES,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC2_BATTERY",
                cart_type: MBC2_BATTERY,
                rom_size_code: 0x01,
                ram_size_code: 0x00,
                expected_mapper: MapperType::Mbc2,
                expected_ram_bytes: MBC2_RAM_BYTES,
                has_battery: true,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC3",
                cart_type: MBC3,
                rom_size_code: 0x01,
                ram_size_code: 0x00,
                expected_mapper: MapperType::Mbc3,
                expected_ram_bytes: 0,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC3_RAM",
                cart_type: MBC3_RAM,
                rom_size_code: 0x01,
                ram_size_code: 0x02,
                expected_mapper: MapperType::Mbc3,
                expected_ram_bytes: 8 * 1024,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC3_RAM_BATTERY",
                cart_type: MBC3_RAM_BATTERY,
                rom_size_code: 0x01,
                ram_size_code: 0x03,
                expected_mapper: MapperType::Mbc3,
                expected_ram_bytes: 32 * 1024,
                has_battery: true,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC3_TIMER_BATTERY",
                cart_type: MBC3_TIMER_BATTERY,
                rom_size_code: 0x01,
                ram_size_code: 0x00,
                expected_mapper: MapperType::Mbc3,
                expected_ram_bytes: 0,
                has_battery: true,
                has_timer: true,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC3_TIMER_RAM_BATTERY",
                cart_type: MBC3_TIMER_RAM_BATTERY,
                rom_size_code: 0x01,
                ram_size_code: 0x03,
                expected_mapper: MapperType::Mbc3,
                expected_ram_bytes: 32 * 1024,
                has_battery: true,
                has_timer: true,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC5",
                cart_type: MBC5,
                rom_size_code: 0x01,
                ram_size_code: 0x00,
                expected_mapper: MapperType::Mbc5,
                expected_ram_bytes: 0,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC5_RAM",
                cart_type: MBC5_RAM,
                rom_size_code: 0x01,
                ram_size_code: 0x02,
                expected_mapper: MapperType::Mbc5,
                expected_ram_bytes: 8 * 1024,
                has_battery: false,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC5_RAM_BATTERY",
                cart_type: MBC5_RAM_BATTERY,
                rom_size_code: 0x01,
                ram_size_code: 0x04,
                expected_mapper: MapperType::Mbc5,
                expected_ram_bytes: 128 * 1024,
                has_battery: true,
                has_timer: false,
                has_rumble: false,
            },
            MapperConformanceCase {
                name: "MBC5_RUMBLE",
                cart_type: MBC5_RUMBLE,
                rom_size_code: 0x01,
                ram_size_code: 0x00,
                expected_mapper: MapperType::Mbc5,
                expected_ram_bytes: 0,
                has_battery: false,
                has_timer: false,
                has_rumble: true,
            },
            MapperConformanceCase {
                name: "MBC5_RUMBLE_RAM",
                cart_type: MBC5_RUMBLE_RAM,
                rom_size_code: 0x01,
                ram_size_code: 0x02,
                expected_mapper: MapperType::Mbc5,
                expected_ram_bytes: 8 * 1024,
                has_battery: false,
                has_timer: false,
                has_rumble: true,
            },
            MapperConformanceCase {
                name: "MBC5_RUMBLE_RAM_BATTERY",
                cart_type: MBC5_RUMBLE_RAM_BATTERY,
                rom_size_code: 0x01,
                ram_size_code: 0x03,
                expected_mapper: MapperType::Mbc5,
                expected_ram_bytes: 32 * 1024,
                has_battery: true,
                has_timer: false,
                has_rumble: true,
            },
        ]
    }

    #[test]
    fn rom_only_loading_does_not_query_rtc_clock() {
        let rom = make_rom(32 * 1024, ROM_ONLY, 0x00, 0x00);

        let cart = Cartridge::from_bytes_with_clock(rom, Box::new(PanicClock))
            .expect("ROM-only cartridge should load without consulting RTC clock");

        assert_eq!(cart.mapper, MapperType::RomOnly);
        assert!(!cart.has_timer);
    }

    #[test]
    fn supported_cartridge_type_matrix_matches_expected_capabilities() {
        for case in mapper_conformance_cases() {
            let rom_len = rom_size_bytes_from_code(case.rom_size_code)
                .expect("all test cases use valid ROM size codes");
            let rom = make_rom(
                rom_len,
                case.cart_type,
                case.rom_size_code,
                case.ram_size_code,
            );
            let cart = Cartridge::from_bytes(rom)
                .unwrap_or_else(|err| panic!("{} should load successfully: {err}", case.name));

            assert_eq!(cart.mapper, case.expected_mapper, "{} mapper", case.name);
            assert_eq!(
                cart.ram.len(),
                case.expected_ram_bytes,
                "{} RAM bytes",
                case.name
            );
            assert_eq!(cart.has_battery, case.has_battery, "{} battery", case.name);
            assert_eq!(cart.has_timer, case.has_timer, "{} timer", case.name);
            assert_eq!(
                cart.has_rumble(),
                case.has_rumble,
                "{} rumble flag",
                case.name
            );
            assert!(
                !cart.rumble_active(),
                "{} rumble starts disabled",
                case.name
            );

            let expected_battery_save =
                case.has_battery && (case.expected_ram_bytes > 0 || case.has_timer);
            assert_eq!(
                cart.has_battery_save(),
                expected_battery_save,
                "{} battery save capability",
                case.name
            );
        }
    }

    #[test]
    fn mapper_matrix_rejects_invalid_ram_size_combinations() {
        let invalid_cases = [
            ("ROM_ONLY", ROM_ONLY, 0x00),
            ("MBC1", MBC1, 0x01),
            ("MBC2", MBC2, 0x01),
            ("MBC3", MBC3, 0x01),
            ("MBC3_TIMER_BATTERY", MBC3_TIMER_BATTERY, 0x01),
            ("MBC5", MBC5, 0x01),
            ("MBC5_RUMBLE", MBC5_RUMBLE, 0x01),
        ];

        for (name, cart_type, rom_size_code) in invalid_cases {
            let rom_len = rom_size_bytes_from_code(rom_size_code)
                .expect("invalid matrix cases use valid ROM size codes");
            let rom = make_rom(rom_len, cart_type, rom_size_code, 0x02);
            match Cartridge::from_bytes(rom) {
                Err(CartridgeError::UnsupportedRamSizeForCartridge {
                    cart_type: actual_type,
                    ram_size_code,
                }) => {
                    assert_eq!(actual_type, cart_type, "{name} cart type mismatch");
                    assert_eq!(ram_size_code, 0x02, "{name} RAM code mismatch");
                }
                Err(other) => {
                    panic!("{name} should reject with RAM-size compatibility error: {other}")
                }
                Ok(_) => panic!("{name} should reject non-zero RAM size code"),
            }
        }
    }

    #[test]
    fn metadata_reports_capabilities_for_mbc3_timer_ram_battery() {
        let rom = make_rom(64 * 1024, MBC3_TIMER_RAM_BATTERY, 0x01, 0x03);
        let cart = Cartridge::from_bytes(rom).expect("valid MBC3 timer+RAM ROM should load");
        let metadata = cart.metadata();

        assert_eq!(metadata.cart_type_code, MBC3_TIMER_RAM_BATTERY);
        assert_eq!(metadata.mapper, CartridgeMapper::Mbc3);
        assert_eq!(metadata.rom_size_code, 0x01);
        assert_eq!(metadata.ram_size_code, 0x03);
        assert_eq!(metadata.rom_size_bytes, 64 * 1024);
        assert_eq!(metadata.rom_bank_count, 4);
        assert_eq!(metadata.declared_ram_size_bytes, 32 * 1024);
        assert_eq!(metadata.effective_ram_size_bytes, 32 * 1024);
        assert_eq!(metadata.ram_bank_count, 4);
        assert!(!metadata.compatibility_ram_mode);
        assert!(metadata.has_battery);
        assert!(metadata.has_timer);
        assert!(!metadata.has_rumble);
        assert!(metadata.has_battery_save);
        assert!(!metadata.rumble_active);
    }

    #[test]
    fn metadata_marks_rom_only_compatibility_ram_mode() {
        let rom = make_rom(32 * 1024, ROM_ONLY, 0x00, 0x00);
        let cart = Cartridge::from_bytes(rom).expect("valid ROM-only ROM should load");
        let metadata = cart.metadata();

        assert_eq!(metadata.cart_type_code, ROM_ONLY);
        assert_eq!(metadata.mapper, CartridgeMapper::RomOnly);
        assert_eq!(metadata.declared_ram_size_bytes, 0);
        assert_eq!(metadata.effective_ram_size_bytes, RAM_BANK_BYTES);
        assert_eq!(metadata.ram_bank_count, 1);
        assert!(metadata.compatibility_ram_mode);
        assert!(!metadata.has_battery);
        assert!(!metadata.has_timer);
        assert!(!metadata.has_rumble);
        assert!(!metadata.has_battery_save);
        assert!(!metadata.rumble_active);
    }

    #[test]
    fn capabilities_report_mapper_flags_and_cgb_header_support() {
        let mut rom = make_rom(64 * 1024, MBC3_TIMER_RAM_BATTERY, 0x01, 0x03);
        rom[0x0143] = 0x80; // CGB-compatible flag (non-CGB behavior still unchanged in current scope)
        let cart = Cartridge::from_bytes(rom).expect("valid MBC3 timer+RAM ROM should load");
        let capabilities = cart.capabilities();

        assert_eq!(capabilities.mapper, CartridgeMapper::Mbc3);
        assert!(capabilities.has_declared_ram);
        assert!(capabilities.has_effective_ram);
        assert!(!capabilities.compatibility_ram_mode);
        assert!(capabilities.has_battery);
        assert!(capabilities.has_timer);
        assert!(!capabilities.has_rumble);
        assert!(capabilities.has_battery_save);
        assert_eq!(capabilities.cgb_header_flag_raw, 0x80);
        assert_eq!(
            capabilities.cgb_support,
            capabilities::CartridgeCgbSupport::Supported
        );
        assert!(capabilities.supports_cgb);
        assert!(!capabilities.cgb_only);
    }

    #[test]
    fn capabilities_distinguish_declared_vs_effective_ram_and_cgb_only_header() {
        let mut rom = make_rom(32 * 1024, ROM_ONLY, 0x00, 0x00);
        rom[0x0143] = 0xC0; // CGB-only flag (still DMG-loadable in current scope)
        let cart = Cartridge::from_bytes(rom).expect("valid ROM-only ROM should load");
        let capabilities = cart.capabilities();

        assert_eq!(capabilities.mapper, CartridgeMapper::RomOnly);
        assert!(!capabilities.has_declared_ram);
        assert!(capabilities.has_effective_ram);
        assert!(capabilities.compatibility_ram_mode);
        assert!(!capabilities.has_battery);
        assert!(!capabilities.has_timer);
        assert!(!capabilities.has_rumble);
        assert!(!capabilities.has_battery_save);
        assert_eq!(capabilities.cgb_header_flag_raw, 0xC0);
        assert_eq!(
            capabilities.cgb_support,
            capabilities::CartridgeCgbSupport::Required
        );
        assert!(capabilities.supports_cgb);
        assert!(capabilities.cgb_only);
    }

    #[test]
    fn capabilities_treat_unknown_cgb_header_flags_as_none() {
        let mut rom = make_rom(32 * 1024, ROM_ONLY, 0x00, 0x00);
        rom[0x0143] = 0x42;
        let cart = Cartridge::from_bytes(rom).expect("valid ROM-only ROM should load");
        let capabilities = cart.capabilities();

        assert_eq!(capabilities.cgb_header_flag_raw, 0x42);
        assert_eq!(
            capabilities.cgb_support,
            capabilities::CartridgeCgbSupport::None
        );
        assert!(!capabilities.supports_cgb);
        assert!(!capabilities.cgb_only);
    }

    #[test]
    fn metadata_debug_report_formats_core_fields_and_warnings() {
        let metadata = CartridgeMetadata {
            title: "TESTROM".to_string(),
            cart_type_code: 0x03,
            mapper: CartridgeMapper::Mbc1,
            rom_size_code: 0x01,
            ram_size_code: 0x03,
            rom_size_bytes: 64 * 1024,
            rom_bank_count: 4,
            declared_ram_size_bytes: 32 * 1024,
            effective_ram_size_bytes: 32 * 1024,
            ram_bank_count: 4,
            compatibility_ram_mode: false,
            has_battery: true,
            has_timer: false,
            has_rumble: false,
            has_battery_save: true,
            rumble_active: false,
            header_warnings: vec![
                CartridgeHeaderWarning::NintendoLogoMismatch,
                CartridgeHeaderWarning::HeaderChecksumMismatch {
                    header_value: 0xAA,
                    computed_value: 0xBB,
                },
            ],
        };

        let report = metadata.debug_report();
        assert!(report.contains("Cartridge Metadata"));
        assert!(report.contains("Title: TESTROM"));
        assert!(report.contains("Type: 0x03 (MBC1)"));
        assert!(report.contains("Header warnings (2):"));
        assert!(report.contains("- Nintendo logo mismatch"));
        assert!(report.contains("- Header checksum mismatch (header 0xAA, computed 0xBB)"));
    }

    #[test]
    fn metadata_debug_report_marks_empty_warning_list() {
        let metadata = CartridgeMetadata {
            title: String::new(),
            cart_type_code: ROM_ONLY,
            mapper: CartridgeMapper::RomOnly,
            rom_size_code: 0x00,
            ram_size_code: 0x00,
            rom_size_bytes: 32 * 1024,
            rom_bank_count: 2,
            declared_ram_size_bytes: 0,
            effective_ram_size_bytes: RAM_BANK_BYTES,
            ram_bank_count: 1,
            compatibility_ram_mode: true,
            has_battery: false,
            has_timer: false,
            has_rumble: false,
            has_battery_save: false,
            rumble_active: false,
            header_warnings: Vec::new(),
        };

        let report = metadata.debug_report();
        assert!(report.contains("Title: <empty title>"));
        assert!(report.contains("Header warnings (0):"));
        assert!(report.contains("- none"));
    }

    #[test]
    fn header_diagnostics_warn_but_do_not_block_loading() {
        let rom = make_rom(64 * 1024, MBC1, 0x01, 0x00);
        let cart = Cartridge::from_bytes(rom).expect("invalid header should still load");
        let warnings = cart.header_warnings();

        assert!(
            warnings
                .iter()
                .any(|warning| matches!(warning, CartridgeHeaderWarning::NintendoLogoMismatch))
        );
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            CartridgeHeaderWarning::HeaderChecksumMismatch { .. }
        )));
        assert!(warnings.iter().any(|warning| matches!(
            warning,
            CartridgeHeaderWarning::GlobalChecksumMismatch { .. }
        )));
    }

    #[test]
    fn header_diagnostics_accept_valid_logo_and_checksums() {
        let mut rom = make_rom(64 * 1024, MBC1, 0x01, 0x00);
        apply_valid_header_signature(&mut rom);

        let cart = Cartridge::from_bytes(rom).expect("valid header should load");
        assert!(cart.header_warnings().is_empty());
        assert!(cart.metadata().header_warnings.is_empty());
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
    fn save_ram_persistence_bytes_roundtrip_restores_battery_ram() {
        let rom = make_rom(64 * 1024, MBC1_RAM_BATTERY, 0x01, 0x02);
        let mut first = Cartridge::from_bytes(rom.clone()).expect("cartridge should load");
        first.write_rom_control(0x0000, 0x0A);
        first.write_ram_byte(0xA000, 0x5A);
        first.write_ram_byte(0xA123, 0xC3);

        let persisted = first
            .export_save_ram_bytes()
            .expect("battery-backed RAM should export persistence bytes");

        let mut second = Cartridge::from_bytes(rom).expect("cartridge should load");
        second.import_save_ram_bytes(&persisted);
        second.write_rom_control(0x0000, 0x0A);
        assert_eq!(second.read_ram_byte(0xA000), 0x5A);
        assert_eq!(second.read_ram_byte(0xA123), 0xC3);
    }

    #[test]
    fn rtc_persistence_bytes_roundtrip_restores_mbc3_rtc_state() {
        let clock = TestClock::new(100);
        let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
        let mut first = Cartridge::from_bytes_with_clock(rom.clone(), Box::new(clock.clone()))
            .expect("cartridge should load");
        first.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
        first.write_rom_control(0x4000, 0x0C); // day high
        first.write_ram_byte(0xA000, 0x40); // halt
        first.write_rom_control(0x4000, 0x08); // seconds
        first.write_ram_byte(0xA000, 33);

        let rtc_bytes = first
            .export_rtc_persistence_bytes()
            .expect("MBC3 timer cartridge should export RTC persistence bytes");

        let mut second = Cartridge::from_bytes_with_clock(rom, Box::new(TestClock::new(100)))
            .expect("cartridge should load");
        assert!(second.import_rtc_persistence_bytes(&rtc_bytes));

        second.write_rom_control(0x0000, 0x0A);
        second.write_rom_control(0x4000, 0x0C);
        assert_eq!(second.read_ram_byte(0xA000) & 0x40, 0x40);
        second.write_rom_control(0x4000, 0x08);
        assert_eq!(second.read_ram_byte(0xA000), 33);
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
}
