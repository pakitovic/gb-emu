pub(in crate::apu) const NR10_INDEX: usize = 0x10;
pub(in crate::apu) const NR11_INDEX: usize = 0x11;
pub(in crate::apu) const NR12_INDEX: usize = 0x12;
pub(in crate::apu) const NR13_INDEX: usize = 0x13;
pub(in crate::apu) const NR14_INDEX: usize = 0x14;
pub(in crate::apu) const NR21_INDEX: usize = 0x16;
pub(in crate::apu) const NR22_INDEX: usize = 0x17;
pub(in crate::apu) const NR23_INDEX: usize = 0x18;
pub(in crate::apu) const NR24_INDEX: usize = 0x19;
pub(in crate::apu) const NR30_INDEX: usize = 0x1A;
pub(in crate::apu) const NR31_INDEX: usize = 0x1B;
pub(in crate::apu) const NR32_INDEX: usize = 0x1C;
pub(in crate::apu) const NR33_INDEX: usize = 0x1D;
pub(in crate::apu) const NR34_INDEX: usize = 0x1E;
pub(in crate::apu) const NR41_INDEX: usize = 0x20;
pub(in crate::apu) const NR42_INDEX: usize = 0x21;
pub(in crate::apu) const NR43_INDEX: usize = 0x22;
pub(in crate::apu) const NR44_INDEX: usize = 0x23;
pub(in crate::apu) const NR50_INDEX: usize = 0x24;
pub(in crate::apu) const NR51_INDEX: usize = 0x25;
pub(in crate::apu) const NR52_INDEX: usize = 0x26;
pub(in crate::apu) const WAVE_RAM_START_INDEX: usize = 0x30;
pub(in crate::apu) const WAVE_RAM_END_INDEX: usize = 0x3F;

pub(in crate::apu) const MAX_PENDING_AUDIO_TCYCLE_FRAMES: usize = 262_144;

pub(in crate::apu) const DIV_APU_BIT: u16 = 1 << 12;
pub(in crate::apu) const CHANNEL_COUNT: usize = 4;
pub(in crate::apu) const MAX_SQUARE_LENGTH: u8 = 64;
pub(in crate::apu) const MAX_NOISE_LENGTH: u8 = 64;
pub(in crate::apu) const MAX_WAVE_LENGTH: u16 = 256;
pub(in crate::apu) const MAX_FREQUENCY: u16 = 2_047;

pub(in crate::apu) const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

pub(in crate::apu) const NOISE_DIVISORS: [u16; 8] = [8, 16, 32, 48, 64, 80, 96, 112];
