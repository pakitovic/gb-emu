use super::*;

#[test]
fn dma_scheduler_mode_edge_events_expose_oam_transfer_lifecycle() {
    let mut bus = make_test_bus();

    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);
    assert!(!bus.debug_dma_mode_edge_events().entered_oam);
    assert!(!bus.debug_dma_mode_edge_events().exited_oam);

    bus.write_byte(0xFF46, 0x80);
    bus.tick(7);
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);
    assert!(!bus.debug_dma_mode_edge_events().entered_oam);

    bus.tick(1);
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Oam);
    assert!(
        bus.debug_dma_mode_edge_events().entered_oam,
        "OAM DMA start should pulse an entered_oam edge on the scheduler tick that starts the transfer"
    );
    assert!(
        !bus.debug_dma_mode_edge_events().exited_oam,
        "OAM DMA start tick should not also emit exited_oam"
    );

    bus.tick(1);
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Oam);
    assert_eq!(
        bus.debug_dma_mode_edge_events(),
        Default::default(),
        "DMA mode entry edge should be a single-tick pulse"
    );

    tick_n(&mut bus, 638);
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Oam);
    assert_eq!(bus.debug_dma_mode_edge_events(), Default::default());

    bus.tick(1);
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);
    assert!(
        bus.debug_dma_mode_edge_events().exited_oam,
        "OAM DMA completion should pulse an exited_oam edge"
    );
    assert!(
        !bus.debug_dma_mode_edge_events().entered_oam,
        "OAM DMA completion tick should not emit entered_oam without a restart"
    );

    bus.tick(1);
    assert_eq!(bus.debug_dma_mode_edge_events(), Default::default());
}

#[test]
fn dma_scheduler_centralizes_dmg_cpu_access_policy_for_segments() {
    let mut bus = make_test_bus();

    for &segment in &[
        AddressSegment::Oam,
        AddressSegment::Vram,
        AddressSegment::Wram,
    ] {
        assert_eq!(
            bus.dma_cpu_access_decision_for_segment(segment, DmaCpuAccessKind::Read),
            DmaCpuAccessDecision::Allow,
            "DMA should not block CPU segment access while idle ({segment:?}, read)"
        );
        assert_eq!(
            bus.dma_cpu_access_decision_for_segment(segment, DmaCpuAccessKind::Write),
            DmaCpuAccessDecision::Allow,
            "DMA should not block CPU segment access while idle ({segment:?}, write)"
        );
    }

    bus.write_byte(0xFF46, 0x80);
    bus.tick(8); // OAM DMA active

    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Oam);
    assert_eq!(
        bus.dma_cpu_access_decision_for_segment(AddressSegment::Oam, DmaCpuAccessKind::Read),
        DmaCpuAccessDecision::Block
    );
    assert_eq!(
        bus.dma_cpu_access_decision_for_segment(AddressSegment::Oam, DmaCpuAccessKind::Write),
        DmaCpuAccessDecision::Block
    );

    for &segment in &[
        AddressSegment::Rom,
        AddressSegment::Vram,
        AddressSegment::Wram,
        AddressSegment::EchoWram,
        AddressSegment::Io,
        AddressSegment::Hram,
        AddressSegment::Ie,
    ] {
        assert_eq!(
            bus.dma_cpu_access_decision_for_segment(segment, DmaCpuAccessKind::Read),
            DmaCpuAccessDecision::Allow,
            "DMG OAM DMA scheduler scaffold should currently block only OAM (read {segment:?})"
        );
        assert_eq!(
            bus.dma_cpu_access_decision_for_segment(segment, DmaCpuAccessKind::Write),
            DmaCpuAccessDecision::Allow,
            "DMG OAM DMA scheduler scaffold should currently block only OAM (write {segment:?})"
        );
    }
}

#[test]
fn dma_scheduler_debug_guard_preserves_progress_under_stress() {
    let mut bus = make_test_bus();

    // Prime a couple of DMA source pages with distinct patterns so the guard
    // also validates real scheduler work, not only idle fast-path ticking.
    for i in 0..0x100u16 {
        bus.write_wram(0xC000 + i, (i as u8).wrapping_mul(3).wrapping_add(1));
        bus.write_wram(0xC100 + i, (i as u8).wrapping_mul(5).wrapping_add(7));
    }

    // Stress the common idle path in debug builds. This is intentionally a
    // timeout-based guard (script/CI owns the threshold), not a timing assert.
    for _ in 0..32_000_000u32 {
        bus.tick_dma_scheduler_tcycle();
    }
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);

    // Then stress active OAM DMA scheduling and completion repeatedly.
    for round in 0..4_096u16 {
        let source_high = if (round & 1) == 0 { 0xC0 } else { 0xC1 };
        bus.write_byte(0xFF46, source_high);
        for _ in 0..(8 + (0xA0 * 4)) {
            bus.tick_dma_scheduler_tcycle();
        }
        assert_eq!(
            bus.debug_dma_mode_kind(),
            DmaSchedulerMode::Idle,
            "DMA scheduler should complete OAM transfer and return to idle each round"
        );
    }

    // Final transfer should come from page C1 and copy the expected bytes.
    assert_eq!(bus.read_oam(0xFE00, SegmentAccess::Hardware), 0x07);
    assert_eq!(bus.read_oam(0xFE01, SegmentAccess::Hardware), 0x0C);
    assert_eq!(bus.read_oam(0xFE02, SegmentAccess::Hardware), 0x11);
}

#[test]
fn cgb_dma_mmio_scaffold_registers_are_dmg_noops_but_capture_shadow_bits_and_request_mode() {
    let mut bus = make_test_bus();

    assert_eq!(bus.read_byte(0xFF51), 0xFF);
    assert_eq!(bus.read_byte(0xFF52), 0xFF);
    assert_eq!(bus.read_byte(0xFF53), 0xFF);
    assert_eq!(bus.read_byte(0xFF54), 0xFF);
    assert_eq!(bus.read_byte(0xFF55), 0xFF);

    bus.write_byte(0xFF51, 0xAB);
    bus.write_byte(0xFF52, 0xCD);
    bus.write_byte(0xFF53, 0xEF);
    bus.write_byte(0xFF54, 0x12);
    bus.write_byte(0xFF55, 0x83); // HDMA mode request scaffold

    assert_eq!(bus.read_byte(0xFF51), 0xFF);
    assert_eq!(bus.read_byte(0xFF52), 0xFF);
    assert_eq!(bus.read_byte(0xFF53), 0xFF);
    assert_eq!(bus.read_byte(0xFF54), 0xFF);
    assert_eq!(bus.read_byte(0xFF55), 0xFF);
    assert_eq!(
        bus.debug_cgb_dma_scaffold_shadows(),
        (0xAB, 0xC0, 0x0F, 0x10, 0x83),
        "HDMA scaffold should capture masked future-relevant fields while remaining DMG-noop"
    );
    assert_eq!(
        bus.debug_cgb_dma_scaffold_last_requested_mode(),
        Some(DmaSchedulerMode::Hdma)
    );
    assert_eq!(
        bus.debug_dma_mode_kind(),
        DmaSchedulerMode::Idle,
        "DMG scope should not enter CGB DMA scheduler modes when HDMA registers are written"
    );
    assert_eq!(
        bus.debug_dma_mode_edge_events(),
        Default::default(),
        "HDMA scaffold writes should not emit DMA scheduler mode edges in DMG scope"
    );

    bus.write_byte(0xFF55, 0x04); // GDMA mode request scaffold
    assert_eq!(
        bus.debug_cgb_dma_scaffold_last_requested_mode(),
        Some(DmaSchedulerMode::Gdma)
    );
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);
}

#[test]
fn cgb_dma_scaffold_runtime_is_model_gated_off_for_current_dmg_family_models() {
    for model in [
        HardwareModel::Dmg0,
        HardwareModel::Dmg,
        HardwareModel::Mgb,
        HardwareModel::Sgb,
        HardwareModel::Sgb2,
    ] {
        let bus = make_test_bus_with_model(model);
        assert!(
            !bus.debug_cgb_dma_scaffold_runtime_enabled(),
            "CGB DMA runtime scaffold must stay gated off for current DMG-family model {model:?}"
        );
    }
}

#[test]
fn cgb_dma_gdma_scaffold_can_transfer_one_block_when_runtime_is_test_enabled() {
    let mut bus = make_test_bus();
    bus.debug_force_enable_cgb_dma_scaffold_runtime(true);

    for i in 0..0x20u16 {
        bus.write_wram(0xC000 + i, (i as u8).wrapping_mul(7).wrapping_add(3));
    }

    bus.write_byte(0xFF51, 0xC0);
    bus.write_byte(0xFF52, 0x0F); // low nibble masked, source aligns to C000
    bus.write_byte(0xFF53, 0x00);
    bus.write_byte(0xFF54, 0x00);
    bus.write_byte(0xFF55, 0x00); // GDMA, 1 block (0x10 bytes)

    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);
    assert_eq!(
        bus.debug_cgb_dma_scaffold_last_requested_mode(),
        Some(DmaSchedulerMode::Gdma)
    );
    assert_eq!(
        bus.debug_cgb_dma_transfer_scaffold_state(),
        (0xC000, 0x8000, 1)
    );

    bus.tick(1);
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Gdma);
    assert!(bus.debug_dma_mode_edge_events().entered_gdma);

    bus.tick(1);
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);
    assert!(bus.debug_dma_mode_edge_events().exited_gdma);
    assert_eq!(
        bus.debug_cgb_dma_transfer_scaffold_state(),
        (0xC010, 0x8010, 0)
    );

    for i in 0..0x10u16 {
        assert_eq!(
            bus.read_vram(0x8000 + i, SegmentAccess::Hardware),
            ((i as u8).wrapping_mul(7)).wrapping_add(3),
            "GDMA scaffold should copy the first 0x10-byte block into VRAM"
        );
    }
    assert_eq!(
        bus.read_vram(0x8010, SegmentAccess::Hardware),
        0x00,
        "GDMA scaffold with length=0 should stop after one block"
    );
}

#[test]
fn cgb_dma_hdma_scaffold_uses_hblank_edges_and_stop_request_when_runtime_is_test_enabled() {
    let mut bus = make_test_bus();
    bus.debug_force_enable_cgb_dma_scaffold_runtime(true);

    for i in 0..0x20u16 {
        bus.write_wram(0xC000 + i, 0x80u8.wrapping_add(i as u8));
    }

    bus.write_byte(0xFF51, 0xC0);
    bus.write_byte(0xFF52, 0x00);
    bus.write_byte(0xFF53, 0x00);
    bus.write_byte(0xFF54, 0x20); // dest 0x8020
    bus.write_byte(0xFF55, 0x81); // HDMA, 2 blocks

    bus.tick(1); // scheduler latches pending request into active HDMA mode
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Hdma);
    assert!(bus.debug_dma_mode_edge_events().entered_hdma);
    assert_eq!(
        bus.read_vram(0x8020, SegmentAccess::Hardware),
        0x00,
        "HDMA scaffold should wait for an HBlank entry edge before copying a block"
    );

    let mut saw_hblank_transfer = false;
    for _ in 0..(154 * 456 * 2) {
        bus.tick(1);
        if bus.debug_ppu_mode_edge_events().entered_hblank {
            saw_hblank_transfer = true;
            break;
        }
    }
    assert!(
        saw_hblank_transfer,
        "Expected an HBlank edge for HDMA scaffold test"
    );

    for i in 0..0x10u16 {
        assert_eq!(
            bus.read_vram(0x8020 + i, SegmentAccess::Hardware),
            0x80u8.wrapping_add(i as u8),
            "HDMA scaffold should copy exactly one block on each HBlank edge"
        );
    }
    assert_eq!(
        bus.read_vram(0x8030, SegmentAccess::Hardware),
        0x00,
        "Second HDMA block should not copy until a later HBlank edge"
    );
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Hdma);
    assert_eq!(
        bus.debug_cgb_dma_transfer_scaffold_state(),
        (0xC010, 0x8030, 1)
    );

    bus.write_byte(0xFF55, 0x00); // stop active HDMA (CGB behavior scaffolded, DMG public behavior unaffected)
    assert_eq!(bus.debug_dma_mode_kind(), DmaSchedulerMode::Idle);

    let before_second_block = bus.read_vram(0x8030, SegmentAccess::Hardware);
    let mut saw_next_hblank = false;
    for _ in 0..(154 * 456 * 2) {
        bus.tick(1);
        if bus.debug_ppu_mode_edge_events().entered_hblank {
            saw_next_hblank = true;
            break;
        }
    }
    assert!(
        saw_next_hblank,
        "Expected another HBlank edge after HDMA stop"
    );
    assert_eq!(
        bus.read_vram(0x8030, SegmentAccess::Hardware),
        before_second_block,
        "Stopping the HDMA scaffold should prevent later HBlank edges from copying more blocks"
    );
}

#[test]
fn oam_dma_transfers_160_bytes_and_finishes_after_start_delay() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFE00, 0xAA);
    bus.write_byte(0x8000, 0x12);
    bus.write_byte(0x809F, 0x34);

    bus.write_byte(0xFF46, 0x80);
    // Fresh DMA keeps OAM accessible for one M-cycle.
    assert_eq!(bus.read_byte(0xFE00), 0xAA);

    bus.tick(8);
    // DMA starts at M=2; OAM reads are now blocked.
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..640 {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte_raw(0xFE00), 0x12);
    assert_eq!(bus.read_byte_raw(0xFE9F), 0x34);
}

#[test]
fn tick_chunking_preserves_oam_dma_progress_and_timer_surface() {
    let mut chunked = make_test_bus();
    let mut single = make_test_bus();

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF40, 0x00); // LCD off to avoid OAM read blocking during verification
        bus.write_byte(0xFF07, 0x05);
        bus.write_byte(0xFF06, 0x33);
        bus.write_byte(0xFF05, 0xF0);
        for i in 0..0xA0u16 {
            bus.write_byte(0xC000 + i, ((i as u8).wrapping_mul(3)).wrapping_add(1));
        }
        bus.write_byte(0xFF46, 0xC0);
    }
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 160, 200] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }

    for i in 0..0xA0u16 {
        let addr = 0xFE00 + i;
        assert_eq!(
            chunked.read_byte(addr),
            single.read_byte(addr),
            "OAM mismatch at {:04X} after DMA under different tick chunking",
            addr
        );
    }
}

#[test]
fn oam_dma_blocks_cpu_writes_to_oam_during_transfer() {
    let mut bus = make_test_bus();
    bus.write_byte(0x8000, 0x55);
    bus.write_byte(0xFF46, 0x80);
    bus.tick(8);

    bus.write_byte(0xFE00, 0xAA); // ignored while DMA active
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..640 {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte_raw(0xFE00), 0x55);
}

#[test]
fn oam_dma_restart_switches_source_after_two_mcycles() {
    let mut bus = make_test_bus();
    bus.write_byte(0x8000, 0x11);
    bus.write_byte(0x8100, 0x22);

    bus.write_byte(0xFF46, 0x80);
    bus.tick(8); // DMA starts
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    bus.write_byte(0xFF46, 0x81); // request restart
    bus.tick(4); // M=1 after restart request
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..644 {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte_raw(0xFE00), 0x22);
}

#[test]
fn oam_dma_restart_keeps_previous_transfer_running_during_full_restart_delay() {
    let mut bus = make_test_bus();

    // Source A pattern.
    bus.write_byte(0x8000, 0xA0);
    bus.write_byte(0x8001, 0xA1);
    bus.write_byte(0x8002, 0xA2);
    // Source B distinct first bytes.
    bus.write_byte(0x8100, 0xB0);
    bus.write_byte(0x8101, 0xB1);

    bus.write_byte(0xFF46, 0x80);
    bus.tick(8); // DMA A starts.
    bus.tick(4); // Copy first byte from A -> OAM[0].
    assert_eq!(bus.read_byte_raw(0xFE00), 0xA0);

    bus.write_byte(0xFF46, 0x81); // request restart to source B
    bus.tick(8); // full restart delay window

    // Previous DMA should keep running during all 8 t-cycles of restart delay.
    assert_eq!(bus.read_byte_raw(0xFE01), 0xA1);
    assert_eq!(bus.read_byte_raw(0xFE02), 0xA2);

    // New DMA should take over after the delay and restart from OAM index 0.
    bus.tick(4);
    assert_eq!(bus.read_byte_raw(0xFE00), 0xB0);
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
    assert_eq!(bus.read_byte_raw(0xFE00), 0x66);

    bus.write_byte(0xFF46, 0xFF);
    for _ in 0..648 {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte_raw(0xFE00), 0x77);
}
