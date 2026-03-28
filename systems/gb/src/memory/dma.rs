use super::Bus;
use super::bus_access::{AddressSegment, SegmentAccess};
use crate::hardware::HardwareModel;

mod arbiter;
mod cgb_scaffold;
mod oam;
mod state;

pub(super) use self::state::DmaState;
pub(in crate::memory) use self::state::{
    CgbDmaMmioRegister, DmaCpuAccessDecision, DmaCpuAccessKind, DmaSchedulerEdgeEvents,
    DmaSchedulerMode, cgb_dma_mmio_register,
};
use self::state::{
    OAM_DMA_BYTE_PERIOD_T_CYCLES, OAM_DMA_BYTES, OAM_DMA_START_DELAY_T_CYCLES,
    OAM_DMA_TRANSFER_T_CYCLES,
};

impl DmaState {
    pub(super) fn configure_model_gates(bus: &mut Bus, model: HardwareModel) {
        bus.dma.cgb_scaffold.runtime_enabled = Self::model_supports_cgb_dma(model);
    }

    pub(super) fn write_register(bus: &mut Bus, source_high: u8) {
        bus.io[0x46] = source_high;
        Self::request_oam_transfer(bus, source_high);
    }

    fn tick_once(bus: &mut Bus) {
        Self::reset_mode_edge_events(bus);

        // Fast path for the overwhelmingly common DMG case in debug builds:
        // no OAM DMA active/pending and no CGB DMA scaffold runtime enabled.
        if matches!(bus.dma.mode, DmaSchedulerMode::Idle)
            && bus.dma.oam.start_delay_tcycles == 0
            && bus.dma.oam.pending_source.is_none()
            && !bus.dma.cgb_scaffold.runtime_enabled
        {
            return;
        }

        let start_oam_now = Self::step_oam_start_delay(bus);
        Self::step_active_transfer_tcycle(bus);

        if start_oam_now {
            // Start (or restart) OAM DMA. On restart, the previous transfer
            // keeps running through the full restart delay, then the new
            // transfer takes over on the following scheduler tick.
            Self::start_or_restart_oam_transfer(bus);
        }

        Self::start_pending_cgb_dma_transfer(bus);
    }

    fn step_active_transfer_tcycle(bus: &mut Bus) {
        match bus.dma.mode {
            DmaSchedulerMode::Idle => {}
            DmaSchedulerMode::Oam => Self::step_oam_transfer_tcycle(bus),
            DmaSchedulerMode::Gdma => Self::step_gdma_transfer_scaffold(bus),
            DmaSchedulerMode::Hdma => Self::step_hdma_transfer_scaffold(bus),
        }
    }

    fn active_oam_dma_word(bus: &Bus) -> Option<(u8, u8)> {
        if !matches!(bus.dma.mode, DmaSchedulerMode::Oam) || bus.dma.oam.index >= OAM_DMA_BYTES {
            return None;
        }

        let word_index = bus.dma.oam.index & !0x01;
        let source = bus.dma.oam.source.wrapping_add(word_index as u16);
        Some((
            bus.read_byte_raw(source),
            bus.read_byte_raw(source.wrapping_add(1)),
        ))
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
        match prev {
            DmaSchedulerMode::Idle => {}
            DmaSchedulerMode::Oam => bus.dma.mode_edge_events.exited_oam = true,
            DmaSchedulerMode::Gdma => bus.dma.mode_edge_events.exited_gdma = true,
            DmaSchedulerMode::Hdma => bus.dma.mode_edge_events.exited_hdma = true,
        }
        match next {
            DmaSchedulerMode::Idle => {}
            DmaSchedulerMode::Oam => bus.dma.mode_edge_events.entered_oam = true,
            DmaSchedulerMode::Gdma => bus.dma.mode_edge_events.entered_gdma = true,
            DmaSchedulerMode::Hdma => bus.dma.mode_edge_events.entered_hdma = true,
        }
    }
}

impl Bus {
    pub(super) fn write_dma(&mut self, source_high: u8) {
        DmaState::write_register(self, source_high);
    }

    pub(super) fn tick_dma_scheduler_tcycle(&mut self) {
        DmaState::tick_once(self);
    }

    pub(super) fn configure_dma_model_gates(&mut self, model: HardwareModel) {
        DmaState::configure_model_gates(self, model);
    }

    pub(super) fn read_cgb_dma_mmio_scaffold(&self, addr: u16) -> Option<u8> {
        DmaState::read_cgb_dma_mmio_scaffold(self, addr)
    }

    pub(super) fn write_cgb_dma_mmio_scaffold(&mut self, addr: u16, value: u8) -> bool {
        DmaState::write_cgb_dma_mmio_scaffold(self, addr, value)
    }

    pub(super) fn dma_blocks_oam_cpu_read(&self) -> bool {
        DmaState::cpu_blocks_oam_read(self)
    }

    pub(super) fn dma_blocks_oam_cpu_write(&self) -> bool {
        DmaState::cpu_blocks_oam_write(self)
    }

    pub(in crate::memory) fn active_oam_dma_word(&self) -> Option<(u8, u8)> {
        DmaState::active_oam_dma_word(self)
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

    #[cfg(test)]
    pub(super) fn debug_cgb_dma_scaffold_shadows(&self) -> (u8, u8, u8, u8, u8) {
        (
            self.dma.cgb_scaffold.hdma1_shadow,
            self.dma.cgb_scaffold.hdma2_shadow,
            self.dma.cgb_scaffold.hdma3_shadow,
            self.dma.cgb_scaffold.hdma4_shadow,
            self.dma.cgb_scaffold.hdma5_shadow,
        )
    }

    #[cfg(test)]
    pub(super) fn debug_cgb_dma_scaffold_last_requested_mode(&self) -> Option<DmaSchedulerMode> {
        self.dma.cgb_scaffold.last_requested_mode
    }

    #[cfg(test)]
    pub(super) fn debug_force_enable_cgb_dma_scaffold_runtime(&mut self, enabled: bool) {
        self.dma.cgb_scaffold.runtime_enabled = enabled;
    }

    #[cfg(test)]
    pub(super) fn debug_cgb_dma_scaffold_runtime_enabled(&self) -> bool {
        self.dma.cgb_scaffold.runtime_enabled
    }

    #[cfg(test)]
    pub(super) fn debug_cgb_dma_transfer_scaffold_state(&self) -> (u16, u16, u8) {
        (
            self.dma.cgb_scaffold.transfer_source,
            self.dma.cgb_scaffold.transfer_dest,
            self.dma.cgb_scaffold.transfer_blocks_remaining,
        )
    }
}
