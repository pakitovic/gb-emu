use super::super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub(super) fn make_rom(
    size: usize,
    cart_type: u8,
    rom_size_code: u8,
    ram_size_code: u8,
) -> Vec<u8> {
    let mut rom = vec![0; size];
    rom[0x0147] = cart_type;
    rom[0x0148] = rom_size_code;
    rom[0x0149] = ram_size_code;
    rom
}

pub(super) fn fill_each_rom_bank_first_byte(rom: &mut [u8]) {
    let bank_count = rom.len() / ROM_BANK_BYTES;
    for bank in 0..bank_count {
        rom[bank * ROM_BANK_BYTES] = bank as u8;
    }
}

pub(super) fn apply_valid_header_signature(rom: &mut [u8]) {
    rom[HEADER_LOGO_START..=HEADER_LOGO_END].copy_from_slice(&NINTENDO_LOGO_BYTES);
    let header_checksum = compute_header_checksum(rom);
    rom[HEADER_CHECKSUM_OFFSET] = header_checksum;
    let global_checksum = compute_global_checksum(rom);
    rom[GLOBAL_CHECKSUM_HIGH_OFFSET] = (global_checksum >> 8) as u8;
    rom[GLOBAL_CHECKSUM_LOW_OFFSET] = global_checksum as u8;
}

#[derive(Clone)]
pub(super) struct TestClock {
    now_epoch_secs: Arc<AtomicU64>,
}

impl TestClock {
    pub(super) fn new(now_epoch_secs: u64) -> Self {
        Self {
            now_epoch_secs: Arc::new(AtomicU64::new(now_epoch_secs)),
        }
    }

    pub(super) fn set_now_epoch_secs(&self, now_epoch_secs: u64) {
        self.now_epoch_secs.store(now_epoch_secs, Ordering::Relaxed);
    }
}

impl RtcClock for TestClock {
    fn now_epoch_secs(&self) -> u64 {
        self.now_epoch_secs.load(Ordering::Relaxed)
    }
}

pub(super) struct PanicClock;

impl RtcClock for PanicClock {
    fn now_epoch_secs(&self) -> u64 {
        panic!("ROM loading without RTC support should not query the clock");
    }
}

#[derive(Clone, Copy)]
pub(super) struct MapperConformanceCase {
    pub(super) name: &'static str,
    pub(super) cart_type: u8,
    pub(super) rom_size_code: u8,
    pub(super) ram_size_code: u8,
    pub(super) expected_mapper: MapperType,
    pub(super) expected_ram_bytes: usize,
    pub(super) has_battery: bool,
    pub(super) has_timer: bool,
    pub(super) has_rumble: bool,
}

pub(super) fn mapper_conformance_cases() -> [MapperConformanceCase; 19] {
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
