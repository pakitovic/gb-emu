use super::super::constants::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::apu) enum ApuRegister {
    Nr10,
    Nr11,
    Nr12,
    Nr13,
    Nr14,
    Nr21,
    Nr22,
    Nr23,
    Nr24,
    Nr30,
    Nr31,
    Nr32,
    Nr33,
    Nr34,
    Nr41,
    Nr42,
    Nr43,
    Nr44,
    Nr50,
    Nr51,
    Nr52,
    WaveRam(usize),
    Other(usize),
}

impl ApuRegister {
    fn from_addr(addr: u16) -> Option<Self> {
        let index = addr.checked_sub(0xFF00)? as usize;
        if index > 0x7F {
            return None;
        }

        Some(match index {
            NR10_INDEX => Self::Nr10,
            NR11_INDEX => Self::Nr11,
            NR12_INDEX => Self::Nr12,
            NR13_INDEX => Self::Nr13,
            NR14_INDEX => Self::Nr14,
            NR21_INDEX => Self::Nr21,
            NR22_INDEX => Self::Nr22,
            NR23_INDEX => Self::Nr23,
            NR24_INDEX => Self::Nr24,
            NR30_INDEX => Self::Nr30,
            NR31_INDEX => Self::Nr31,
            NR32_INDEX => Self::Nr32,
            NR33_INDEX => Self::Nr33,
            NR34_INDEX => Self::Nr34,
            NR41_INDEX => Self::Nr41,
            NR42_INDEX => Self::Nr42,
            NR43_INDEX => Self::Nr43,
            NR44_INDEX => Self::Nr44,
            NR50_INDEX => Self::Nr50,
            NR51_INDEX => Self::Nr51,
            NR52_INDEX => Self::Nr52,
            WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX => Self::WaveRam(index),
            _ => Self::Other(index),
        })
    }

    pub(in crate::apu) fn io_index(self) -> usize {
        match self {
            Self::Nr10 => NR10_INDEX,
            Self::Nr11 => NR11_INDEX,
            Self::Nr12 => NR12_INDEX,
            Self::Nr13 => NR13_INDEX,
            Self::Nr14 => NR14_INDEX,
            Self::Nr21 => NR21_INDEX,
            Self::Nr22 => NR22_INDEX,
            Self::Nr23 => NR23_INDEX,
            Self::Nr24 => NR24_INDEX,
            Self::Nr30 => NR30_INDEX,
            Self::Nr31 => NR31_INDEX,
            Self::Nr32 => NR32_INDEX,
            Self::Nr33 => NR33_INDEX,
            Self::Nr34 => NR34_INDEX,
            Self::Nr41 => NR41_INDEX,
            Self::Nr42 => NR42_INDEX,
            Self::Nr43 => NR43_INDEX,
            Self::Nr44 => NR44_INDEX,
            Self::Nr50 => NR50_INDEX,
            Self::Nr51 => NR51_INDEX,
            Self::Nr52 => NR52_INDEX,
            Self::WaveRam(index) | Self::Other(index) => index,
        }
    }
}

pub(in crate::apu) fn decode_register(addr: u16) -> Option<ApuRegister> {
    ApuRegister::from_addr(addr)
}
