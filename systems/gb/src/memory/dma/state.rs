pub(super) const OAM_DMA_START_DELAY_T_CYCLES: u8 = 8;
pub(super) const OAM_DMA_BYTE_PERIOD_T_CYCLES: u8 = 4;
pub(super) const OAM_DMA_BYTES: u8 = 0xA0;
pub(super) const OAM_DMA_TRANSFER_T_CYCLES: u16 =
    (OAM_DMA_BYTES as u16) * (OAM_DMA_BYTE_PERIOD_T_CYCLES as u16);
const REG_HDMA1: u16 = 0xFF51;
const REG_HDMA2: u16 = 0xFF52;
const REG_HDMA3: u16 = 0xFF53;
const REG_HDMA4: u16 = 0xFF54;
const REG_HDMA5: u16 = 0xFF55;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::memory) enum DmaSchedulerMode {
    #[default]
    Idle,
    Oam,
    Gdma,
    Hdma,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::memory) struct DmaSchedulerEdgeEvents {
    pub entered_oam: bool,
    pub exited_oam: bool,
    pub entered_gdma: bool,
    pub exited_gdma: bool,
    pub entered_hdma: bool,
    pub exited_hdma: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum CgbDmaMmioRegister {
    Hdma1,
    Hdma2,
    Hdma3,
    Hdma4,
    Hdma5,
}

#[derive(Default)]
pub(super) struct CgbDmaScaffoldState {
    pub(super) runtime_enabled: bool,
    pub(super) hdma1_shadow: u8,
    pub(super) hdma2_shadow: u8,
    pub(super) hdma3_shadow: u8,
    pub(super) hdma4_shadow: u8,
    pub(super) hdma5_shadow: u8,
    pub(super) last_requested_mode: Option<DmaSchedulerMode>,
    pub(super) pending_request_mode: Option<DmaSchedulerMode>,
    pub(super) transfer_source: u16,
    pub(super) transfer_dest: u16,
    pub(super) transfer_blocks_remaining: u8,
}

#[derive(Default)]
pub(super) struct OamDmaSchedulerState {
    pub(super) source: u16,
    pub(super) pending_source: Option<u16>,
    pub(super) transfer_tcycles_remaining: u16,
    pub(super) start_delay_tcycles: u8,
    pub(super) byte_phase_tcycles: u8,
    pub(super) index: u8,
}

#[derive(Default)]
pub(in crate::memory) struct DmaState {
    pub(super) mode: DmaSchedulerMode,
    pub(super) mode_edge_events: DmaSchedulerEdgeEvents,
    pub(super) oam: OamDmaSchedulerState,
    pub(super) cgb_scaffold: CgbDmaScaffoldState,
}

pub(in crate::memory) fn cgb_dma_mmio_register(addr: u16) -> Option<CgbDmaMmioRegister> {
    match addr {
        REG_HDMA1 => Some(CgbDmaMmioRegister::Hdma1),
        REG_HDMA2 => Some(CgbDmaMmioRegister::Hdma2),
        REG_HDMA3 => Some(CgbDmaMmioRegister::Hdma3),
        REG_HDMA4 => Some(CgbDmaMmioRegister::Hdma4),
        REG_HDMA5 => Some(CgbDmaMmioRegister::Hdma5),
        _ => None,
    }
}
