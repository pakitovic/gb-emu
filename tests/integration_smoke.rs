use gb_emu::cartridge::{Cartridge, CartridgeError};
use gb_emu::gameboy::GameBoy;

fn make_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    rom
}

#[test]
fn gameboy_step_executes_nop_and_advances_pc() {
    let mut rom = make_rom_32kb();
    rom[0x0100] = 0x00; // NOP

    let cartridge = Cartridge::from_bytes(rom).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);
    let cycles = gb.step();

    assert_eq!(cycles, 4);
    assert_eq!(gb.cpu.registers.pc, 0x0101);
}

#[test]
fn cartridge_rejects_unsupported_type() {
    let mut rom = make_rom_32kb();
    rom[0x0147] = 0xFF;

    let result = Cartridge::from_bytes(rom);
    assert!(matches!(
        result,
        Err(CartridgeError::UnsupportedCartridgeType(0xFF))
    ));
}
