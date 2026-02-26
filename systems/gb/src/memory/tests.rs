use super::bus_access::{AddressSegment, SegmentAccess, address_segment};
use super::cgb_mmio::{CgbMmioRegister, cgb_mmio_register};
use super::dma::{
    CgbDmaMmioRegister, DmaCpuAccessDecision, DmaCpuAccessKind, DmaSchedulerMode,
    cgb_dma_mmio_register,
};
use super::test_utils::{make_test_bus, make_test_bus_with_model, tick_n};
use super::*;
use crate::cartridge::{Cartridge, CartridgeMapper};
use crate::hardware::HardwareModel;
use crate::input::Button;
use crate::timing::DMG_CPU_T_CYCLES_PER_M_CYCLE;

fn wait_for_transition(bus: &mut Bus, ly: u8, from_mode: u8, to_mode: u8) {
    let mut prev_mode = bus.read_byte(0xFF41) & 0x03;
    for _ in 0..(154 * 456 * 2) {
        bus.tick(1);
        let cur_mode = bus.read_byte(0xFF41) & 0x03;
        let cur_ly = bus.read_byte(0xFF44);
        if cur_ly == ly && prev_mode == from_mode && cur_mode == to_mode {
            return;
        }
        prev_mode = cur_mode;
    }
    panic!("Transition LY={ly} {from_mode}->{to_mode} not observed");
}

fn wait_for_ly_mode(bus: &mut Bus, target_ly: u8, target_mode: u8) {
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == target_ly && mode == target_mode {
            return;
        }
        bus.tick(1);
    }
    panic!("LY={target_ly} mode={target_mode} not observed");
}

fn measure_hblank_until_ly_increment(bus: &mut Bus, ly: u8) -> u16 {
    let mut ticks = 0u16;
    for _ in 0..512 {
        if bus.read_byte(0xFF44) != ly {
            return ticks;
        }
        bus.tick(1);
        ticks = ticks.wrapping_add(1);
    }
    panic!("LY did not increment within expected HBlank window");
}

fn wait_for_ly(bus: &mut Bus, target_ly: u8) {
    for _ in 0..(154 * 456 * 2) {
        if bus.read_byte(0xFF44) == target_ly {
            return;
        }
        bus.tick(1);
    }
    panic!("LY={target_ly} not observed");
}

fn wait_for_visible_hblank(bus: &mut Bus) {
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if (1..144).contains(&ly) && mode == 0 {
            return;
        }
        bus.tick(1);
    }
    panic!("Visible HBlank not observed");
}

fn wait_for_next_frame(bus: &mut Bus) {
    let start = bus.frame_counter();
    for _ in 0..(154 * 456 * 2) {
        if bus.frame_counter() > start {
            return;
        }
        bus.tick(1);
    }
    panic!("Frame boundary not observed");
}

// PPU timing-sensitive tests remain under memory because they exercise bus/MMIO/DMA/STAT integration.
mod ppu_framebuffer_integration;
mod ppu_mode3_boundaries_integration;
mod ppu_startup_stat_integration;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimingContractSnapshot {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    iflags: u8,
    ly: u8,
    stat: u8,
    frame_counter: u64,
}

fn timing_contract_snapshot(bus: &Bus) -> TimingContractSnapshot {
    TimingContractSnapshot {
        div: bus.read_byte(0xFF04),
        tima: bus.read_byte(0xFF05),
        tma: bus.read_byte(0xFF06),
        tac: bus.read_byte(0xFF07),
        iflags: bus.interrupt_flags(),
        ly: bus.read_byte(0xFF44),
        stat: bus.read_byte(0xFF41),
        frame_counter: bus.frame_counter(),
    }
}

fn tick_chunk_and_compare_timing_state(chunked: &mut Bus, single: &mut Bus, tcycles: u8) {
    chunked.tick(tcycles);
    for _ in 0..tcycles {
        single.tick(1);
    }

    assert_eq!(
        timing_contract_snapshot(chunked),
        timing_contract_snapshot(single),
        "Bus::tick chunking changed visible timer/PPU timing state after {tcycles} t-cycles chunk"
    );
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
fn bus_internal_cartridge_capabilities_api_surfaces_header_and_mapper_flags() {
    let mut rom = vec![0; 64 * 1024];
    rom[0x0147] = 0x1E; // MBC5 rumble+RAM+battery
    rom[0x0148] = 0x01; // 64 KiB
    rom[0x0149] = 0x03; // 32 KiB RAM
    rom[0x0143] = 0x80;
    let cartridge = Cartridge::from_bytes(rom).expect("valid cartridge should load");
    let mut bus = Bus::new(cartridge);

    let caps = bus.cartridge_capabilities();
    assert_eq!(caps.mapper, CartridgeMapper::Mbc5);
    assert!(caps.has_declared_ram);
    assert!(caps.has_effective_ram);
    assert!(caps.has_battery);
    assert!(!caps.has_timer);
    assert!(caps.has_rumble);
    assert!(caps.has_battery_save);
    assert!(caps.supports_cgb);
    assert!(!caps.cgb_only);

    bus.write_byte(0x4000, 0x08); // enable rumble bit on MBC5 rumble register
    assert!(bus.rumble_active());
    assert!(bus.cartridge_has_rumble());
}

#[test]
fn bus_cartridge_model_compatibility_uses_header_flags_and_selected_model() {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32 KiB
    rom[0x0149] = 0x00;
    rom[0x0143] = 0x80; // CGB-compatible
    rom[0x0146] = 0x03; // SGB-supported
    let dmg_bus = Bus::new_with_model(
        Cartridge::from_bytes(rom.clone()).expect("valid cartridge should load"),
        HardwareModel::Dmg,
    );
    let sgb_bus = Bus::new_with_model(
        Cartridge::from_bytes(rom).expect("valid cartridge should load"),
        HardwareModel::Sgb,
    );

    let dmg_compat = dmg_bus.cartridge_model_compatibility();
    assert!(dmg_compat.mode_request.prefers_cgb());
    assert!(dmg_compat.dmg_mode_allowed);
    assert!(!dmg_compat.cgb_mode_supported_by_model);
    assert!(!dmg_compat.cgb_mode_possible);
    assert!(dmg_compat.sgb_features_requested);
    assert!(!dmg_compat.sgb_features_supported_by_model);

    let sgb_compat = sgb_bus.cartridge_model_compatibility();
    assert!(sgb_compat.sgb_features_requested);
    assert!(sgb_compat.sgb_features_supported_by_model);
    assert!(sgb_compat.sgb_features_possible);
    assert!(!sgb_compat.cgb_mode_possible);
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
fn dmg_family_models_use_fixed_dmg_clock_ratio_policy() {
    for model in [
        HardwareModel::Dmg0,
        HardwareModel::Dmg,
        HardwareModel::Mgb,
        HardwareModel::Sgb,
        HardwareModel::Sgb2,
    ] {
        let bus = make_test_bus_with_model(model);
        assert_eq!(
            bus.cpu_tcycles_for_mcycles(1),
            DMG_CPU_T_CYCLES_PER_M_CYCLE,
            "unexpected CPU m-cycle ratio for model {model:?}"
        );
        assert_eq!(
            bus.cpu_tcycles_for_mcycles(2),
            DMG_CPU_T_CYCLES_PER_M_CYCLE * 2,
            "unexpected 2-mcycle conversion for model {model:?}"
        );
    }
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
fn tick_chunking_preserves_div_tima_state_across_timer_edges_and_control_writes() {
    let mut chunked = make_test_bus();
    let mut single = make_test_bus();

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF07, 0x05); // TAC: enable + 16 t-cycles period
        bus.write_byte(0xFF06, 0x9C); // TMA
        bus.write_byte(0xFF05, 0xFA); // TIMA near overflow
    }
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[1, 3, 4, 8, 15, 2, 17, 9, 11] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF04, 0x00); // DIV reset edge-sensitive behavior
    }
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[5, 7, 13, 1, 16, 4, 3, 19] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF07, 0x07); // switch to another enabled timer source
    }
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[2, 6, 10, 14, 18, 22, 31] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }
}

#[test]
fn tick_chunking_preserves_ly_stat_and_tima_through_visible_mode_transitions() {
    let mut chunked = make_test_bus();
    let mut single = make_test_bus();

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF43, 0x07); // SCX low bits affect visible mode3 timing
        bus.write_byte(0xFF41, 0x28); // enable mode2 + mode0 STAT sources
        bus.write_byte(0xFF07, 0x05); // timer enabled for concurrent DIV/TIMA activity
        bus.write_byte(0xFF06, 0x77);
        bus.write_byte(0xFF05, 0xF8);
    }

    wait_for_ly_mode(&mut chunked, 2, 2);
    wait_for_ly_mode(&mut single, 2, 2);
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 34, 21] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }
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
fn tac_disable_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // selected input bit becomes high
    bus.write_byte(0xFF07, 0x00); // disable timer => falling edge => TIMA++

    assert_eq!(bus.read_byte(0xFF05), 0x01);
}

#[test]
fn tac_frequency_switch_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // bit3 high while bit5 is still low
    bus.write_byte(0xFF07, 0x06); // switch to bit5 source => falling edge => TIMA++

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
fn serial_transfer_completes_after_eight_div_aligned_falling_edges() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'A');
    bus.write_byte(0xFF02, 0x81);

    for _ in 0..4095 {
        bus.tick(1);
    }

    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x80);

    bus.tick(1);

    assert_ne!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x00);
    assert!(bus.serial_output().contains('A'));
}

#[test]
fn serial_transfer_is_phase_aligned_to_div_and_not_to_start_write() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    // Shift DIV phase so completion is not exactly 4096 cycles after SC write.
    bus.tick(7);
    bus.write_byte(0xFF01, b'B');
    bus.write_byte(0xFF02, 0x81);

    for _ in 0..4088 {
        bus.tick(1);
    }
    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);

    bus.tick(1);
    assert_ne!(bus.interrupt_flags() & (1 << 3), 0);
}

#[test]
fn serial_stop_cancels_transfer_and_does_not_request_interrupt() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'X');
    bus.write_byte(0xFF02, 0x81);

    bus.tick(255); // before first serial clock falling edge
    bus.write_byte(0xFF02, 0x00); // explicit stop

    for _ in 0..5000 {
        bus.tick(1);
    }

    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x00);
    assert!(!bus.serial_output().contains('X'));
}

#[test]
fn serial_restart_uses_latest_tx_byte() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'A');
    bus.write_byte(0xFF02, 0x81); // start transfer

    bus.tick(200); // transfer still in progress
    bus.write_byte(0xFF01, b'B');
    bus.write_byte(0xFF02, 0x81); // restart transfer

    let mut finished = false;
    for _ in 0..5000 {
        bus.tick(1);
        if (bus.interrupt_flags() & (1 << 3)) != 0 {
            finished = true;
            break;
        }
    }

    assert!(finished, "serial transfer did not complete after restart");
    assert_eq!(bus.serial_output(), "B");
}

#[test]
fn p1_reads_action_buttons_when_button_group_is_selected() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x10); // P15=0 (buttons), P14=1 (dpad not selected)
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);

    bus.set_button_pressed(Button::A, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);

    bus.set_button_pressed(Button::Start, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x06);
}

#[test]
fn p1_reads_dpad_buttons_when_direction_group_is_selected() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x20); // P14=0 (dpad), P15=1 (buttons not selected)
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);

    bus.set_button_pressed(Button::Right, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);

    bus.set_button_pressed(Button::Up, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0A);
}

#[test]
fn joypad_interrupt_is_requested_on_new_selected_press() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF00, 0x20); // select dpad

    bus.set_button_pressed(Button::Right, true);
    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_interrupt_flags(0x00);
    bus.set_button_pressed(Button::Right, true); // still pressed, no new edge
    assert_eq!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_button_pressed(Button::Right, false);
    assert_eq!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_button_pressed(Button::Right, true); // new falling edge
    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);
}

#[test]
fn joypad_interrupt_can_be_requested_when_selection_changes() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    bus.set_button_pressed(Button::A, true);
    bus.write_byte(0xFF00, 0x10); // select action keys after A is already pressed

    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);
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

#[test]
fn dmg0_boot_profile_uses_expected_div_phase_and_ly_start() {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00;
    rom[0x0148] = 0x00;
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    let bus = Bus::new_with_model(cart, HardwareModel::Dmg0);

    assert_eq!(bus.timer.div_counter, 0x1830);
    assert_eq!(bus.io[0x44], 0x91);
    assert_eq!(bus.ppu.ly_counter, 96);
}

#[test]
fn sgb_boot_div_phase_depends_on_header_checksum() {
    let make_bus = |checksum_hi: u8, checksum_lo: u8| {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        rom[0x014E] = checksum_hi;
        rom[0x014F] = checksum_lo;
        let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
        Bus::new_with_model(cart, HardwareModel::Sgb)
    };

    // boot_div-S.gb checksum bytes at 0x014E/0x014F.
    let bus_a = make_bus(0x34, 0x12);
    assert_eq!(bus_a.timer.div_counter, 0xD860);

    // boot_div2-S.gb checksum bytes at 0x014E/0x014F.
    let bus_b = make_bus(0x96, 0xA7);
    assert_eq!(bus_b.timer.div_counter, 0xD850);
}
