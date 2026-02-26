use super::super::*;
use super::support::*;

#[test]
fn accepts_rom_only_32kb() {
    let rom = make_rom(32 * 1024, ROM_ONLY, 0x00, 0x00);
    let cart = Cartridge::from_bytes(rom).expect("valid ROM should load");
    assert_eq!(cart.read_rom_byte(0x1234), 0x00);
}

#[test]
fn supports_extended_rom_size_codes() {
    assert_eq!(rom_size_bytes_from_code(0x00), Some(32 * 1024));
    assert_eq!(rom_size_bytes_from_code(0x06), Some(2 * 1024 * 1024));
    assert_eq!(rom_size_bytes_from_code(0x08), Some(8 * 1024 * 1024));
    assert_eq!(rom_size_bytes_from_code(0x52), Some(72 * ROM_BANK_BYTES));
    assert_eq!(rom_size_bytes_from_code(0x53), Some(80 * ROM_BANK_BYTES));
    assert_eq!(rom_size_bytes_from_code(0x54), Some(96 * ROM_BANK_BYTES));
}

#[test]
fn mbc1_switches_rom_bank_low_bits() {
    let mut rom = make_rom(64 * 1024, MBC1, 0x01, 0x00);
    rom[0x4000] = 0x11; // bank 1 first byte
    rom[0x4000 + 0x4000] = 0x22; // bank 2 first byte

    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1 ROM should load");
    assert_eq!(cart.read_rom_byte(0x4000), 0x11);

    cart.write_rom_control(0x2000, 0x02);
    assert_eq!(cart.read_rom_byte(0x4000), 0x22);
}

#[test]
fn mbc1_modes_and_banks_external_ram() {
    let rom = make_rom(256 * 1024, MBC1_RAM_BATTERY, 0x03, 0x03);
    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1+RAM+BATTERY ROM should load");

    // Disabled RAM reads as open bus and ignores writes.
    cart.write_ram_byte(0xA000, 0x66);
    assert_eq!(cart.read_ram_byte(0xA000), 0xFF);

    cart.write_rom_control(0x0000, 0x0A);
    cart.write_ram_byte(0xA000, 0x11);
    assert_eq!(cart.read_ram_byte(0xA000), 0x11);

    // Enter RAM banking mode and switch RAM bank.
    cart.write_rom_control(0x6000, 0x01);
    cart.write_rom_control(0x4000, 0x01);
    cart.write_ram_byte(0xA000, 0x22);
    assert_eq!(cart.read_ram_byte(0xA000), 0x22);

    cart.write_rom_control(0x4000, 0x00);
    assert_eq!(cart.read_ram_byte(0xA000), 0x11);

    cart.write_rom_control(0x4000, 0x01);
    assert_eq!(cart.read_ram_byte(0xA000), 0x22);

    cart.write_rom_control(0x0000, 0x00);
    assert_eq!(cart.read_ram_byte(0xA000), 0xFF);
}

#[test]
fn mbc1_mode_switch_changes_fixed_rom_region() {
    let mut rom = make_rom(2 * 1024 * 1024, MBC1, 0x06, 0x00);
    fill_each_rom_bank_first_byte(&mut rom);
    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1 ROM should load");

    cart.write_rom_control(0x2000, 0x01);
    cart.write_rom_control(0x4000, 0x02);
    assert_eq!(cart.read_rom_byte(0x0000), 0x00);
    assert_eq!(cart.read_rom_byte(0x4000), 0x41);

    cart.write_rom_control(0x6000, 0x01);
    assert_eq!(cart.read_rom_byte(0x0000), 0x40);
    assert_eq!(cart.read_rom_byte(0x4000), 0x41);
}

#[test]
fn mbc1_ram_battery_switches_rom_bank() {
    let mut rom = make_rom(64 * 1024, MBC1_RAM_BATTERY, 0x01, 0x02);
    rom[0x4000] = 0x11;
    rom[0x4000 + 0x4000] = 0x22;

    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC1+RAM+BATTERY ROM should load");
    cart.write_rom_control(0x2000, 0x02);
    assert_eq!(cart.read_rom_byte(0x4000), 0x22);
}

#[test]
fn mbc5_switches_rom_bank_and_allows_bank_zero() {
    let mut rom = make_rom(64 * 1024, MBC5_RAM_BATTERY, 0x01, 0x02);
    rom[0x0000] = 0x10; // bank 0 first byte
    rom[0x4000] = 0x11; // bank 1 first byte
    rom[0x8000] = 0x22; // bank 2 first byte

    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 ROM should load");
    assert_eq!(cart.read_rom_byte(0x4000), 0x11);

    cart.write_rom_control(0x2000, 0x02);
    assert_eq!(cart.read_rom_byte(0x4000), 0x22);

    cart.write_rom_control(0x2000, 0x00);
    assert_eq!(cart.read_rom_byte(0x4000), 0x10);
}

#[test]
fn mbc5_supports_rom_high_bit_and_ram_banks() {
    let mut rom = make_rom(8 * 1024 * 1024, MBC5_RAM_BATTERY, 0x08, 0x03);
    fill_each_rom_bank_first_byte(&mut rom);

    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 ROM should load");
    assert_eq!(cart.read_rom_byte(0x4000), 0x01);

    cart.write_rom_control(0x2000, 0x01);
    cart.write_rom_control(0x3000, 0x01);
    assert_eq!(cart.read_rom_byte(0x4000), 0x01); // bank 257 wraps byte value to 0x01

    cart.write_rom_control(0x0000, 0x0A);
    cart.write_ram_byte(0xA000, 0x11);
    cart.write_rom_control(0x4000, 0x01);
    cart.write_ram_byte(0xA000, 0x22);
    cart.write_rom_control(0x4000, 0x00);
    assert_eq!(cart.read_ram_byte(0xA000), 0x11);
    cart.write_rom_control(0x4000, 0x01);
    assert_eq!(cart.read_ram_byte(0xA000), 0x22);
}

#[test]
fn mbc5_non_rumble_uses_full_4bit_ram_bank_register() {
    let rom = make_rom(64 * 1024, MBC5_RAM, 0x01, 0x04);
    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 ROM should load");
    cart.write_rom_control(0x0000, 0x0A);

    cart.write_rom_control(0x4000, 0x00);
    cart.write_ram_byte(0xA000, 0x11);

    cart.write_rom_control(0x4000, 0x08);
    cart.write_ram_byte(0xA000, 0x88);

    cart.write_rom_control(0x4000, 0x00);
    assert_eq!(cart.read_ram_byte(0xA000), 0x11);
    cart.write_rom_control(0x4000, 0x08);
    assert_eq!(cart.read_ram_byte(0xA000), 0x88);
    assert!(!cart.has_rumble());
    assert!(!cart.rumble_active());
}

#[test]
fn mbc5_rumble_masks_ram_bank_bit3_and_tracks_motor_state() {
    let rom = make_rom(64 * 1024, MBC5_RUMBLE_RAM_BATTERY, 0x01, 0x04);
    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC5 RUMBLE ROM should load");
    cart.write_rom_control(0x0000, 0x0A);

    cart.write_rom_control(0x4000, 0x00);
    cart.write_ram_byte(0xA000, 0x11);

    cart.write_rom_control(0x4000, 0x08);
    assert!(cart.rumble_active());
    cart.write_ram_byte(0xA000, 0x22);

    cart.write_rom_control(0x4000, 0x00);
    assert!(!cart.rumble_active());
    assert_eq!(cart.read_ram_byte(0xA000), 0x22);

    cart.write_rom_control(0x4000, 0x01);
    cart.write_ram_byte(0xA000, 0x33);

    cart.write_rom_control(0x4000, 0x09);
    assert!(cart.rumble_active());
    assert_eq!(cart.read_ram_byte(0xA000), 0x33);

    cart.write_rom_control(0x4000, 0x00);
    assert_eq!(cart.read_ram_byte(0xA000), 0x22);
    assert!(cart.has_rumble());
}

#[test]
fn mbc2_switches_rom_bank_and_uses_4bit_ram_cells() {
    let mut rom = make_rom(64 * 1024, MBC2_BATTERY, 0x01, 0x00);
    rom[0x4000] = 0x11;
    rom[0x8000] = 0x22;

    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC2 ROM should load");
    assert_eq!(cart.read_rom_byte(0x4000), 0x11);

    // MBC2 ROM banking: use an address with A8=1.
    cart.write_rom_control(0x2100, 0x02);
    assert_eq!(cart.read_rom_byte(0x4000), 0x22);

    // RAM is disabled until A8=0 write with 0x0A.
    cart.write_ram_byte(0xA000, 0xAB);
    assert_eq!(cart.read_ram_byte(0xA000), 0xFF);

    cart.write_rom_control(0x0000, 0x0A);
    cart.write_ram_byte(0xA000, 0xAB);
    assert_eq!(cart.read_ram_byte(0xA000), 0xFB);

    // A000 and A200 alias because MBC2 RAM is 512 x 4-bit.
    assert_eq!(cart.read_ram_byte(0xA200), 0xFB);
}

#[test]
fn mbc3_switches_rom_bank_and_maps_zero_to_one() {
    let mut rom = make_rom(64 * 1024, MBC3, 0x01, 0x00);
    rom[0x0000] = 0x10;
    rom[0x4000] = 0x11;
    rom[0x8000] = 0x22;

    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC3 ROM should load");
    assert_eq!(cart.read_rom_byte(0x4000), 0x11);

    cart.write_rom_control(0x2000, 0x02);
    assert_eq!(cart.read_rom_byte(0x4000), 0x22);

    cart.write_rom_control(0x2000, 0x00);
    assert_eq!(cart.read_rom_byte(0x4000), 0x11);
}

#[test]
fn mbc3_banks_ram_and_latches_rtc_registers() {
    let rom = make_rom(256 * 1024, MBC3_TIMER_RAM_BATTERY, 0x03, 0x03);
    let mut cart = Cartridge::from_bytes(rom).expect("valid MBC3 ROM should load");

    cart.write_rom_control(0x0000, 0x0A); // RAM/RTC enable

    cart.write_rom_control(0x4000, 0x00);
    cart.write_ram_byte(0xA000, 0x11);
    cart.write_rom_control(0x4000, 0x01);
    cart.write_ram_byte(0xA000, 0x22);
    cart.write_rom_control(0x4000, 0x00);
    assert_eq!(cart.read_ram_byte(0xA000), 0x11);
    cart.write_rom_control(0x4000, 0x01);
    assert_eq!(cart.read_ram_byte(0xA000), 0x22);

    // RTC seconds register select.
    cart.write_rom_control(0x4000, 0x08);
    cart.write_ram_byte(0xA000, 10);

    // Latch 0->1 captures snapshot.
    cart.write_rom_control(0x6000, 0x00);
    cart.write_rom_control(0x6000, 0x01);
    cart.write_ram_byte(0xA000, 20);

    // Reads use latched snapshot until next latch.
    assert_eq!(cart.read_ram_byte(0xA000), 10);
}
