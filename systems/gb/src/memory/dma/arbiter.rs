use super::*;

impl DmaState {
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
}
