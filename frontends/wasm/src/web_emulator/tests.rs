use super::audio::parse_audio_resampler_quality;
use super::*;
use gb_emu::gameboy::{SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::sgb::{CMD_MASK_EN, CMD_MLT_REQ};
use gb_runtime::session::RuntimeSession;

fn make_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    rom
}

fn make_rom_with_title_32kb(title: &str) -> Vec<u8> {
    let mut rom = make_rom_32kb();
    let title_bytes = title.as_bytes();
    let copy_len = title_bytes.len().min(16);
    rom[0x0134..0x0134 + copy_len].copy_from_slice(&title_bytes[..copy_len]);
    rom
}

fn make_sgb_enhanced_rom_32kb() -> Vec<u8> {
    let mut rom = make_rom_32kb();
    rom[0x0146] = 0x03; // SGB support flag
    rom
}

fn make_mbc1_battery_ram_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x03; // MBC1 + RAM + BATTERY
    rom[0x0148] = 0x00; // 32KB
    rom[0x0149] = 0x02; // 8KB RAM
    rom
}

fn setup_ch1_routed_output(web: &mut WebEmulator) {
    let gb = web.session.gameboy_mut();
    gb.bus.write_byte(0xFF26, 0x00);
    gb.bus.write_byte(0xFF26, 0x80);
    gb.bus.write_byte(0xFF24, 0x77);
    gb.bus.write_byte(0xFF25, 0x11);
    gb.bus.write_byte(0xFF11, 0x80);
    gb.bus.write_byte(0xFF12, 0xF0);
    gb.bus.write_byte(0xFF13, 0xFC);
    gb.bus.write_byte(0xFF14, 0x87);
}

fn feed_sgb_packet_via_p1(web: &mut WebEmulator, packet: &[u8; 16]) {
    let gb = web.session.gameboy_mut();
    gb.bus.write_byte(0xFF00, 0x00);
    for byte in packet {
        for bit in 0..8 {
            let bit_value = (byte >> bit) & 0x01;
            let p1_write = if bit_value == 0 { 0x10 } else { 0x20 };
            gb.bus.write_byte(0xFF00, p1_write);
        }
    }
    gb.bus.write_byte(0xFF00, 0x10);
}

fn make_single_packet_command(command_id: u8, payload: &[u8]) -> [u8; 16] {
    let mut packet = [0u8; 16];
    packet[0] = (command_id << 3) | 0x01;
    for (index, value) in payload.iter().copied().enumerate() {
        if index + 1 >= packet.len() {
            break;
        }
        packet[index + 1] = value;
    }
    packet
}

fn select_sgb_player(web: &mut WebEmulator, target_player: u8) {
    let gb = web.session.gameboy_mut();
    gb.bus.write_byte(0xFF00, 0x30);
    while gb.current_joypad_player_index() != target_player {
        gb.bus.write_byte(0xFF00, 0x10);
        gb.bus.write_byte(0xFF00, 0x30);
    }
}

fn write_sgb_transfer_block(web: &mut WebEmulator, transfer: &[u8; 0x1000]) {
    let gb = web.session.gameboy_mut();
    for (offset, byte) in transfer.iter().copied().enumerate() {
        gb.bus.write_byte(0x8000 + offset as u16, byte);
    }
}

#[test]
fn constructor_rejects_invalid_model_string() {
    let rom = make_rom_32kb();
    let err = match WebEmulator::new_internal(&rom, Some("cgb"), None) {
        Ok(_) => panic!("invalid model should be rejected"),
        Err(err) => err,
    };
    assert!(err.contains("Unsupported model"));
}

#[test]
fn constructor_rejects_invalid_rom_bytes() {
    let err = match WebEmulator::new_internal(&[0x00, 0x01, 0x02], None, None) {
        Ok(_) => panic!("invalid rom bytes should be rejected"),
        Err(err) => err,
    };
    assert!(!err.is_empty());
}

#[test]
fn constructor_accepts_optional_boot_rom_payload() {
    let rom = make_rom_32kb();
    let boot_rom = vec![0x31; 0x200];

    let web = WebEmulator::new_internal(&rom, Some("dmg"), Some(&boot_rom))
        .expect("valid optional boot ROM should initialize");
    assert_eq!(web.frame_counter(), 0);
}

#[test]
fn video_palette_defaults_to_model_profile_and_produces_rgba_frame() {
    let rom = make_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("mgb"), None).expect("web emulator should initialize");
    assert_eq!(web.video_palette(), "mgb");
    assert_eq!(web.screen_width(), SCREEN_WIDTH as u32);
    assert_eq!(web.screen_height(), SCREEN_HEIGHT as u32);
    assert_eq!(web.rgba_frame().len(), SCREEN_WIDTH * SCREEN_HEIGHT * 4);
}

#[test]
fn auto_palette_uses_sgb_boot_palette_immediately_on_sgb_models() {
    let rom = make_sgb_enhanced_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("sgb"), None).expect("web emulator should initialize");
    assert_eq!(web.video_palette(), "sgb");

    let mask_en = make_single_packet_command(CMD_MASK_EN, &[0x00]);
    feed_sgb_packet_via_p1(&mut web, &mask_en);

    assert_eq!(web.video_palette(), "sgb");
}

#[test]
fn sgb_model_uses_title_specific_boot_palette_for_non_enhanced_kirby() {
    let rom = make_rom_with_title_32kb("KIRBY DREAM LAND");
    let mut web =
        WebEmulator::new_internal(&rom, Some("sgb"), None).expect("web emulator should initialize");

    assert_eq!(web.video_palette(), "sgb");
    let rgba = web.rgba_frame();
    assert_eq!(&rgba[0..4], &[0xFF, 0xC6, 0xFF, 0xFF]);
}

#[test]
fn auto_palette_keeps_dmg_base_on_non_sgb_model_even_if_sgb_traffic_is_detected() {
    let rom = make_sgb_enhanced_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("dmg"), None).expect("web emulator should initialize");
    assert_eq!(web.video_palette(), "dmg");

    let mask_en = make_single_packet_command(CMD_MASK_EN, &[0x00]);
    feed_sgb_packet_via_p1(&mut web, &mask_en);

    assert_eq!(web.video_palette(), "dmg");
}

#[test]
fn sgb_palette_uses_composed_border_dimensions_when_border_is_available() {
    let rom = make_sgb_enhanced_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("sgb"), None).expect("web emulator should initialize");
    web.set_video_palette("sgb")
        .expect("sgb palette should be accepted");

    let mut chr_transfer = [0u8; 0x1000];
    for row in 0..8 {
        chr_transfer[row * 2] = 0xFF;
    }
    write_sgb_transfer_block(&mut web, &chr_transfer);
    let chr_trn = make_single_packet_command(0x13, &[0x00]);
    feed_sgb_packet_via_p1(&mut web, &chr_trn);

    let mut pct_transfer = [0u8; 0x1000];
    for entry in 0..(32 * 28) {
        let offset = entry * 2;
        pct_transfer[offset..offset + 2].copy_from_slice(&0x1000u16.to_le_bytes());
    }
    let palette_base = 0x0800;
    pct_transfer[palette_base + 2..palette_base + 4].copy_from_slice(&0x001Fu16.to_le_bytes());
    write_sgb_transfer_block(&mut web, &pct_transfer);
    let pct_trn = make_single_packet_command(0x14, &[]);
    feed_sgb_packet_via_p1(&mut web, &pct_trn);

    let (width, height) = RuntimeSession::sgb_border_frame_size();
    assert_eq!(web.screen_width(), width as u32);
    assert_eq!(web.screen_height(), height as u32);
    assert_eq!(web.rgba_frame().len(), width * height * 4);
}

#[test]
fn set_video_palette_accepts_auto_and_named_profiles() {
    let rom = make_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("dmg"), None).expect("web emulator should initialize");

    assert_eq!(web.video_palette(), "dmg");
    web.set_video_palette("mgb")
        .expect("mgb palette should be accepted");
    assert_eq!(web.video_palette(), "mgb");

    web.set_video_palette("auto")
        .expect("auto should reset model default");
    assert_eq!(web.video_palette(), "dmg");
}

#[test]
fn set_video_palette_accepts_manual_cgb_presets_without_promoting_to_sgb_frame() {
    let rom = make_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("dmg"), None).expect("web emulator should initialize");

    web.set_video_palette("cgb-blue")
        .expect("manual CGB preset should be accepted");

    assert_eq!(web.video_palette(), "cgb-blue");
    assert_eq!(web.screen_width(), SCREEN_WIDTH as u32);
    assert_eq!(web.screen_height(), SCREEN_HEIGHT as u32);
    assert_eq!(web.rgba_frame().len(), SCREEN_WIDTH * SCREEN_HEIGHT * 4);
}

#[test]
fn palette_overrides_ini_can_override_cgb_auto_colors_by_header_crc32() {
    let rom = make_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("dmg"), None).expect("web emulator should initialize");
    let header_crc32 = web.session.gameboy().rom_header_crc32();

    web.set_video_palette("cgb")
        .expect("cgb palette should be accepted");
    web.set_palette_overrides_ini(&format!(
        "[gb.override.{header_crc32:08X}]\npal[0]=0x112233\n"
    ))
    .expect("override INI should be accepted");

    assert_eq!(web.palette_override_count(), 1);
    let rgba = web.rgba_frame();
    assert_eq!(&rgba[0..4], &[0x11, 0x22, 0x33, 0xFF]);

    web.clear_palette_overrides();
    assert_eq!(web.palette_override_count(), 0);
}

#[test]
fn palette_overrides_ini_can_override_sgb_boot_palette_before_cart_commands() {
    let rom = make_rom_with_title_32kb("UNKNOWN GAME");
    let mut web =
        WebEmulator::new_internal(&rom, Some("sgb"), None).expect("web emulator should initialize");
    let before = web.rgba_frame();
    let header_crc32 = web.session.gameboy().rom_header_crc32();

    web.set_palette_overrides_ini(&format!(
        "[gb.override.{header_crc32:08X}]\npal[0]=0x112233\n"
    ))
    .expect("override INI should be accepted");

    let overridden = web.rgba_frame();
    assert_eq!(web.palette_override_count(), 1);
    assert_eq!(&overridden[0..4], &[0x10, 0x21, 0x31, 0xFF]);

    web.clear_palette_overrides();
    assert_eq!(web.palette_override_count(), 0);
    assert_eq!(web.rgba_frame(), before);
}

#[test]
fn set_video_palette_accepts_manual_sgb_built_in_presets_without_promoting_to_sgb_frame() {
    let rom = make_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("dmg"), None).expect("web emulator should initialize");

    web.set_video_palette("sgb-2c")
        .expect("manual SGB preset should be accepted");

    assert_eq!(web.video_palette(), "sgb-2c");
    assert_eq!(web.screen_width(), SCREEN_WIDTH as u32);
    assert_eq!(web.screen_height(), SCREEN_HEIGHT as u32);

    let rgba = web.rgba_frame();
    assert_eq!(&rgba[0..4], &[0xFF, 0xC6, 0xFF, 0xFF]);
}

#[test]
fn parse_video_palette_selection_rejects_unknown_profile_names() {
    let err = super::video::parse_video_palette_selection("sepia")
        .expect_err("unsupported palette should fail");
    assert!(err.contains("Unsupported palette"));
}

#[test]
fn parse_palette_overrides_ini_entry_count_reports_valid_entries() {
    assert_eq!(
        crate::parse_palette_overrides_ini_entry_count("[gb.override.302017CC]\npal[0]=0x112233\n")
            .expect("override INI should parse"),
        1
    );
}

#[test]
fn constructor_rejects_short_boot_rom_payload() {
    let rom = make_rom_32kb();
    let short_boot_rom = vec![0x00; 0x80];

    let err = match WebEmulator::new_internal(&rom, Some("dmg"), Some(&short_boot_rom)) {
        Ok(_) => panic!("short boot ROM payload should be rejected"),
        Err(err) => err,
    };
    assert!(err.contains("at least 256 bytes"));
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
fn drain_audio_samples_queue_mode_does_not_pad_short_core_budget() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
    setup_ch1_routed_output(&mut web);
    web.run_frame().expect("a frame should be produced");

    let first = web.drain_audio_samples(1024);
    assert!(!first.is_empty());
    assert_eq!(first.len() % 2, 0);
    assert!(first.len() < 2 * 1024);
    assert!(first.iter().any(|sample| sample.abs() > 0.0));

    let second = web.drain_audio_samples(1024);
    assert!(second.is_empty());
}

#[test]
fn shared_runtime_queue_controller_is_exposed_for_web_refill_policy() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");

    assert_eq!(web.audio_queue_refill_block_samples(), 512);
    assert_eq!(web.audio_queue_max_refill_blocks(), 32);
    assert!(!web.audio_queue_clear_required());

    let target = web.observe_audio_queue_target(1.0, 0);
    assert!(target >= 2_048);
    assert!(!web.audio_queue_clear_required());
    web.commit_audio_queue_refill(1.0, 0);

    let _target = web.observe_audio_queue_target(2.0, 65_000);
    assert!(web.audio_queue_clear_required());
    web.commit_audio_queue_refill(2.0, 0);
    assert!(!web.audio_queue_clear_required());
}

#[test]
fn drain_audio_samples_realtime_can_emit_core_apu_signal() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");

    setup_ch1_routed_output(&mut web);

    web.run_frame().expect("a frame should be produced");
    let samples = web.drain_audio_samples_realtime(512);
    assert_eq!(samples.len(), 1_024);
    assert!(samples.iter().any(|sample| *sample != 0.0));
}

#[test]
fn set_audio_sample_rate_preserves_pending_core_apu_queue() {
    let rom = make_rom_32kb();
    let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");

    setup_ch1_routed_output(&mut web);

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
fn sgb_multiplayer_input_api_routes_player_buttons_and_reports_state() {
    let rom = make_rom_32kb();
    let mut web =
        WebEmulator::new_internal(&rom, Some("sgb"), None).expect("web emulator should initialize");
    let mlt_req = make_single_packet_command(CMD_MLT_REQ, &[0x01]); // 2 players
    feed_sgb_packet_via_p1(&mut web, &mlt_req);

    assert_eq!(web.joypad_player_count(), 2);
    assert!(web.set_player_button(1, 4, true).is_ok());

    select_sgb_player(&mut web, 1);
    assert_eq!(web.current_joypad_player_index(), 1);
    let gb = web.session.gameboy_mut();
    gb.bus.write_byte(0xFF00, 0x10);
    assert_eq!(gb.bus.read_byte(0xFF00) & 0x0F, 0x0E);
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

    {
        let gb = web.session.gameboy_mut();
        gb.bus.write_byte(0x0000, 0x0A); // RAM enable (MBC1)
        gb.bus.write_byte(0xA000, 0x5A);
    }
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
