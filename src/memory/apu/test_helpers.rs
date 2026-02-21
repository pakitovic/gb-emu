use super::super::Bus;

impl Bus {
    #[cfg(test)]
    pub(in crate::memory) fn apu_frame_sequencer_step(&self) -> u8 {
        self.apu.test_frame_sequencer_step()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_frame_sequencer_ticks(&self) -> u64 {
        self.apu.test_frame_sequencer_ticks()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_length_tick_count(&self) -> u64 {
        self.apu.test_length_tick_count()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_sweep_tick_count(&self) -> u64 {
        self.apu.test_sweep_tick_count()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_envelope_tick_count(&self) -> u64 {
        self.apu.test_envelope_tick_count()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_last_mixed_sample(&self) -> f32 {
        self.apu.test_last_mixed_sample()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_last_mixed_sample_stereo(&self) -> (f32, f32) {
        self.apu.test_last_mixed_sample_stereo()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_envelope_volume(&self) -> u8 {
        self.apu.test_square2_envelope_volume()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_envelope_timer(&self) -> u8 {
        self.apu.test_square2_envelope_timer()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_length_counter(&self) -> u8 {
        self.apu.test_square2_length_counter()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square1_frequency(&self) -> u16 {
        self.apu.test_square1_frequency()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square1_enabled(&self) -> bool {
        self.apu.test_square1_enabled()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_square2_enabled(&self) -> bool {
        self.apu.test_square2_enabled()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_wave_position(&self) -> u8 {
        self.apu.test_wave_position()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_wave_sample_buffer(&self) -> u8 {
        self.apu.test_wave_sample_buffer()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_noise_lfsr(&self) -> u16 {
        self.apu.test_noise_lfsr()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_analog_hpf_coeff(&self) -> f32 {
        self.apu.test_analog_hpf_coeff()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_analog_low_pass_alpha(&self) -> f32 {
        self.apu.test_analog_low_pass_alpha()
    }

    #[cfg(test)]
    pub(in crate::memory) fn apu_analog_soft_clip_drive(&self) -> f32 {
        self.apu.test_analog_soft_clip_drive()
    }
}
