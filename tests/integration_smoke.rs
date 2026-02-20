use gb_emu::audio::AudioMixer;
use gb_emu::cartridge::{Cartridge, CartridgeError};
use gb_emu::gameboy::GameBoy;
use gb_emu::timing::{DMG_T_CYCLES_PER_SECOND, FramePacer};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    rom[0x0149] = 0x00; // no external RAM
    rom
}

fn make_mbc1_battery_rom_64kb() -> Vec<u8> {
    let mut rom = vec![0; 64 * 1024];
    rom[0x0147] = 0x03; // MBC1+RAM+BATTERY
    rom[0x0148] = 0x01; // 64KB
    rom[0x0149] = 0x02; // 8KB RAM
    rom
}

fn unique_temp_file_path(name: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("gb_emu_integration_{name}_{pid}_{nanos}.{ext}"))
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

#[test]
fn battery_backed_ram_persists_via_gameboy_bus() {
    let rom_path = unique_temp_file_path("battery_save", "gb");
    let save_path = rom_path.with_extension("sav");
    fs::write(&rom_path, make_mbc1_battery_rom_64kb()).expect("ROM file write should work");

    let cartridge = Cartridge::from_file(&rom_path).expect("cartridge should load");
    let mut gb = GameBoy::new(cartridge);
    gb.bus.write_byte(0x0000, 0x0A); // RAM enable
    gb.bus.write_byte(0xA000, 0x7B);
    gb.flush_battery_save().expect("save flush should succeed");
    drop(gb);

    let cartridge_reload = Cartridge::from_file(&rom_path).expect("reloaded cartridge should load");
    let mut gb_reload = GameBoy::new(cartridge_reload);
    gb_reload.bus.write_byte(0x0000, 0x0A); // RAM enable
    assert_eq!(gb_reload.bus.read_byte(0xA000), 0x7B);

    let _ = fs::remove_file(save_path);
    let _ = fs::remove_file(rom_path);
}
