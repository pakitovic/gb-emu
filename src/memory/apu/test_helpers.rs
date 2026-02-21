use super::super::Bus;

impl Bus {
    #[cfg(test)]
    pub(in crate::memory) fn apu_frame_sequencer_step(&self) -> u8 {
        self.apu.frame_sequencer_step
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_frame_sequencer_ticks(&self) -> u64 {
        self.apu.frame_sequencer_ticks
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_length_tick_count(&self) -> u64 {
        self.apu.length_tick_count
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_sweep_tick_count(&self) -> u64 {
        self.apu.sweep_tick_count
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_envelope_tick_count(&self) -> u64 {
        self.apu.envelope_tick_count
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_last_mixed_sample(&self) -> f32 {
        self.apu.last_mixed_sample
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_last_mixed_sample_stereo(&self) -> (f32, f32) {
        (
            self.apu.last_mixed_sample_left,
            self.apu.last_mixed_sample_right,
        )
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_envelope_volume(&self) -> u8 {
        self.apu.square2.envelope.volume
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_envelope_timer(&self) -> u8 {
        self.apu.square2.envelope.timer
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_length_counter(&self) -> u8 {
        self.apu.square2.length_counter
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square1_frequency(&self) -> u16 {
        self.apu.square1.frequency
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square1_enabled(&self) -> bool {
        self.apu.square1.enabled
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_enabled(&self) -> bool {
        self.apu.square2.enabled
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_wave_position(&self) -> u8 {
        self.apu.wave.wave_position
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_wave_sample_buffer(&self) -> u8 {
        self.apu.wave.sample_buffer
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_noise_lfsr(&self) -> u16 {
        self.apu.noise.lfsr
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_analog_hpf_coeff(&self) -> f32 {
        self.apu.analog_profile.hpf_coeff
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_analog_low_pass_alpha(&self) -> f32 {
        self.apu.analog_profile.low_pass_alpha
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_analog_soft_clip_drive(&self) -> f32 {
        self.apu.analog_profile.soft_clip_drive
    }
}
