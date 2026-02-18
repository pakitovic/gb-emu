use super::*;

fn make_test_bus() -> Bus {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    Bus::new(cart)
}

#[test]
fn echo_ram_mirrors_work_ram() {
    let mut bus = make_test_bus();
    bus.write_byte(0xC123, 0xAB);
    assert_eq!(bus.read_byte(0xE123), 0xAB);

    bus.write_byte(0xE456, 0xCD);
    assert_eq!(bus.read_byte(0xC456), 0xCD);
}

#[test]
fn div_increments_every_256_tcycles_and_resets_on_write() {
    let mut bus = make_test_bus();
    assert_eq!(bus.read_byte(0xFF04), 0x00);

    bus.tick(255);
    assert_eq!(bus.read_byte(0xFF04), 0x00);

    bus.tick(1);
    assert_eq!(bus.read_byte(0xFF04), 0x01);

    bus.write_byte(0xFF04, 0x99);
    assert_eq!(bus.read_byte(0xFF04), 0x00);
}

#[test]
fn timer_overflow_reloads_tma_and_requests_interrupt() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF07, 0x05); // TAC: enable + 16 t-cycles period
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(16);
    assert_eq!(bus.read_byte(0xFF05), 0x00);
    assert_eq!(bus.interrupt_flags() & (1 << 2), 0);

    bus.tick(4);

    assert_eq!(bus.read_byte(0xFF05), 0x42);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn div_write_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // div bit3 becomes high
    bus.write_byte(0xFF04, 0x00); // reset DIV => falling edge => TIMA++

    assert_eq!(bus.read_byte(0xFF05), 0x01);
}

#[test]
fn tima_write_during_reload_cancels_pending_reload() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(16); // overflow -> pending reload (4 cycles)
    assert_eq!(bus.read_byte(0xFF05), 0x00);

    bus.write_byte(0xFF05, 0x99); // cancel reload
    bus.tick(4);

    assert_eq!(bus.read_byte(0xFF05), 0x99);
    assert_eq!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn tima_write_on_reload_cycle_is_ignored() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(20); // overflow + reload happened; reload block active
    assert_eq!(bus.read_byte(0xFF05), 0x42);

    bus.write_byte(0xFF05, 0x99); // ignored during reload block

    assert_eq!(bus.read_byte(0xFF05), 0x42);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn tma_write_on_reload_cycle_updates_reloaded_tima() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(19); // overflow happened, 1 t-cycle left for reload
    bus.write_byte(0xFF06, 0x99); // updates TMA and imminent reload value
    bus.tick(1);

    assert_eq!(bus.read_byte(0xFF05), 0x99);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn entering_vblank_requests_vblank_interrupt() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    // 144 scanlines * 456 t-cycles per line.
    for _ in 0..(144 * 456) {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte(0xFF44), 144);
    assert_ne!(bus.interrupt_flags() & (1 << 0), 0);
}

#[test]
fn oam_dma_transfers_160_bytes_and_finishes_after_start_delay() {
    let mut bus = make_test_bus();
    bus.write_byte(0x8000, 0x12);
    bus.write_byte(0x809F, 0x34);

    bus.write_byte(0xFF46, 0x80);

    // During DMA, OAM reads are blocked.
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..648 {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte(0xFE00), 0x12);
    assert_eq!(bus.read_byte(0xFE9F), 0x34);
}

#[test]
fn oam_dma_blocks_cpu_writes_to_oam_during_transfer() {
    let mut bus = make_test_bus();
    bus.write_byte(0x8000, 0x55);
    bus.write_byte(0xFF46, 0x80);

    bus.write_byte(0xFE00, 0xAA); // ignored while DMA active
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..648 {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte(0xFE00), 0x55);
}

#[test]
fn oam_dma_remaps_fe_ff_sources_to_de_df_on_dmg() {
    let mut bus = make_test_bus();
    bus.write_byte(0xDE00, 0x66);
    bus.write_byte(0xDF00, 0x77);
    bus.write_byte(0xFE00, 0x11);

    bus.write_byte(0xFF46, 0xFE);
    for _ in 0..648 {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte(0xFE00), 0x66);

    bus.write_byte(0xFF46, 0xFF);
    for _ in 0..648 {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte(0xFE00), 0x77);
}
