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
