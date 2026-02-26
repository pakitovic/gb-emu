use super::Bus;
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;

pub(in crate::memory) fn make_test_bus() -> Bus {
    make_test_bus_with_model(HardwareModel::default())
}

pub(in crate::memory) fn make_test_bus_with_model(model: HardwareModel) -> Bus {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    let mut bus = Bus::new_with_model(cart, model);
    // Unit tests use a neutral baseline instead of post-boot runtime defaults.
    bus.write_byte(0xFF04, 0x00); // DIV
    bus.write_byte(0xFF44, 0x00); // LY
    bus
}

pub(in crate::memory) fn tick_n(bus: &mut Bus, ticks: usize) {
    let mut remaining = ticks;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as usize) as u8;
        bus.tick(chunk);
        remaining -= chunk as usize;
    }
}
