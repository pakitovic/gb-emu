use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

mod channels;
mod constants;
mod core;
mod interface;
mod mix;
mod mmio;
mod sequencer;
#[cfg(test)]
mod tests;

use channels::{NoiseChannel, SquareChannel, WaveChannel};
use constants::*;

#[derive(Default)]
pub(in crate::apu) struct FrameSequencerState {
    step: u8,
    ticks: u64,
    length_tick_count: u64,
    sweep_tick_count: u64,
    envelope_tick_count: u64,
}

pub(in crate::apu) struct AnalogPathState {
    last_mixed_sample_left: f32,
    last_mixed_sample_right: f32,
    last_mixed_sample: f32,
    lpf_output_prev_left: f32,
    lpf_output_prev_right: f32,
    hpf_input_prev_left: f32,
    hpf_output_prev_left: f32,
    hpf_input_prev_right: f32,
    hpf_output_prev_right: f32,
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
    pending_tcycle_samples: Vec<f32>,
    capture_tcycle_stream: bool,
}

pub(crate) struct ApuState {
    analog_profile: AnalogCalibrationProfile,
    enabled: bool,
    channel_on_mask: u8,
    timing: FrameSequencerState,
    square1: SquareChannel,
    square2: SquareChannel,
    wave: WaveChannel,
    noise: NoiseChannel,
    analog: AnalogPathState,
    stream: StreamCaptureState,
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
