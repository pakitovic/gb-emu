mod api;
mod banking;
mod capabilities;
mod clock;
mod constants;
mod header;
mod load;
mod mbc;
mod persistence;
mod rtc;
mod spec;
mod types;

pub(crate) use self::capabilities::CartridgeCapabilities;
pub use self::types::{CartridgeError, CartridgeHeaderWarning, CartridgeMapper, CartridgeMetadata};

use self::clock::{FixedRtcClock, RtcClock, SystemRtcClock};
use self::constants::*;
#[cfg(test)]
use self::header::{compute_global_checksum, compute_header_checksum};
use self::header::{diagnose_header, parse_title};
use self::rtc::Mbc3Rtc;
use self::spec::{
    cartridge_spec, is_mbc5_rumble_type, mapper_uses_ram_gate, public_mapper,
    ram_size_bytes_from_code, rom_size_bytes_from_code,
};
use self::types::{CartridgeSpec, MapperType};

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

#[cfg(test)]
mod tests;
