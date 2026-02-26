use super::Bus;
use super::bus_access::AddressSegment;

const OAM_DMA_START_DELAY_T_CYCLES: u8 = 8;
const OAM_DMA_BYTE_PERIOD_T_CYCLES: u8 = 4;
const OAM_DMA_BYTES: u8 = 0xA0;
const OAM_DMA_TRANSFER_T_CYCLES: u16 =
    (OAM_DMA_BYTES as u16) * (OAM_DMA_BYTE_PERIOD_T_CYCLES as u16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::memory) enum DmaSchedulerMode {
    #[default]
    Idle,
    Oam,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::memory) struct DmaSchedulerEdgeEvents {
    pub entered_oam: bool,
    pub exited_oam: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum DmaCpuAccessKind {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum DmaCpuAccessDecision {
    Allow,
    Block,
}

#[derive(Default)]
struct OamDmaSchedulerState {
    source: u16,
    pending_source: Option<u16>,
    transfer_tcycles_remaining: u16,
    start_delay_tcycles: u8,
    byte_phase_tcycles: u8,
    index: u8,
}

#[derive(Default)]
pub(super) struct DmaState {
    mode: DmaSchedulerMode,
    mode_edge_events: DmaSchedulerEdgeEvents,
    oam: OamDmaSchedulerState,
}

impl DmaState {
    pub(super) fn write_register(bus: &mut Bus, source_high: u8) {
        bus.io[0x46] = source_high;
        Self::request_oam_transfer(bus, source_high);
    }

    pub(super) fn tick(bus: &mut Bus, tcycles: u8) {
        for _ in 0..tcycles {
            Self::tick_once(bus);
        }
    }

    pub(in crate::memory) fn cpu_access_decision_for_segment(
        bus: &Bus,
        segment: AddressSegment,
        _kind: DmaCpuAccessKind,
    ) -> DmaCpuAccessDecision {
        match (bus.dma.mode, segment) {
            // DMG OAM DMA currently models only OAM CPU blocking. Keep the
            // policy centralized here so HDMA/GDMA can extend it later.
            (DmaSchedulerMode::Oam, AddressSegment::Oam) => DmaCpuAccessDecision::Block,
            _ => DmaCpuAccessDecision::Allow,
        }
    }

    pub(super) fn cpu_blocks_oam_read(bus: &Bus) -> bool {
        matches!(
            Self::cpu_access_decision_for_segment(bus, AddressSegment::Oam, DmaCpuAccessKind::Read),
            DmaCpuAccessDecision::Block
        )
    }

    pub(super) fn cpu_blocks_oam_write(bus: &Bus) -> bool {
        matches!(
            Self::cpu_access_decision_for_segment(
                bus,
                AddressSegment::Oam,
                DmaCpuAccessKind::Write
            ),
            DmaCpuAccessDecision::Block
        )
    }

    fn tick_once(bus: &mut Bus) {
        Self::reset_mode_edge_events(bus);

        let start_oam_now = Self::step_oam_start_delay(bus);
        Self::step_active_transfer_tcycle(bus);

        if start_oam_now {
            // Start (or restart) OAM DMA. On restart, the previous transfer
            // keeps running through the full restart delay, then the new
            // transfer takes over on the following scheduler tick.
            Self::start_or_restart_oam_transfer(bus);
        }
    }

    fn request_oam_transfer(bus: &mut Bus, source_high: u8) {
        bus.dma.oam.pending_source = Some(Self::normalize_oam_source(source_high));
        // M=0 write, M=1 idle, M=2 transfer starts (DMG/SGB behavior).
        bus.dma.oam.start_delay_tcycles = OAM_DMA_START_DELAY_T_CYCLES;
    }

    fn normalize_oam_source(source_high: u8) -> u16 {
        // On DMG/SGB hardware, FE/FF source pages are remapped to DE/DF.
        let normalized = match source_high {
            0xFE | 0xFF => source_high.wrapping_sub(0x20),
            _ => source_high,
        };
        (normalized as u16) << 8
    }

    fn step_oam_start_delay(bus: &mut Bus) -> bool {
        if bus.dma.oam.start_delay_tcycles == 0 {
            return false;
        }

        bus.dma.oam.start_delay_tcycles = bus.dma.oam.start_delay_tcycles.saturating_sub(1);
        bus.dma.oam.start_delay_tcycles == 0
    }

    fn step_active_transfer_tcycle(bus: &mut Bus) {
        if !matches!(bus.dma.mode, DmaSchedulerMode::Oam) {
            return;
        }

        debug_assert!(bus.dma.oam.transfer_tcycles_remaining > 0);

        bus.dma.oam.byte_phase_tcycles = bus.dma.oam.byte_phase_tcycles.wrapping_add(1);
        if bus.dma.oam.byte_phase_tcycles == OAM_DMA_BYTE_PERIOD_T_CYCLES {
            bus.dma.oam.byte_phase_tcycles = 0;
            if bus.dma.oam.index < OAM_DMA_BYTES {
                let src = bus.dma.oam.source.wrapping_add(bus.dma.oam.index as u16);
                let value = bus.read_byte_raw(src);
                bus.write_oam_index_internal(bus.dma.oam.index as usize, value);
                bus.dma.oam.index = bus.dma.oam.index.wrapping_add(1);
            }
        }

        bus.dma.oam.transfer_tcycles_remaining =
            bus.dma.oam.transfer_tcycles_remaining.saturating_sub(1);
        if bus.dma.oam.transfer_tcycles_remaining == 0 {
            Self::set_mode(bus, DmaSchedulerMode::Idle);
        }
    }

    fn start_or_restart_oam_transfer(bus: &mut Bus) {
        let Some(source) = bus.dma.oam.pending_source.take() else {
            return;
        };

        bus.dma.oam.source = source;
        bus.dma.oam.transfer_tcycles_remaining = OAM_DMA_TRANSFER_T_CYCLES;
        bus.dma.oam.byte_phase_tcycles = 0;
        bus.dma.oam.index = 0;
        Self::set_mode(bus, DmaSchedulerMode::Oam);
    }

    fn reset_mode_edge_events(bus: &mut Bus) {
        bus.dma.mode_edge_events = DmaSchedulerEdgeEvents::default();
    }

    fn set_mode(bus: &mut Bus, next: DmaSchedulerMode) {
        let prev = bus.dma.mode;
        if prev == next {
            return;
        }

        bus.dma.mode = next;
        if matches!(prev, DmaSchedulerMode::Oam) {
            bus.dma.mode_edge_events.exited_oam = true;
        }
        if matches!(next, DmaSchedulerMode::Oam) {
            bus.dma.mode_edge_events.entered_oam = true;
        }
    }
}

impl Bus {
    pub(super) fn write_dma(&mut self, source_high: u8) {
        DmaState::write_register(self, source_high);
    }

    pub(super) fn tick_dma_scheduler(&mut self, tcycles: u8) {
        DmaState::tick(self, tcycles);
    }

    pub(super) fn dma_blocks_oam_cpu_read(&self) -> bool {
        DmaState::cpu_blocks_oam_read(self)
    }

    pub(super) fn dma_blocks_oam_cpu_write(&self) -> bool {
        DmaState::cpu_blocks_oam_write(self)
    }

    #[cfg(test)]
    pub(in crate::memory) fn dma_cpu_access_decision_for_segment(
        &self,
        segment: AddressSegment,
        kind: DmaCpuAccessKind,
    ) -> DmaCpuAccessDecision {
        DmaState::cpu_access_decision_for_segment(self, segment, kind)
    }

    #[cfg(test)]
    pub(super) fn debug_dma_mode_kind(&self) -> DmaSchedulerMode {
        self.dma.mode
    }

    #[cfg(test)]
    pub(super) fn debug_dma_mode_edge_events(&self) -> DmaSchedulerEdgeEvents {
        self.dma.mode_edge_events
    }
}
