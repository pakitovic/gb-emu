use super::GameBoy;
use crate::cartridge::Cartridge;

fn make_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    rom
}

#[test]
fn run_frame_with_limit_returns_none_if_budget_is_too_small() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
    let mut gb = GameBoy::new(cartridge);
    let start = gb.frame_counter();

    let result = gb.run_frame_with_limit(1);

    assert!(result.is_none());
    assert_eq!(gb.frame_counter(), start);
}

#[test]
fn run_frame_with_limit_advances_frame_counter() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
    let mut gb = GameBoy::new(cartridge);
    let start = gb.frame_counter();

    let cycles = gb
        .run_frame_with_limit(50_000)
        .expect("frame should be produced within step budget");

    assert!(cycles > 0);
    assert!(gb.frame_counter() > start);
}
