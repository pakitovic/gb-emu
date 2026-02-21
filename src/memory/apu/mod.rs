use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

mod bus;
mod channels;
mod constants;
mod core;
mod mix;
mod register_dispatch;
mod sequencer;
#[cfg(test)]
mod test_helpers;
#[cfg(test)]
mod tests;
use channels::{NoiseChannel, SquareChannel, WaveChannel};
pub(in crate::memory::apu) use constants::*;

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
