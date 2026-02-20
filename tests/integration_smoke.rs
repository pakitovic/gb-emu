use gb_emu::audio::AudioMixer;
use gb_emu::cartridge::{Cartridge, CartridgeError, CartridgeMapper};
use gb_emu::gameboy::GameBoy;
use gb_emu::hardware::HardwareModel;
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

fn make_mbc2_battery_rom_64kb() -> Vec<u8> {
    let mut rom = vec![0; 64 * 1024];
    rom[0x0147] = 0x06; // MBC2+BATTERY
    rom[0x0148] = 0x01; // 64KB
    rom[0x0149] = 0x00; // RAM size code for MBC2
    rom
}

fn make_mbc3_timer_rom_32kb() -> Vec<u8> {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x0F; // MBC3+TIMER+BATTERY
    rom[0x0148] = 0x00; // 32KB
    rom[0x0149] = 0x00; // no external RAM
    rom
}

fn make_mbc5_rumble_ram_battery_rom_64kb() -> Vec<u8> {
    let mut rom = vec![0; 64 * 1024];
    rom[0x0147] = 0x1E; // MBC5+RUMBLE+RAM+BATTERY
    rom[0x0148] = 0x01; // 64KB
    rom[0x0149] = 0x04; // 128KB RAM to validate bank bit masking
    rom
}

#[derive(Clone, Copy)]
struct MapperSmokeCase {
    name: &'static str,
    cart_type: u8,
    rom_size_code: u8,
    ram_size_code: u8,
}

fn make_mapper_case_rom(case: MapperSmokeCase) -> Vec<u8> {
    let rom_len = match case.rom_size_code {
        0x00 => 32 * 1024,
        0x01 => 64 * 1024,
        other => panic!("unsupported ROM size code in test case: 0x{other:02X}"),
    };
    let mut rom = vec![0; rom_len];
    rom[0x0147] = case.cart_type;
    rom[0x0148] = case.rom_size_code;
    rom[0x0149] = case.ram_size_code;
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

fn clock_apu_div_falling_edges(gb: &mut GameBoy, edges: usize) {
    for _ in 0..edges {
        tick_n_tcycles(gb, 4096);
        gb.bus.write_byte(0xFF04, 0x00);
    }
}

fn tick_n_tcycles(gb: &mut GameBoy, mut tcycles: usize) {
    while tcycles > 0 {
        let chunk = tcycles.min(u8::MAX as usize) as u8;
        gb.bus.tick(chunk);
        tcycles -= chunk as usize;
    }
}

fn left_channel_rms(samples: &[f32], skip_frames: usize) -> f32 {
    let mut sum_sq = 0.0f64;
    let mut count = 0usize;
    for frame in samples.chunks_exact(2).skip(skip_frames) {
        let sample = frame[0] as f64;
        sum_sq += sample * sample;
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    (sum_sq / (count as f64)).sqrt() as f32
}

fn left_channel_delta_rms(samples: &[f32], skip_frames: usize) -> f32 {
    let mut sum_sq = 0.0f64;
    let mut count = 0usize;
    let mut previous: Option<f64> = None;
    for frame in samples.chunks_exact(2).skip(skip_frames) {
        let sample = frame[0] as f64;
        if let Some(prev) = previous {
            let delta = sample - prev;
            sum_sq += delta * delta;
            count += 1;
        }
        previous = Some(sample);
    }
    if count == 0 {
        return 0.0;
    }
    (sum_sq / (count as f64)).sqrt() as f32
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

    assert_eq!(samples.len(), 10_000);
    assert!(samples.iter().all(|sample| *sample == 0.0));
    assert_eq!(mixer.pending_samples(), 0);
    assert_eq!(pacer.drain_audio_tcycles(), 0);
}

#[test]
fn gameboy_exposes_apu_tcycle_stream_for_realtime_audio() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);
    gb.set_audio_tcycle_stream_enabled(true);

    gb.bus.write_byte(0xFF26, 0x00);
    gb.bus.write_byte(0xFF26, 0x80);
    gb.bus.write_byte(0xFF24, 0x77);
    gb.bus.write_byte(0xFF25, 0x11); // CH1 to both outputs
    gb.bus.write_byte(0xFF11, 0x80);
    gb.bus.write_byte(0xFF12, 0xF0);
    gb.bus.write_byte(0xFF13, 0xFC);
    gb.bus.write_byte(0xFF14, 0x87); // trigger CH1

    gb.bus.tick(255);
    gb.bus.tick(255);
    gb.bus.tick(2);
    let samples = gb.drain_audio_tcycle_samples();
    assert_eq!(samples.len(), 1_024);
    assert!(samples.iter().any(|sample| *sample != 0.0));
    assert!(gb.drain_audio_tcycle_samples().is_empty());
}

#[test]
fn gameboy_audio_tcycle_stream_is_disabled_by_default() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);

    gb.bus.write_byte(0xFF26, 0x00);
    gb.bus.write_byte(0xFF26, 0x80);
    gb.bus.write_byte(0xFF24, 0x77);
    gb.bus.write_byte(0xFF25, 0x11);
    gb.bus.write_byte(0xFF11, 0x80);
    gb.bus.write_byte(0xFF12, 0xF0);
    gb.bus.write_byte(0xFF13, 0xFC);
    gb.bus.write_byte(0xFF14, 0x87);

    gb.bus.tick(255);
    gb.bus.tick(255);
    gb.bus.tick(2);

    assert!(gb.drain_audio_tcycle_samples().is_empty());
}

#[test]
fn apu_sweep_negate_clear_disables_channel_via_gameboy_bus() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);

    gb.bus.write_byte(0xFF26, 0x00);
    gb.bus.write_byte(0xFF26, 0x80);
    gb.bus.write_byte(0xFF10, 0x19); // period=1, negate, shift=1
    gb.bus.write_byte(0xFF11, 0x80);
    gb.bus.write_byte(0xFF12, 0xF0);
    gb.bus.write_byte(0xFF13, 0xE8);
    gb.bus.write_byte(0xFF14, 0x83); // trigger
    assert_ne!(gb.bus.read_byte(0xFF26) & 0x01, 0x00);

    clock_apu_div_falling_edges(&mut gb, 3); // execute sweep step at sequencer step 2
    gb.bus.write_byte(0xFF10, 0x11); // clear negate after subtraction

    assert_eq!(gb.bus.read_byte(0xFF26) & 0x01, 0x00);
}

#[test]
fn apu_trigger_length_zero_expires_after_63_length_clocks_via_gameboy_bus() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);

    gb.bus.write_byte(0xFF26, 0x00);
    gb.bus.write_byte(0xFF26, 0x80);
    clock_apu_div_falling_edges(&mut gb, 1); // move to sequencer step 1 (non-length step)

    gb.bus.write_byte(0xFF17, 0xF0); // DAC on
    gb.bus.write_byte(0xFF19, 0xC0); // trigger + length enable with zero counter
    assert_ne!(gb.bus.read_byte(0xFF26) & 0x02, 0x00);

    clock_apu_div_falling_edges(&mut gb, 126); // 63 length clocks
    assert_eq!(gb.bus.read_byte(0xFF26) & 0x02, 0x00);
}

#[test]
fn apu_wave_retrigger_keeps_previous_buffer_first_sample_via_gameboy_bus() {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);
    gb.set_audio_tcycle_stream_enabled(true);

    gb.bus.write_byte(0xFF26, 0x00);
    gb.bus.write_byte(0xFF26, 0x80);
    gb.bus.write_byte(0xFF24, 0x77);
    gb.bus.write_byte(0xFF25, 0x44); // CH3 to both sides
    gb.bus.write_byte(0xFF30, 0x12); // first high nibble negative
    gb.bus.write_byte(0xFF31, 0xE4); // second high nibble positive
    gb.bus.write_byte(0xFF1A, 0x80);
    gb.bus.write_byte(0xFF1C, 0x20);
    gb.bus.write_byte(0xFF1D, 0xFF); // period=2
    gb.bus.write_byte(0xFF1E, 0x87); // trigger

    gb.bus.tick(4); // advance until sample buffer loaded from byte 1
    let _ = gb.drain_audio_tcycle_samples();

    gb.bus.write_byte(0xFF1E, 0x87); // retrigger while channel active
    gb.bus.tick(1); // first sample uses preserved buffer high nibble
    let samples = gb.drain_audio_tcycle_samples();
    assert!(!samples.is_empty());
    assert_eq!(samples.len() % 2, 0);
    assert!(samples[0].abs() > 0.000_1);
}

#[test]
fn apu_noise_shift14_has_lower_tail_energy_than_shift0_via_gameboy_bus() {
    fn run_noise(polynomial: u8) -> Vec<f32> {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
        let mut gb = GameBoy::new(cartridge);
        gb.set_audio_tcycle_stream_enabled(true);

        gb.bus.write_byte(0xFF26, 0x00);
        gb.bus.write_byte(0xFF26, 0x80);
        gb.bus.write_byte(0xFF24, 0x77);
        gb.bus.write_byte(0xFF25, 0x88); // CH4 to both sides
        gb.bus.write_byte(0xFF21, 0xF0); // DAC on
        gb.bus.write_byte(0xFF22, polynomial);
        gb.bus.write_byte(0xFF23, 0x80); // trigger
        tick_n_tcycles(&mut gb, 80_000);
        gb.drain_audio_tcycle_samples()
    }

    let shift0 = run_noise(0x00);
    let shift14 = run_noise(0xE0);
    assert!(!shift0.is_empty());
    assert!(!shift14.is_empty());

    let rms_shift0 = left_channel_rms(&shift0, 10_000);
    let rms_shift14 = left_channel_rms(&shift14, 10_000);
    let delta_shift0 = left_channel_delta_rms(&shift0, 10_000);
    let delta_shift14 = left_channel_delta_rms(&shift14, 10_000);
    assert!(rms_shift0 > 0.01);
    assert!(rms_shift14 > 0.01);
    assert!(
        delta_shift0 > delta_shift14 * 1.5,
        "expected shift0 tail to have higher variation than shift14 (shift0_delta={delta_shift0}, shift14_delta={delta_shift14}, shift0_rms={rms_shift0}, shift14_rms={rms_shift14})"
    );
}

#[test]
fn apu_model_specific_analog_profiles_produce_distinct_levels_via_gameboy_bus() {
    fn run_square2_rms(model: HardwareModel) -> f32 {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
        let mut gb = GameBoy::new_with_model(cartridge, model);
        gb.set_audio_tcycle_stream_enabled(true);

        gb.bus.write_byte(0xFF26, 0x00);
        gb.bus.write_byte(0xFF26, 0x80);
        gb.bus.write_byte(0xFF24, 0x77);
        gb.bus.write_byte(0xFF25, 0x22); // CH2 to both sides
        gb.bus.write_byte(0xFF16, 0x80);
        gb.bus.write_byte(0xFF17, 0xF0); // DAC on, volume 15
        gb.bus.write_byte(0xFF18, 0xFC); // high frequency
        gb.bus.write_byte(0xFF19, 0x87); // trigger

        tick_n_tcycles(&mut gb, 24_000);
        let samples = gb.drain_audio_tcycle_samples();
        assert!(!samples.is_empty());
        left_channel_rms(&samples, 4_000)
    }

    let dmg_rms = run_square2_rms(HardwareModel::Dmg);
    let mgb_rms = run_square2_rms(HardwareModel::Mgb);
    assert!(dmg_rms > 0.0);
    assert!(mgb_rms > 0.0);
    assert!(
        (dmg_rms - mgb_rms).abs() > 0.005,
        "expected model-specific analog profiles to produce distinct RMS (dmg={dmg_rms}, mgb={mgb_rms})"
    );
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

#[test]
fn mbc2_ram_nibbles_are_visible_via_gameboy_bus() {
    let cartridge =
        Cartridge::from_bytes(make_mbc2_battery_rom_64kb()).expect("cartridge should load");
    let mut gb = GameBoy::new(cartridge);

    gb.bus.write_byte(0xA000, 0xAB);
    assert_eq!(gb.bus.read_byte(0xA000), 0xFF); // disabled

    gb.bus.write_byte(0x0000, 0x0A); // RAM enable (A8=0)
    gb.bus.write_byte(0xA000, 0xAB);
    assert_eq!(gb.bus.read_byte(0xA000), 0xFB);
}

#[test]
fn mbc3_rtc_registers_are_accessible_via_gameboy_bus() {
    let cartridge =
        Cartridge::from_bytes(make_mbc3_timer_rom_32kb()).expect("cartridge should load");
    let mut gb = GameBoy::new(cartridge);

    gb.bus.write_byte(0x0000, 0x0A); // RAM/RTC enable
    gb.bus.write_byte(0x4000, 0x0C); // day high
    gb.bus.write_byte(0xA000, 0x40); // halt clock
    gb.bus.write_byte(0x4000, 0x08); // seconds
    gb.bus.write_byte(0xA000, 42);

    gb.bus.write_byte(0x6000, 0x00); // latch step 1
    gb.bus.write_byte(0x6000, 0x01); // latch step 2
    assert_eq!(gb.bus.read_byte(0xA000), 42);
}

#[test]
fn mbc5_rumble_uses_low3_ram_bank_bits_via_gameboy_bus() {
    let cartridge = Cartridge::from_bytes(make_mbc5_rumble_ram_battery_rom_64kb())
        .expect("cartridge should load");
    let mut gb = GameBoy::new(cartridge);

    assert!(gb.cartridge_has_rumble());
    assert!(!gb.rumble_active());

    gb.bus.write_byte(0x0000, 0x0A); // RAM enable
    gb.bus.write_byte(0x4000, 0x00); // bank 0, rumble off
    gb.bus.write_byte(0xA000, 0x11);

    gb.bus.write_byte(0x4000, 0x08); // bank 0, rumble on
    assert!(gb.rumble_active());
    gb.bus.write_byte(0xA000, 0x22);

    gb.bus.write_byte(0x4000, 0x00); // bank 0, rumble off
    assert!(!gb.rumble_active());
    assert_eq!(gb.bus.read_byte(0xA000), 0x22);

    gb.bus.write_byte(0x4000, 0x01); // bank 1, rumble off
    gb.bus.write_byte(0xA000, 0x33);

    gb.bus.write_byte(0x4000, 0x09); // bank 1, rumble on
    assert!(gb.rumble_active());
    assert_eq!(gb.bus.read_byte(0xA000), 0x33);

    gb.bus.write_byte(0x4000, 0x00); // bank 0
    assert_eq!(gb.bus.read_byte(0xA000), 0x22);
}

#[test]
fn supported_mapper_type_matrix_constructs_gameboy_and_steps() {
    let cases = [
        MapperSmokeCase {
            name: "ROM_ONLY",
            cart_type: 0x00,
            rom_size_code: 0x00,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "ROM_RAM",
            cart_type: 0x08,
            rom_size_code: 0x00,
            ram_size_code: 0x02,
        },
        MapperSmokeCase {
            name: "ROM_RAM_BATTERY",
            cart_type: 0x09,
            rom_size_code: 0x00,
            ram_size_code: 0x03,
        },
        MapperSmokeCase {
            name: "MBC1",
            cart_type: 0x01,
            rom_size_code: 0x01,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "MBC1_RAM",
            cart_type: 0x02,
            rom_size_code: 0x01,
            ram_size_code: 0x02,
        },
        MapperSmokeCase {
            name: "MBC1_RAM_BATTERY",
            cart_type: 0x03,
            rom_size_code: 0x01,
            ram_size_code: 0x03,
        },
        MapperSmokeCase {
            name: "MBC2",
            cart_type: 0x05,
            rom_size_code: 0x01,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "MBC2_BATTERY",
            cart_type: 0x06,
            rom_size_code: 0x01,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "MBC3",
            cart_type: 0x11,
            rom_size_code: 0x01,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "MBC3_RAM",
            cart_type: 0x12,
            rom_size_code: 0x01,
            ram_size_code: 0x02,
        },
        MapperSmokeCase {
            name: "MBC3_RAM_BATTERY",
            cart_type: 0x13,
            rom_size_code: 0x01,
            ram_size_code: 0x03,
        },
        MapperSmokeCase {
            name: "MBC3_TIMER_BATTERY",
            cart_type: 0x0F,
            rom_size_code: 0x01,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "MBC3_TIMER_RAM_BATTERY",
            cart_type: 0x10,
            rom_size_code: 0x01,
            ram_size_code: 0x03,
        },
        MapperSmokeCase {
            name: "MBC5",
            cart_type: 0x19,
            rom_size_code: 0x01,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "MBC5_RAM",
            cart_type: 0x1A,
            rom_size_code: 0x01,
            ram_size_code: 0x02,
        },
        MapperSmokeCase {
            name: "MBC5_RAM_BATTERY",
            cart_type: 0x1B,
            rom_size_code: 0x01,
            ram_size_code: 0x04,
        },
        MapperSmokeCase {
            name: "MBC5_RUMBLE",
            cart_type: 0x1C,
            rom_size_code: 0x01,
            ram_size_code: 0x00,
        },
        MapperSmokeCase {
            name: "MBC5_RUMBLE_RAM",
            cart_type: 0x1D,
            rom_size_code: 0x01,
            ram_size_code: 0x02,
        },
        MapperSmokeCase {
            name: "MBC5_RUMBLE_RAM_BATTERY",
            cart_type: 0x1E,
            rom_size_code: 0x01,
            ram_size_code: 0x03,
        },
    ];

    for case in cases {
        let cartridge = Cartridge::from_bytes(make_mapper_case_rom(case))
            .unwrap_or_else(|err| panic!("{} should load: {err}", case.name));
        let mut gb = GameBoy::new(cartridge);
        let cycles = gb.step();
        assert_eq!(cycles, 4, "{} should execute NOP from ROM", case.name);
    }
}

#[test]
fn gameboy_exposes_cartridge_metadata_for_debug() {
    let cartridge = Cartridge::from_bytes(make_mbc5_rumble_ram_battery_rom_64kb())
        .expect("cartridge should load");
    let gb = GameBoy::new(cartridge);
    let metadata = gb.cartridge_metadata();

    assert_eq!(metadata.mapper, CartridgeMapper::Mbc5);
    assert_eq!(metadata.cart_type_code, 0x1E);
    assert_eq!(metadata.rom_size_code, 0x01);
    assert_eq!(metadata.ram_size_code, 0x04);
    assert_eq!(metadata.rom_bank_count, 4);
    assert_eq!(metadata.ram_bank_count, 16);
    assert_eq!(metadata.declared_ram_size_bytes, 128 * 1024);
    assert_eq!(metadata.effective_ram_size_bytes, 128 * 1024);
    assert!(metadata.has_battery);
    assert!(!metadata.has_timer);
    assert!(metadata.has_rumble);
    assert!(metadata.has_battery_save);
    assert!(!metadata.rumble_active);
    assert!(!metadata.header_warnings.is_empty());
}

fn wait_for_ly_mode(gb: &mut GameBoy, target_ly: u8, target_mode: u8) {
    for _ in 0..(154 * 456 * 2) {
        let ly = gb.bus.read_byte(0xFF44);
        let mode = gb.bus.read_byte(0xFF41) & 0x03;
        if ly == target_ly && mode == target_mode {
            return;
        }
        gb.bus.tick(1);
    }
    panic!("LY={target_ly} mode={target_mode} not observed");
}

fn ticks_until_stat_irq(gb: &mut GameBoy) -> u16 {
    let mut ticks = 0u16;
    for _ in 0..512 {
        if (gb.bus.interrupt_flags() & (1 << 1)) != 0 {
            return ticks;
        }
        gb.bus.tick(1);
        ticks = ticks.saturating_add(1);
    }
    panic!("STAT IRQ not observed within expected window");
}

fn setup_gameboy_line2_mode3_hidden_obj(obj_enabled: bool) -> GameBoy {
    let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
    let mut gb = GameBoy::new(cartridge);
    gb.bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    gb.bus.write_byte(0xFF42, 0x00); // SCY
    gb.bus.write_byte(0xFF43, 0x00); // SCX

    // Hidden X=0 sprite on LY=2 to introduce mode3 OBJ contention.
    gb.bus.write_byte(0xFE00, 18); // Y => top at LY=2
    gb.bus.write_byte(0xFE01, 0); // X hidden/off-screen
    gb.bus.write_byte(0xFE02, 0x00); // tile
    gb.bus.write_byte(0xFE03, 0x00); // attrs

    let mut lcdc = 0x91; // LCD on + BG on
    if obj_enabled {
        lcdc |= 0x02;
    }
    gb.bus.write_byte(0xFF40, lcdc);
    wait_for_ly_mode(&mut gb, 2, 3);
    gb
}

#[test]
fn mode3_obj_contention_delays_stat_mode0_irq_via_gameboy_bus() {
    let mut gb_no_obj = setup_gameboy_line2_mode3_hidden_obj(false);
    let mut gb_with_obj = setup_gameboy_line2_mode3_hidden_obj(true);

    gb_no_obj.bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    gb_with_obj.bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    gb_no_obj
        .bus
        .set_interrupt_flags(gb_no_obj.bus.interrupt_flags() & !(1 << 1));
    gb_with_obj
        .bus
        .set_interrupt_flags(gb_with_obj.bus.interrupt_flags() & !(1 << 1));

    let no_obj_ticks = ticks_until_stat_irq(&mut gb_no_obj);
    let with_obj_ticks = ticks_until_stat_irq(&mut gb_with_obj);
    assert!(
        with_obj_ticks > no_obj_ticks,
        "expected OBJ contention to delay mode0 STAT IRQ (no_obj={no_obj_ticks}, with_obj={with_obj_ticks})"
    );
}
