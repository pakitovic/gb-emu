use super::channels::{NoiseChannel, SquareChannel, WaveChannel};
use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

#[derive(Default)]
pub(in crate::apu) struct FrameSequencerState {
    pub(in crate::apu) step: u8,
    pub(in crate::apu) ticks: u64,
    pub(in crate::apu) length_tick_count: u64,
    pub(in crate::apu) sweep_tick_count: u64,
    pub(in crate::apu) envelope_tick_count: u64,
}

pub(in crate::apu) struct AnalogPathState {
    pub(in crate::apu) last_mixed_sample_left: f32,
    pub(in crate::apu) last_mixed_sample_right: f32,
    pub(in crate::apu) last_mixed_sample: f32,
    pub(in crate::apu) lpf_output_prev_left: f32,
    pub(in crate::apu) lpf_output_prev_right: f32,
    pub(in crate::apu) hpf_input_prev_left: f32,
    pub(in crate::apu) hpf_output_prev_left: f32,
    pub(in crate::apu) hpf_input_prev_right: f32,
    pub(in crate::apu) hpf_output_prev_right: f32,
}

impl Default for AnalogPathState {
    fn default() -> Self {
        Self {
            last_mixed_sample_left: 0.0,
            last_mixed_sample_right: 0.0,
            last_mixed_sample: 0.0,
            lpf_output_prev_left: 0.0,
            lpf_output_prev_right: 0.0,
            hpf_input_prev_left: 0.0,
            hpf_output_prev_left: 0.0,
            hpf_input_prev_right: 0.0,
            hpf_output_prev_right: 0.0,
        }
    }
}

#[derive(Default)]
pub(in crate::apu) struct StreamCaptureState {
    pub(in crate::apu) pending_tcycle_samples: Vec<f32>,
    pub(in crate::apu) capture_tcycle_stream: bool,
}

pub(crate) struct ApuState {
    pub(in crate::apu) analog_profile: AnalogCalibrationProfile,
    pub(in crate::apu) enabled: bool,
    pub(in crate::apu) channel_on_mask: u8,
    pub(in crate::apu) timing: FrameSequencerState,
    pub(in crate::apu) square1: SquareChannel,
    pub(in crate::apu) square2: SquareChannel,
    pub(in crate::apu) wave: WaveChannel,
    pub(in crate::apu) noise: NoiseChannel,
    pub(in crate::apu) analog: AnalogPathState,
    pub(in crate::apu) stream: StreamCaptureState,
}

impl Default for ApuState {
    fn default() -> Self {
        Self {
            analog_profile: AnalogCalibrationProfile::for_model(HardwareModel::Dmg).normalized(),
            enabled: false,
            channel_on_mask: 0,
            timing: FrameSequencerState::default(),
            square1: SquareChannel::with_sweep(),
            square2: SquareChannel::default(),
            wave: WaveChannel::default(),
            noise: NoiseChannel::default(),
            analog: AnalogPathState::default(),
            stream: StreamCaptureState::default(),
        }
    }
}
