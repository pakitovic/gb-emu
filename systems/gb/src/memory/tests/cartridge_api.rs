use super::*;

#[test]
fn bus_internal_cartridge_capabilities_api_surfaces_header_and_mapper_flags() {
    let mut rom = vec![0; 64 * 1024];
    rom[0x0147] = 0x1E; // MBC5 rumble+RAM+battery
    rom[0x0148] = 0x01; // 64 KiB
    rom[0x0149] = 0x03; // 32 KiB RAM
    rom[0x0143] = 0x80;
    let cartridge = Cartridge::from_bytes(rom).expect("valid cartridge should load");
    let mut bus = Bus::new(cartridge);

    let caps = bus.cartridge_capabilities();
    assert_eq!(caps.mapper, CartridgeMapper::Mbc5);
    assert!(caps.has_declared_ram);
    assert!(caps.has_effective_ram);
    assert!(caps.has_battery);
    assert!(!caps.has_timer);
    assert!(caps.has_rumble);
    assert!(caps.has_battery_save);
    assert!(caps.supports_cgb);
    assert!(!caps.cgb_only);

    bus.write_byte(0x4000, 0x08); // enable rumble bit on MBC5 rumble register
    assert!(bus.rumble_active());
    assert!(bus.cartridge_has_rumble());
}

#[test]
fn bus_cartridge_model_compatibility_uses_header_flags_and_selected_model() {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32 KiB
    rom[0x0149] = 0x00;
    rom[0x0143] = 0x80; // CGB-compatible
    rom[0x0146] = 0x03; // SGB-supported
    let dmg_bus = Bus::new_with_model(
        Cartridge::from_bytes(rom.clone()).expect("valid cartridge should load"),
        HardwareModel::Dmg,
    );
    let sgb_bus = Bus::new_with_model(
        Cartridge::from_bytes(rom).expect("valid cartridge should load"),
        HardwareModel::Sgb,
    );

    let dmg_compat = dmg_bus.cartridge_model_compatibility();
    assert!(dmg_compat.mode_request.prefers_cgb());
    assert!(dmg_compat.dmg_mode_allowed);
    assert!(!dmg_compat.cgb_mode_supported_by_model);
    assert!(!dmg_compat.cgb_mode_possible);
    assert!(dmg_compat.sgb_features_requested);
    assert!(!dmg_compat.sgb_features_supported_by_model);

    let sgb_compat = sgb_bus.cartridge_model_compatibility();
    assert!(sgb_compat.sgb_features_requested);
    assert!(sgb_compat.sgb_features_supported_by_model);
    assert!(sgb_compat.sgb_features_possible);
    assert!(!sgb_compat.cgb_mode_possible);
}
