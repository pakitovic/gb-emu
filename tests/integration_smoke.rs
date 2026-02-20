use gb_emu::audio::AudioMixer;
use gb_emu::cartridge::{Cartridge, CartridgeError};
use gb_emu::gameboy::GameBoy;
use gb_emu::timing::{DMG_T_CYCLES_PER_SECOND, FramePacer};

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

#[test]
fn gameboy_run_frame_with_limit_produces_frame() {
    let rom = make_rom_32kb();
    let cartridge = Cartridge::from_bytes(rom).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);
    let start = gb.frame_counter();

    let cycles = gb
        .run_frame_with_limit(false, 50_000)
        .expect("frame should be produced within step budget");

    assert!(cycles > 0);
    assert!(gb.frame_counter() > start);
}

#[test]
fn frame_pacer_audio_clock_feeds_realtime_audio_block() {
    let mut pacer = FramePacer::default();
    let mut mixer = AudioMixer::new(48_000);

    pacer.consume_emulated_cycles(DMG_T_CYCLES_PER_SECOND / 10);
    let samples = mixer.drain_realtime_block(pacer.drain_audio_tcycles(), 5_000);

    assert_eq!(samples.len(), 5_000);
    assert!(samples.iter().all(|sample| *sample == 0.0));
    assert_eq!(mixer.pending_samples(), 0);
    assert_eq!(pacer.drain_audio_tcycles(), 0);
}
