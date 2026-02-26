use super::bus_access::{AddressSegment, SegmentAccess, address_segment};
use super::dma::{
    CgbDmaMmioRegister, DmaCpuAccessDecision, DmaCpuAccessKind, DmaSchedulerMode,
    cgb_dma_mmio_register,
};
use super::mmio::{CgbMmioRegister, cgb_mmio_register};
use super::test_support::{make_test_bus, make_test_bus_with_model, tick_n};
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

mod boot_profiles;
mod bus_mapping;
mod cartridge_api;
mod cgb_scaffolds;
mod dma;
mod joypad;
mod serial;
mod timing;
