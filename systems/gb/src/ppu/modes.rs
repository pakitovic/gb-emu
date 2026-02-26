use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub(in crate::memory) enum PpuMode {
    #[default]
    HBlank = STAT_MODE_HBLANK,
    VBlank = STAT_MODE_VBLANK,
    Oam = STAT_MODE_OAM,
    Transfer = STAT_MODE_TRANSFER,
}

impl PpuMode {
    pub(super) fn from_stat_mode_bits(bits: u8) -> Self {
        match bits & 0x03 {
            STAT_MODE_HBLANK => Self::HBlank,
            STAT_MODE_VBLANK => Self::VBlank,
            STAT_MODE_OAM => Self::Oam,
            STAT_MODE_TRANSFER => Self::Transfer,
            _ => unreachable!(),
        }
    }

    pub(super) fn stat_mode_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::memory) struct PpuModeEdgeEvents {
    pub(in crate::memory) entered_hblank: bool,
    pub(in crate::memory) entered_vblank: bool,
    pub(in crate::memory) entered_oam: bool,
    pub(in crate::memory) entered_transfer: bool,
}

impl PpuModeEdgeEvents {
    pub(super) fn for_entered_mode(mode: PpuMode) -> Self {
        let mut events = Self::default();
        match mode {
            PpuMode::HBlank => events.entered_hblank = true,
            PpuMode::VBlank => events.entered_vblank = true,
            PpuMode::Oam => events.entered_oam = true,
            PpuMode::Transfer => events.entered_transfer = true,
        }
        events
    }
}
