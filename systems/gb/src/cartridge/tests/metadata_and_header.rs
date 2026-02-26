use super::super::capabilities;
use super::super::*;
use super::support::*;

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
