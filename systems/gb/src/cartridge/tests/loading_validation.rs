use super::super::*;
use super::support::*;

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
