use super::GameBoy;
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

#[test]
fn run_frame_with_limit_returns_when_lcd_is_disabled_for_one_frame_budget() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
    let mut gb = GameBoy::new(cartridge);
    let start = gb.frame_counter();
    gb.bus.write_byte(0xFF40, 0x00);

    let cycles = gb
        .run_frame_with_limit(200_000)
        .expect("lcd-off budget should still advance runtime pacing");

    assert!(cycles >= crate::timing::DMG_T_CYCLES_PER_FRAME);
    assert_eq!(gb.frame_counter(), start);
}

#[test]
fn boot_rom_executes_at_reset_and_hands_off_to_cartridge_after_ff50_disable() {
    let mut rom = make_rom_32kb();
    rom[0x0004] = 0x3E; // LD A, d8
    rom[0x0005] = 0x77;
    let cartridge = Cartridge::from_bytes(rom).expect("test ROM should load");

    let mut boot_rom = [0x00; BOOT_ROM_WINDOW_SIZE];
    boot_rom[0x0000] = 0x3E; // LD A, d8
    boot_rom[0x0001] = 0x01;
    boot_rom[0x0002] = 0xE0; // LDH (0xFF50), A
    boot_rom[0x0003] = 0x50;

    let mut gb =
        GameBoy::new_with_model_and_boot_rom(cartridge, HardwareModel::Dmg, Some(boot_rom));
    assert_eq!(gb.cpu().registers().pc, 0x0000);

    gb.step();
    assert_eq!(gb.cpu().registers().a, 0x01);
    assert_eq!(gb.cpu().registers().pc, 0x0002);

    gb.step();
    assert_eq!(gb.cpu().registers().pc, 0x0004);

    gb.step();
    assert_eq!(gb.cpu().registers().a, 0x77);
    assert_eq!(gb.cpu().registers().pc, 0x0006);
}

#[test]
fn recent_pc_trace_keeps_recent_instruction_entry_pcs_in_order() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
    let mut gb = GameBoy::new(cartridge);

    for _ in 0..20 {
        gb.step();
    }

    let trace = gb.recent_pc_trace();
    assert_eq!(trace.len(), 16);
    assert_eq!(trace[0], 0x0104);
    assert_eq!(trace[15], 0x0113);
}

#[test]
fn copy_vram_hardware_block_reads_back_vram_contents() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
    let mut gb = GameBoy::new(cartridge);
    gb.bus.write_byte(0xFF40, 0x00);
    gb.bus.write_byte(0x8000, 0x12);
    gb.bus.write_byte(0x8001, 0x34);

    let mut block = [0u8; 2];
    assert!(gb.copy_vram_hardware_block(0x8000, &mut block));
    assert_eq!(block, [0x12, 0x34]);
    assert!(!gb.copy_vram_hardware_block(0x9FFF, &mut block));
}
