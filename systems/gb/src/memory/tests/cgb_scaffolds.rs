use super::*;

#[test]
fn cgb_mmio_scaffold_decodes_key1_vbk_and_svbk_registers() {
    assert_eq!(cgb_mmio_register(0xFF4D), Some(CgbMmioRegister::Key1));
    assert_eq!(cgb_mmio_register(0xFF4F), Some(CgbMmioRegister::Vbk));
    assert_eq!(cgb_mmio_register(0xFF68), Some(CgbMmioRegister::Bgpi));
    assert_eq!(cgb_mmio_register(0xFF69), Some(CgbMmioRegister::Bgpd));
    assert_eq!(cgb_mmio_register(0xFF6A), Some(CgbMmioRegister::Obpi));
    assert_eq!(cgb_mmio_register(0xFF6B), Some(CgbMmioRegister::Obpd));
    assert_eq!(cgb_mmio_register(0xFF70), Some(CgbMmioRegister::Svbk));
    assert_eq!(cgb_mmio_register(0xFF4C), None);
    assert_eq!(cgb_mmio_register(0xFF50), None);
}

#[test]
fn cgb_dma_mmio_scaffold_decodes_hdma_registers() {
    assert_eq!(
        cgb_dma_mmio_register(0xFF51),
        Some(CgbDmaMmioRegister::Hdma1)
    );
    assert_eq!(
        cgb_dma_mmio_register(0xFF52),
        Some(CgbDmaMmioRegister::Hdma2)
    );
    assert_eq!(
        cgb_dma_mmio_register(0xFF53),
        Some(CgbDmaMmioRegister::Hdma3)
    );
    assert_eq!(
        cgb_dma_mmio_register(0xFF54),
        Some(CgbDmaMmioRegister::Hdma4)
    );
    assert_eq!(
        cgb_dma_mmio_register(0xFF55),
        Some(CgbDmaMmioRegister::Hdma5)
    );
    assert_eq!(cgb_dma_mmio_register(0xFF50), None);
    assert_eq!(cgb_dma_mmio_register(0xFF56), None);
}

#[test]
fn cgb_mmio_scaffold_registers_are_dmg_noops_but_capture_shadow_bits() {
    let mut bus = make_test_bus();

    // DMG-visible behavior remains unmapped-like (0xFF) reads.
    assert_eq!(bus.read_byte(0xFF4D), 0xFF);
    assert_eq!(bus.read_byte(0xFF4F), 0xFF);
    assert_eq!(bus.read_byte(0xFF68), 0xFF);
    assert_eq!(bus.read_byte(0xFF69), 0xFF);
    assert_eq!(bus.read_byte(0xFF6A), 0xFF);
    assert_eq!(bus.read_byte(0xFF6B), 0xFF);
    assert_eq!(bus.read_byte(0xFF70), 0xFF);

    bus.write_byte(0xFF4D, 0x81);
    bus.write_byte(0xFF4F, 0xA3);
    bus.write_byte(0xFF68, 0x83); // BGPI idx=3 auto-inc
    bus.write_byte(0xFF69, 0x12); // BGPD[3]=0x12, idx->4
    bus.write_byte(0xFF69, 0x34); // BGPD[4]=0x34, idx->5
    bus.write_byte(0xFF6A, 0x02); // OBPI idx=2 no auto-inc
    bus.write_byte(0xFF6B, 0x56); // OBPD[2]=0x56
    bus.write_byte(0xFF70, 0xFE);

    assert_eq!(bus.read_byte(0xFF4D), 0xFF);
    assert_eq!(bus.read_byte(0xFF4F), 0xFF);
    assert_eq!(bus.read_byte(0xFF68), 0xFF);
    assert_eq!(bus.read_byte(0xFF69), 0xFF);
    assert_eq!(bus.read_byte(0xFF6A), 0xFF);
    assert_eq!(bus.read_byte(0xFF6B), 0xFF);
    assert_eq!(bus.read_byte(0xFF70), 0xFF);

    assert_eq!(
        bus.debug_cgb_mmio_shadows(),
        (0x01, 0x01, 0x06),
        "scaffolding should store masked future-relevant bits while remaining DMG-noop"
    );
    assert_eq!(
        bus.debug_cgb_palette_index_shadows(),
        (0x85, 0x02),
        "palette index scaffolds should store masked index/autoincrement bits and advance BGPI on BGPD writes when auto-increment is set"
    );
    assert_eq!(bus.debug_cgb_palette_shadow_byte(false, 0x03), 0x12);
    assert_eq!(bus.debug_cgb_palette_shadow_byte(false, 0x04), 0x34);
    assert_eq!(bus.debug_cgb_palette_shadow_byte(true, 0x02), 0x56);
}

#[test]
fn cgb_mmio_bank_selection_scaffold_is_connected_but_dmg_fixed() {
    let mut bus = make_test_bus();

    assert_eq!(bus.debug_cgb_effective_bank_selection(), (0, 1));

    bus.write_byte(0xFF4F, 0x01);
    bus.write_byte(0xFF70, 0x07);

    assert_eq!(
        bus.debug_cgb_effective_bank_selection(),
        (0, 1),
        "DMG scope should keep effective VRAM/WRAM bank selection fixed even when VBK/SVBK shadows change"
    );

    bus.write_vram(0x8000, 0x5A, SegmentAccess::Hardware);
    bus.write_wram(0xD000, 0xC3);
    assert_eq!(bus.read_vram(0x8000, SegmentAccess::Hardware), 0x5A);
    assert_eq!(bus.read_wram(0xD000), 0xC3);
}

#[test]
fn cgb_mmio_bank_selection_scaffold_uses_real_multibank_backing_storage() {
    let mut bus = make_test_bus();

    assert_eq!(
        bus.debug_storage_bank_backing_lengths(),
        (0x4000, 0x8000),
        "CGB-ready scaffold should allocate full VRAM/WRAM backing even in DMG mode"
    );

    bus.write_vram_bank_index_internal(0, 0x0123, 0x11);
    bus.write_vram_bank_index_internal(1, 0x0123, 0x22);
    assert_eq!(bus.read_vram_bank_index_internal(0, 0x0123), 0x11);
    assert_eq!(bus.read_vram_bank_index_internal(1, 0x0123), 0x22);
    assert_eq!(
        bus.read_vram(0x8123, SegmentAccess::Hardware),
        0x11,
        "DMG effective VRAM bank should still resolve to bank 0"
    );

    bus.write_wram_bank_index_internal(1, 0x0042, 0x33);
    bus.write_wram_bank_index_internal(2, 0x0042, 0x44);
    assert_eq!(bus.read_wram_bank_index_internal(1, 0x0042), 0x33);
    assert_eq!(bus.read_wram_bank_index_internal(2, 0x0042), 0x44);
    assert_eq!(
        bus.read_wram(0xD042),
        0x33,
        "DMG effective switchable WRAM slot should stay pinned to slot 1"
    );
}
