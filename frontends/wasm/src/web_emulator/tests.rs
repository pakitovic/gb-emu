use super::audio::parse_audio_resampler_quality;
use super::*;

fn make_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    rom
}

fn make_mbc1_battery_ram_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x03; // MBC1 + RAM + BATTERY
    rom[0x0148] = 0x00; // 32KB
    rom[0x0149] = 0x02; // 8KB RAM
    rom
}

#[test]
fn constructor_rejects_invalid_model_string() {
    let rom = make_rom_32kb();
    let err = match WebEmulator::new_internal(&rom, Some("cgb")) {
        Ok(_) => panic!("invalid model should be rejected"),
        Err(err) => err,
    };
    assert!(err.contains("Unsupported model"));
}

#[test]
fn constructor_rejects_invalid_rom_bytes() {
    let err = match WebEmulator::new_internal(&[0x00, 0x01, 0x02], None) {
        Ok(_) => panic!("invalid rom bytes should be rejected"),
        Err(err) => err,
    };
    assert!(!err.is_empty());
}

#[test]
fn drain_audio_samples_realtime_returns_fixed_block_len() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
    web.run_frame().expect("a frame should be produced");

    let samples = web.drain_audio_samples_realtime(512);
    assert_eq!(samples.len(), 1_024);
}

#[test]
fn drain_audio_samples_realtime_can_emit_core_apu_signal() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");

    web.gb.bus.write_byte(0xFF26, 0x00);
    web.gb.bus.write_byte(0xFF26, 0x80);
    web.gb.bus.write_byte(0xFF24, 0x77);
    web.gb.bus.write_byte(0xFF25, 0x11);
    web.gb.bus.write_byte(0xFF11, 0x80);
    web.gb.bus.write_byte(0xFF12, 0xF0);
    web.gb.bus.write_byte(0xFF13, 0xFC);
    web.gb.bus.write_byte(0xFF14, 0x87);

    web.run_frame().expect("a frame should be produced");
    let samples = web.drain_audio_samples_realtime(512);
    assert_eq!(samples.len(), 1_024);
    assert!(samples.iter().any(|sample| *sample != 0.0));
}

#[test]
fn set_audio_sample_rate_preserves_pending_core_apu_queue() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");

    web.gb.bus.write_byte(0xFF26, 0x00);
    web.gb.bus.write_byte(0xFF26, 0x80);
    web.gb.bus.write_byte(0xFF24, 0x77);
    web.gb.bus.write_byte(0xFF25, 0x11);
    web.gb.bus.write_byte(0xFF11, 0x80);
    web.gb.bus.write_byte(0xFF12, 0xF0);
    web.gb.bus.write_byte(0xFF13, 0xFC);
    web.gb.bus.write_byte(0xFF14, 0x87);

    web.run_frame().expect("a frame should be produced");

    let before = web.drain_audio_samples_realtime(64);
    assert_eq!(before.len(), 128);
    assert!(before.iter().any(|sample| sample.abs() > 0.0));

    web.set_audio_sample_rate(44_100);

    let after = web.drain_audio_samples_realtime(64);
    assert_eq!(after.len(), 128);
    assert!(after.iter().all(|sample| sample.is_finite()));
    assert!(
        after.iter().any(|sample| sample.abs() > 0.0),
        "expected queued core APU audio to survive sample-rate change"
    );
}

#[test]
fn web_audio_resampler_quality_defaults_to_cubic_and_can_change() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");

    assert_eq!(web.audio_resampler_quality(), "cubic");
    web.set_audio_resampler_quality("linear")
        .expect("linear should be accepted");
    assert_eq!(web.audio_resampler_quality(), "linear");
    web.set_audio_resampler_quality("cubic")
        .expect("cubic should be accepted");
    assert_eq!(web.audio_resampler_quality(), "cubic");
}

#[test]
fn parse_audio_resampler_quality_rejects_invalid_values() {
    assert_eq!(parse_audio_resampler_quality("nearest"), None);
}

#[test]
fn set_button_accepts_valid_index() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
    assert!(web.set_button(4, true).is_ok());
    assert!(web.set_button(4, false).is_ok());
}

#[test]
fn cartridge_debug_report_exposes_metadata_summary() {
    let rom = make_rom_32kb();
    let web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
    let report = web.cartridge_debug_report();

    assert!(report.contains("Cartridge Metadata"));
    assert!(report.contains("Type: 0x00 (ROM-only)"));
    assert!(report.contains("Header warnings"));
    assert!(report.contains("Nintendo logo mismatch"));
    assert!(web.cartridge_warning_count() >= 1);
    assert!(!web.cartridge_has_battery_save());
    assert!(!web.cartridge_has_rtc_persistence());
}

#[test]
fn persistence_api_exposes_save_ram_roundtrip_and_dirty_flag() {
    let rom = make_mbc1_battery_ram_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
    assert!(web.cartridge_has_battery_save());
    assert!(!web.cartridge_has_rtc_persistence());

    assert!(!web.cartridge_battery_save_dirty());
    assert_eq!(
        web.export_cartridge_save_ram_bytes()
            .expect("battery-backed RAM should be exported")
            .len(),
        8 * 1024
    );

    web.gb.bus.write_byte(0x0000, 0x0A); // RAM enable (MBC1)
    web.gb.bus.write_byte(0xA000, 0x5A);
    assert!(web.cartridge_battery_save_dirty());

    let save = web
        .export_cartridge_save_ram_bytes()
        .expect("save RAM bytes should export");
    assert_eq!(save[0], 0x5A);

    web.mark_cartridge_persistence_clean();
    assert!(!web.cartridge_battery_save_dirty());

    let mut reloaded = WebEmulator::new(&rom, None).expect("web emulator should initialize");
    reloaded.import_cartridge_save_ram_bytes(&save);
    let roundtrip = reloaded
        .export_cartridge_save_ram_bytes()
        .expect("save RAM bytes should export");
    assert_eq!(roundtrip[0], 0x5A);
}
