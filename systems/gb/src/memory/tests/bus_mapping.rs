use super::*;

#[test]
fn echo_ram_mirrors_work_ram() {
    let mut bus = make_test_bus();
    bus.write_byte(0xC123, 0xAB);
    assert_eq!(bus.read_byte(0xE123), 0xAB);

    bus.write_byte(0xE456, 0xCD);
    assert_eq!(bus.read_byte(0xC456), 0xCD);
}

#[test]
fn address_segment_classifies_main_bus_regions() {
    assert_eq!(address_segment(0x0000), AddressSegment::Rom);
    assert_eq!(address_segment(0x8000), AddressSegment::Vram);
    assert_eq!(address_segment(0xA000), AddressSegment::Eram);
    assert_eq!(address_segment(0xC000), AddressSegment::Wram);
    assert_eq!(address_segment(0xE000), AddressSegment::EchoWram);
    assert_eq!(address_segment(0xFE00), AddressSegment::Oam);
    assert_eq!(address_segment(0xFEA0), AddressSegment::NotUsable);
    assert_eq!(address_segment(0xFF00), AddressSegment::Io);
    assert_eq!(address_segment(0xFF80), AddressSegment::Hram);
    assert_eq!(address_segment(0xFFFF), AddressSegment::Ie);
}

#[test]
fn wram_segment_helpers_mirror_main_and_echo_regions() {
    let mut bus = make_test_bus();

    bus.write_wram(0xC123, 0xAB);
    assert_eq!(bus.read_wram(0xE123), 0xAB);

    bus.write_wram(0xE456, 0xCD);
    assert_eq!(bus.read_wram(0xC456), 0xCD);
}

#[test]
fn vram_segment_helpers_centralize_cpu_blocking_and_internal_access() {
    let mut bus = make_test_bus();
    bus.write_vram(0x8000, 0x3C, SegmentAccess::Hardware);

    wait_for_ly_mode(&mut bus, 1, 3);

    assert_eq!(bus.read_vram(0x8000, SegmentAccess::Cpu), 0xFF);
    assert_eq!(bus.read_vram(0x8000, SegmentAccess::Hardware), 0x3C);

    bus.write_vram(0x8000, 0x55, SegmentAccess::Cpu);
    assert_eq!(bus.read_vram(0x8000, SegmentAccess::Hardware), 0x3C);

    bus.write_vram(0x8000, 0x55, SegmentAccess::Hardware);
    assert_eq!(bus.read_vram(0x8000, SegmentAccess::Hardware), 0x55);
}

#[test]
fn oam_segment_helpers_centralize_cpu_blocking_and_internal_access() {
    let mut bus = make_test_bus();
    bus.write_oam(0xFE00, 0x12, SegmentAccess::Hardware);

    bus.write_byte(0xFF46, 0x80);
    bus.tick(8); // DMA active after start delay, before first transferred byte lands.
    assert!(
        bus.ppu_blocks_oam_read(),
        "expected OAM read block while DMA is active"
    );
    assert!(
        bus.ppu_blocks_oam_write(),
        "expected OAM write block while DMA is active"
    );

    assert_eq!(bus.read_oam(0xFE00, SegmentAccess::Cpu), 0xFF);
    assert_eq!(bus.read_oam(0xFE00, SegmentAccess::Hardware), 0x12);

    bus.write_oam(0xFE00, 0x34, SegmentAccess::Cpu);
    assert_eq!(bus.read_oam(0xFE00, SegmentAccess::Hardware), 0x12);

    bus.write_oam(0xFE00, 0x34, SegmentAccess::Hardware);
    assert_eq!(bus.read_oam(0xFE00, SegmentAccess::Hardware), 0x34);
}

#[test]
fn unmapped_io_reads_as_ff_and_ignores_writes() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF03, 0x00);
    assert_eq!(bus.read_byte(0xFF03), 0xFF);

    bus.write_byte(0xFF4C, 0x00);
    assert_eq!(bus.read_byte(0xFF4C), 0xFF);

    bus.write_byte(0xFF4C, 0xAA);
    assert_eq!(bus.read_byte(0xFF4C), 0xFF);
}

#[test]
fn io_register_unused_bits_read_back_as_one() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x00);
    assert_eq!(bus.read_byte(0xFF00) & 0xC0, 0xC0);

    bus.write_byte(0xFF02, 0x00);
    assert_eq!(bus.read_byte(0xFF02) & 0x7E, 0x7E);

    bus.write_byte(0xFF07, 0x00);
    assert_eq!(bus.read_byte(0xFF07) & 0xF8, 0xF8);

    bus.write_byte(0xFF41, 0x00);
    assert_eq!(bus.read_byte(0xFF41) & 0x80, 0x80);

    bus.write_byte(0xFF1A, 0x00);
    assert_eq!(bus.read_byte(0xFF1A) & 0x7F, 0x7F);

    bus.write_byte(0xFF26, 0x00);
    assert_eq!(bus.read_byte(0xFF26) & 0x70, 0x70);
}
