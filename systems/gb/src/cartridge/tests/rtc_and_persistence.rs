use super::super::*;
use super::support::*;

#[test]
fn mbc3_rtc_halt_stops_elapsed_time_with_test_clock() {
    let clock = TestClock::new(100);
    let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
    let mut cart = Cartridge::from_bytes_with_clock(rom, Box::new(clock.clone()))
        .expect("valid MBC3 ROM should load");

    cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
    cart.write_rom_control(0x4000, 0x08); // seconds register
    cart.write_ram_byte(0xA000, 10);

    cart.write_rom_control(0x4000, 0x0C); // day high
    cart.write_ram_byte(0xA000, 0x40); // halt

    clock.set_now_epoch_secs(160);
    cart.write_rom_control(0x4000, 0x08);
    assert_eq!(cart.read_ram_byte(0xA000), 10);

    cart.write_rom_control(0x4000, 0x0C);
    cart.write_ram_byte(0xA000, 0x00); // resume

    clock.set_now_epoch_secs(165);
    cart.write_rom_control(0x4000, 0x08);
    assert_eq!(cart.read_ram_byte(0xA000), 15);
}

#[test]
fn mbc3_rtc_latch_snapshot_is_stable_until_next_latch_with_test_clock() {
    let clock = TestClock::new(10);
    let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
    let mut cart = Cartridge::from_bytes_with_clock(rom, Box::new(clock.clone()))
        .expect("valid MBC3 ROM should load");

    cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
    cart.write_rom_control(0x4000, 0x08); // seconds register
    cart.write_ram_byte(0xA000, 0);

    clock.set_now_epoch_secs(15);
    cart.write_rom_control(0x6000, 0x00);
    cart.write_rom_control(0x6000, 0x01);

    clock.set_now_epoch_secs(20);
    cart.write_rom_control(0x4000, 0x08);
    assert_eq!(cart.read_ram_byte(0xA000), 5);

    cart.write_rom_control(0x6000, 0x00);
    cart.write_rom_control(0x6000, 0x01);
    assert_eq!(cart.read_ram_byte(0xA000), 10);
}

#[test]
fn mbc3_rtc_day_counter_sets_carry_after_overflow_with_test_clock() {
    let clock = TestClock::new(0);
    let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
    let mut cart = Cartridge::from_bytes_with_clock(rom, Box::new(clock.clone()))
        .expect("valid MBC3 ROM should load");

    cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
    cart.write_rom_control(0x4000, 0x0B); // day low
    cart.write_ram_byte(0xA000, 0xFF);
    cart.write_rom_control(0x4000, 0x0C); // day high
    cart.write_ram_byte(0xA000, 0x01); // day bit 8 = 1 => 511 days

    clock.set_now_epoch_secs(86_400);
    cart.write_rom_control(0x4000, 0x0B);
    assert_eq!(cart.read_ram_byte(0xA000), 0x00);
    cart.write_rom_control(0x4000, 0x0C);
    let day_high = cart.read_ram_byte(0xA000);
    assert_eq!(day_high & 0x01, 0x00);
    assert_eq!(day_high & 0x80, 0x80);
}

#[test]
fn save_ram_persistence_bytes_roundtrip_restores_battery_ram() {
    let rom = make_rom(64 * 1024, MBC1_RAM_BATTERY, 0x01, 0x02);
    let mut first = Cartridge::from_bytes(rom.clone()).expect("cartridge should load");
    first.write_rom_control(0x0000, 0x0A);
    first.write_ram_byte(0xA000, 0x5A);
    first.write_ram_byte(0xA123, 0xC3);

    let persisted = first
        .export_save_ram_bytes()
        .expect("battery-backed RAM should export persistence bytes");

    let mut second = Cartridge::from_bytes(rom).expect("cartridge should load");
    second.import_save_ram_bytes(&persisted);
    second.write_rom_control(0x0000, 0x0A);
    assert_eq!(second.read_ram_byte(0xA000), 0x5A);
    assert_eq!(second.read_ram_byte(0xA123), 0xC3);
}

#[test]
fn rtc_persistence_bytes_roundtrip_restores_mbc3_rtc_state() {
    let clock = TestClock::new(100);
    let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
    let mut first = Cartridge::from_bytes_with_clock(rom.clone(), Box::new(clock.clone()))
        .expect("cartridge should load");
    first.write_rom_control(0x0000, 0x0A); // RAM/RTC enable
    first.write_rom_control(0x4000, 0x0C); // day high
    first.write_ram_byte(0xA000, 0x40); // halt
    first.write_rom_control(0x4000, 0x08); // seconds
    first.write_ram_byte(0xA000, 33);

    let rtc_bytes = first
        .export_rtc_persistence_bytes()
        .expect("MBC3 timer cartridge should export RTC persistence bytes");

    let mut second = Cartridge::from_bytes_with_clock(rom, Box::new(TestClock::new(100)))
        .expect("cartridge should load");
    assert!(second.import_rtc_persistence_bytes(&rtc_bytes));

    second.write_rom_control(0x0000, 0x0A);
    second.write_rom_control(0x4000, 0x0C);
    assert_eq!(second.read_ram_byte(0xA000) & 0x40, 0x40);
    second.write_rom_control(0x4000, 0x08);
    assert_eq!(second.read_ram_byte(0xA000), 33);
}
