use super::*;
use crate::bootrom::BOOT_ROM_WINDOW_SIZE;
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;

fn make_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    rom
}

#[test]
fn boot_rom_maps_first_window_until_ff50_bit0_disables_it() {
    let mut rom = make_rom_32kb();
    rom[0x0000] = 0x99;
    rom[0x00FF] = 0x55;
    let cart = Cartridge::from_bytes(rom).expect("test ROM should load");

    let mut boot_rom = [0x00; BOOT_ROM_WINDOW_SIZE];
    boot_rom[0x0000] = 0x11;
    boot_rom[0x00FF] = 0x22;

    let mut bus = Bus::new_with_model_and_boot_rom(cart, HardwareModel::Dmg, Some(boot_rom));
    assert_eq!(bus.read_byte(0x0000), 0x11);
    assert_eq!(bus.read_byte(0x00FF), 0x22);

    bus.write_byte(0xFF50, 0x00);
    assert_eq!(bus.read_byte(0x0000), 0x11);

    bus.write_byte(0xFF50, 0x01);
    assert_eq!(bus.read_byte(0x0000), 0x99);
    assert_eq!(bus.read_byte(0x00FF), 0x55);

    bus.write_byte(0xFF50, 0x00);
    assert_eq!(bus.read_byte(0x0000), 0x99);
}

#[test]
fn missing_boot_rom_keeps_existing_post_boot_defaults() {
    let cart = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
    let bus = Bus::new_with_model_and_boot_rom(cart, HardwareModel::Dmg, None);
    assert_eq!(bus.read_byte(0xFF40), 0x91);
    assert_eq!(bus.read_byte(0xFF47), 0xFC);
    assert_eq!(bus.read_byte(0xFF04), 0xAB);
}
