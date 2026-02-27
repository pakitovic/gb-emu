use gb_emu::cartridge::Cartridge;
use gb_emu::gameboy::GameBoy;
use gb_emu::timing::DMG_T_CYCLES_PER_SECOND;
use gb_runtime::audio::{AudioMixer, MixerSource};
use gb_runtime::cartridge_persistence::load_cartridge_from_file;
use gb_runtime::session::RuntimeSession;
use gb_runtime::timing::FramePacer;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    std::env::temp_dir().join(format!("gb_runtime_integration_{name}_{pid}_{nanos}.{ext}"))
}

fn tick_n_tcycles(gb: &mut GameBoy, mut tcycles: usize) {
    while tcycles > 0 {
        let chunk = tcycles.min(u8::MAX as usize) as u8;
        gb.bus.tick(chunk);
        tcycles -= chunk as usize;
    }
}

#[test]
fn frame_pacer_audio_clock_feeds_realtime_audio_block() {
    let mut pacer = FramePacer::default();
    let mut mixer = AudioMixer::new(48_000);

    pacer.consume_emulated_cycles(DMG_T_CYCLES_PER_SECOND / 10);
    let samples = mixer.drain_realtime_block(pacer.drain_audio_tcycles(), 5_000);

    assert_eq!(samples.len(), 10_000);
    assert!(samples.iter().all(|sample| *sample == 0.0));
    assert_eq!(mixer.pending_samples(), 0);
    assert_eq!(pacer.drain_audio_tcycles(), 0);
}

#[test]
fn audio_realtime_core_apu_guard_stays_finite_and_stereo_aligned_under_stress() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);
    let mut mixer = AudioMixer::new(48_000);
    mixer.set_source(MixerSource::CoreApu);
    gb.set_audio_tcycle_stream_enabled(true);

    let mut saw_non_silent_block = false;
    let mut total_output_scalars = 0usize;

    for round in 0..96 {
        gb.bus.write_byte(0xFF26, 0x00);
        gb.bus.write_byte(0xFF26, 0x80);
        gb.bus.write_byte(0xFF24, 0x77);
        gb.bus
            .write_byte(0xFF25, if (round & 1) == 0 { 0x22 } else { 0x11 });

        if (round % 3) == 0 {
            gb.bus.write_byte(0xFF16, 0x80); // CH2 duty/length
            gb.bus.write_byte(0xFF17, 0xF0); // CH2 envelope (DAC on)
            gb.bus.write_byte(0xFF18, 0xFC);
            gb.bus.write_byte(0xFF19, 0x87); // trigger CH2
        } else {
            gb.bus.write_byte(0xFF20, 0x3F); // CH4 length
            gb.bus.write_byte(0xFF21, 0xF0); // CH4 envelope (DAC on)
            gb.bus.write_byte(0xFF22, 0x00); // CH4 polynomial
            gb.bus.write_byte(0xFF23, 0x80); // trigger CH4
        }

        tick_n_tcycles(&mut gb, 3_072);
        let tcycle_samples = gb.drain_audio_tcycle_samples();
        assert_eq!(tcycle_samples.len() % 2, 0);
        assert!(tcycle_samples.iter().all(|sample| sample.is_finite()));
        assert!(
            tcycle_samples
                .iter()
                .all(|sample| sample.abs() <= 1.0 + f32::EPSILON)
        );

        mixer.push_core_tcycle_samples(&tcycle_samples);
        let block = mixer.drain_realtime_block(0, 256);
        assert_eq!(block.len(), 512);
        assert!(block.iter().all(|sample| sample.is_finite()));
        assert!(
            block
                .iter()
                .all(|sample| sample.abs() <= 1.0 + f32::EPSILON)
        );
        saw_non_silent_block |= block.iter().any(|sample| sample.abs() > 0.000_1);
        total_output_scalars = total_output_scalars.saturating_add(block.len());
    }

    assert!(
        saw_non_silent_block,
        "expected non-silent realtime mixer output"
    );
    assert!(total_output_scalars >= 96 * 512);
}

#[test]
fn runtime_session_unifies_frame_budget_and_core_audio_capture() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let gb = GameBoy::new(cartridge);
    let mut session = RuntimeSession::new(gb, 48_000);

    {
        let gb = session.gameboy_mut();
        gb.bus.write_byte(0xFF26, 0x00);
        gb.bus.write_byte(0xFF26, 0x80);
        gb.bus.write_byte(0xFF24, 0x77);
        gb.bus.write_byte(0xFF25, 0x11);
        gb.bus.write_byte(0xFF11, 0x80);
        gb.bus.write_byte(0xFF12, 0xF0);
        gb.bus.write_byte(0xFF13, 0xFC);
        gb.bus.write_byte(0xFF14, 0x87);
    }

    session.push_host_time(Duration::from_millis(17));
    let mut frames = 0u32;
    while session.has_frame_budget() {
        let cycles = session
            .run_frame_with_limit(250_000)
            .expect("frame should be produced within budget");
        assert!(cycles > 0);
        frames = frames.saturating_add(1);
    }
    assert!(frames > 0);

    let block = session.drain_audio_realtime_block(256);
    assert_eq!(block.len(), 512);
    assert!(block.iter().all(|sample| sample.is_finite()));
    assert!(block.iter().any(|sample| sample.abs() > 0.0));
}

#[test]
fn battery_backed_ram_persists_via_gameboy_and_runtime_file_adapter() {
    let rom_path = unique_temp_file_path("battery_save", "gb");
    let save_path = rom_path.with_extension("sav");
    fs::write(&rom_path, make_mbc1_battery_rom_64kb()).expect("ROM file write should work");

    let (cartridge, persistence) = load_cartridge_from_file(&rom_path).expect("load should work");
    let mut gb = GameBoy::new(cartridge);
    gb.bus.write_byte(0x0000, 0x0A); // RAM enable
    gb.bus.write_byte(0xA000, 0x7B);
    persistence
        .flush_gameboy(&mut gb)
        .expect("save flush should succeed");
    drop(gb);

    let (cartridge_reload, _) = load_cartridge_from_file(&rom_path).expect("reload should work");
    let mut gb_reload = GameBoy::new(cartridge_reload);
    gb_reload.bus.write_byte(0x0000, 0x0A); // RAM enable
    assert_eq!(gb_reload.bus.read_byte(0xA000), 0x7B);

    let _ = fs::remove_file(save_path);
    let _ = fs::remove_file(rom_path);
}
