use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

mod bus;
mod channels;
mod core;
mod mix;
mod sequencer;
use channels::{NoiseChannel, SquareChannel, WaveChannel};

const NR10_INDEX: usize = 0x10;
const NR11_INDEX: usize = 0x11;
const NR12_INDEX: usize = 0x12;
const NR13_INDEX: usize = 0x13;
const NR14_INDEX: usize = 0x14;
const NR21_INDEX: usize = 0x16;
const NR22_INDEX: usize = 0x17;
const NR23_INDEX: usize = 0x18;
const NR24_INDEX: usize = 0x19;
const NR30_INDEX: usize = 0x1A;
const NR31_INDEX: usize = 0x1B;
const NR32_INDEX: usize = 0x1C;
const NR33_INDEX: usize = 0x1D;
const NR34_INDEX: usize = 0x1E;
const NR41_INDEX: usize = 0x20;
const NR42_INDEX: usize = 0x21;
const NR43_INDEX: usize = 0x22;
const NR44_INDEX: usize = 0x23;
const NR50_INDEX: usize = 0x24;
const NR51_INDEX: usize = 0x25;
const NR52_INDEX: usize = 0x26;
const WAVE_RAM_START_INDEX: usize = 0x30;
const WAVE_RAM_END_INDEX: usize = 0x3F;
const MAX_PENDING_AUDIO_TCYCLE_FRAMES: usize = 262_144;

const DIV_APU_BIT: u16 = 1 << 12;
const CHANNEL_COUNT: usize = 4;
const MAX_SQUARE_LENGTH: u8 = 64;
const MAX_NOISE_LENGTH: u8 = 64;
const MAX_WAVE_LENGTH: u16 = 256;
const MAX_FREQUENCY: u16 = 2_047;

const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

const NOISE_DIVISORS: [u16; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

pub(super) struct ApuState {
    analog_profile: AnalogCalibrationProfile,
    enabled: bool,
    channel_on_mask: u8,
    frame_sequencer_step: u8,
    frame_sequencer_ticks: u64,
    length_tick_count: u64,
    sweep_tick_count: u64,
    envelope_tick_count: u64,
    square1: SquareChannel,
    square2: SquareChannel,
    wave: WaveChannel,
    noise: NoiseChannel,
    last_mixed_sample_left: f32,
    last_mixed_sample_right: f32,
    last_mixed_sample: f32,
    lpf_output_prev_left: f32,
    lpf_output_prev_right: f32,
    hpf_input_prev_left: f32,
    hpf_output_prev_left: f32,
    hpf_input_prev_right: f32,
    hpf_output_prev_right: f32,
    pending_tcycle_samples: Vec<f32>,
    capture_tcycle_stream: bool,
}

impl Default for ApuState {
    fn default() -> Self {
        Self {
            analog_profile: AnalogCalibrationProfile::for_model(HardwareModel::Dmg).normalized(),
            enabled: false,
            channel_on_mask: 0,
            frame_sequencer_step: 0,
            frame_sequencer_ticks: 0,
            length_tick_count: 0,
            sweep_tick_count: 0,
            envelope_tick_count: 0,
            square1: SquareChannel::with_sweep(),
            square2: SquareChannel::default(),
            wave: WaveChannel::default(),
            noise: NoiseChannel::default(),
            last_mixed_sample_left: 0.0,
            last_mixed_sample_right: 0.0,
            last_mixed_sample: 0.0,
            lpf_output_prev_left: 0.0,
            lpf_output_prev_right: 0.0,
            hpf_input_prev_left: 0.0,
            hpf_output_prev_left: 0.0,
            hpf_input_prev_right: 0.0,
            hpf_output_prev_right: 0.0,
            pending_tcycle_samples: Vec::new(),
            capture_tcycle_stream: false,
        }
    }
}
