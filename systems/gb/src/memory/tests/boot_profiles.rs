use super::*;

#[test]
fn dmg0_boot_profile_uses_expected_div_phase_and_ly_start() {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00;
    rom[0x0148] = 0x00;
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    let bus = Bus::new_with_model(cart, HardwareModel::Dmg0);

    assert_eq!(bus.timer.div_counter, 0x1830);
    assert_eq!(bus.io[0x44], 0x91);
    assert_eq!(bus.ppu.ly_counter, 96);
}

#[test]
fn sgb_boot_div_phase_depends_on_header_checksum() {
    let make_bus = |checksum_hi: u8, checksum_lo: u8| {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        rom[0x014E] = checksum_hi;
        rom[0x014F] = checksum_lo;
        let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
        Bus::new_with_model(cart, HardwareModel::Sgb)
    };

    // boot_div-S.gb checksum bytes at 0x014E/0x014F.
    let bus_a = make_bus(0x34, 0x12);
    assert_eq!(bus_a.timer.div_counter, 0xD860);

    // boot_div2-S.gb checksum bytes at 0x014E/0x014F.
    let bus_b = make_bus(0x96, 0xA7);
    assert_eq!(bus_b.timer.div_counter, 0xD850);
}
