use super::ApuState;

#[derive(Clone, Copy)]
pub(crate) struct ApuTestDebugState {
    pub(crate) frame_sequencer_step: u8,
    pub(crate) frame_sequencer_ticks: u64,
    pub(crate) length_tick_count: u64,
    pub(crate) sweep_tick_count: u64,
    pub(crate) envelope_tick_count: u64,
    pub(crate) last_mixed_sample: f32,
    pub(crate) last_mixed_sample_left: f32,
    pub(crate) last_mixed_sample_right: f32,
    pub(crate) square2_envelope_volume: u8,
    pub(crate) square2_envelope_timer: u8,
    pub(crate) square2_length_counter: u8,
    pub(crate) square1_frequency: u16,
    pub(crate) square1_enabled: bool,
    pub(crate) square2_enabled: bool,
    pub(crate) wave_position: u8,
    pub(crate) wave_sample_buffer: u8,
    pub(crate) noise_lfsr: u16,
    pub(crate) analog_hpf_coeff: f32,
    pub(crate) analog_low_pass_alpha: f32,
    pub(crate) analog_soft_clip_drive: f32,
}

impl ApuState {
    pub(crate) fn test_debug_state(&self) -> ApuTestDebugState {
        ApuTestDebugState {
            frame_sequencer_step: self.frame_sequencer_step,
            frame_sequencer_ticks: self.frame_sequencer_ticks,
            length_tick_count: self.length_tick_count,
            sweep_tick_count: self.sweep_tick_count,
            envelope_tick_count: self.envelope_tick_count,
            last_mixed_sample: self.last_mixed_sample,
            last_mixed_sample_left: self.last_mixed_sample_left,
            last_mixed_sample_right: self.last_mixed_sample_right,
            square2_envelope_volume: self.square2.envelope.volume,
            square2_envelope_timer: self.square2.envelope.timer,
            square2_length_counter: self.square2.length_counter,
            square1_frequency: self.square1.frequency,
            square1_enabled: self.square1.enabled,
            square2_enabled: self.square2.enabled,
            wave_position: self.wave.wave_position,
            wave_sample_buffer: self.wave.sample_buffer,
            noise_lfsr: self.noise.lfsr,
            analog_hpf_coeff: self.analog_profile.hpf_coeff,
            analog_low_pass_alpha: self.analog_profile.low_pass_alpha,
            analog_soft_clip_drive: self.analog_profile.soft_clip_drive,
        }
    }
}
