use super::*;

impl ApuState {
    pub(crate) fn from_boot_state(io: &[u8; 0x80], model: HardwareModel) -> Self {
        Self::from_boot_registers(io, model)
    }

    pub(crate) fn set_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.analog_profile = calibration.normalized();
        self.reset_analog_filter_state();
    }

    pub(crate) fn read_nr52(&self) -> u8 {
        ((self.enabled as u8) << 7) | (self.channel_on_mask & 0x0F)
    }

    pub(crate) fn write_nr50(&mut self, io: &mut [u8; 0x80], value: u8) {
        if !self.enabled {
            return;
        }
        io[NR50_INDEX] = value;
    }

    pub(crate) fn write_nr51(&mut self, io: &mut [u8; 0x80], value: u8) {
        if !self.enabled {
            return;
        }
        io[NR51_INDEX] = value;
    }

    pub(crate) fn write_nr52(&mut self, io: &mut [u8; 0x80], value: u8) {
        let request_enabled = (value & 0x80) != 0;

        if self.enabled && !request_enabled {
            Self::clear_registers(io);
            self.reset_after_power_toggle(false);
            io[NR52_INDEX] = 0x00;
            return;
        }

        if !self.enabled && request_enabled {
            Self::clear_registers(io);
            self.reset_after_power_toggle(true);
            io[NR52_INDEX] = 0x80;
            return;
        }

        if self.enabled {
            io[NR52_INDEX] = 0x80 | (self.channel_on_mask & 0x0F);
        } else {
            io[NR52_INDEX] = 0x00;
        }
    }

    pub(crate) fn step_frame_sequencer_from_divider(&mut self, old_div: u16, new_div: u16) {
        if !self.enabled {
            return;
        }

        let old_high = (old_div & DIV_APU_BIT) != 0;
        let new_high = (new_div & DIV_APU_BIT) != 0;
        if old_high && !new_high {
            self.clock_frame_sequencer();
        }
    }

    pub(crate) fn step_tcycle_with_io(&mut self, io: &[u8; 0x80]) {
        self.step_tcycle(io);
    }

    pub(crate) fn drain_tcycle_samples(&mut self) -> Vec<f32> {
        if self.pending_tcycle_samples.is_empty() {
            return Vec::new();
        }
        self.pending_tcycle_samples.drain(..).collect()
    }

    pub(crate) fn set_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.capture_tcycle_stream = enabled;
        if !enabled {
            self.pending_tcycle_samples.clear();
        }
    }

    pub(crate) fn write_register(&mut self, io: &mut [u8; 0x80], addr: u16, value: u8) {
        let index = (addr - 0xFF00) as usize;
        if (WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX).contains(&index) {
            io[index] = value;
            return;
        }
        if !self.enabled {
            return;
        }

        io[index] = value;
        let length_clocks_next = self.length_clocks_on_next_frame_step();
        let envelope_clocks_next = self.envelope_clocks_on_next_frame_step();
        match index {
            NR10_INDEX => {
                if self.square1.sweep.write_register(value) {
                    self.square1.enabled = false;
                }
            }
            NR11_INDEX => self.square1.write_duty_length(value),
            NR12_INDEX => self.square1.write_envelope(value),
            NR13_INDEX => self.square1.write_frequency_low(value),
            NR14_INDEX => {
                self.square1
                    .write_frequency_high(value, length_clocks_next, envelope_clocks_next)
            }
            NR21_INDEX => self.square2.write_duty_length(value),
            NR22_INDEX => self.square2.write_envelope(value),
            NR23_INDEX => self.square2.write_frequency_low(value),
            NR24_INDEX => {
                self.square2
                    .write_frequency_high(value, length_clocks_next, envelope_clocks_next)
            }
            NR30_INDEX => self.wave.write_dac_enable(value),
            NR31_INDEX => self.wave.write_length(value),
            NR32_INDEX => self.wave.write_output_level(value),
            NR33_INDEX => self.wave.write_frequency_low(value),
            NR34_INDEX => self.wave.write_frequency_high(value, length_clocks_next),
            NR41_INDEX => self.noise.write_length(value),
            NR42_INDEX => self.noise.write_envelope(value),
            NR43_INDEX => self.noise.write_polynomial(value),
            NR44_INDEX => self
                .noise
                .write_control(value, length_clocks_next, envelope_clocks_next),
            _ => {}
        }
        self.refresh_channel_on_mask();
    }

    fn clear_registers(io: &mut [u8; 0x80]) {
        for register in io.iter_mut().take(NR51_INDEX + 1).skip(NR10_INDEX) {
            *register = 0x00;
        }
    }

    #[cfg(test)]
    pub(crate) fn test_frame_sequencer_step(&self) -> u8 {
        self.frame_sequencer_step
    }

    #[cfg(test)]
    pub(crate) fn test_frame_sequencer_ticks(&self) -> u64 {
        self.frame_sequencer_ticks
    }

    #[cfg(test)]
    pub(crate) fn test_length_tick_count(&self) -> u64 {
        self.length_tick_count
    }

    #[cfg(test)]
    pub(crate) fn test_sweep_tick_count(&self) -> u64 {
        self.sweep_tick_count
    }

    #[cfg(test)]
    pub(crate) fn test_envelope_tick_count(&self) -> u64 {
        self.envelope_tick_count
    }

    #[cfg(test)]
    pub(crate) fn test_last_mixed_sample(&self) -> f32 {
        self.last_mixed_sample
    }

    #[cfg(test)]
    pub(crate) fn test_last_mixed_sample_stereo(&self) -> (f32, f32) {
        (self.last_mixed_sample_left, self.last_mixed_sample_right)
    }

    #[cfg(test)]
    pub(crate) fn test_square2_envelope_volume(&self) -> u8 {
        self.square2.envelope.volume
    }

    #[cfg(test)]
    pub(crate) fn test_square2_envelope_timer(&self) -> u8 {
        self.square2.envelope.timer
    }

    #[cfg(test)]
    pub(crate) fn test_square2_length_counter(&self) -> u8 {
        self.square2.length_counter
    }

    #[cfg(test)]
    pub(crate) fn test_square1_frequency(&self) -> u16 {
        self.square1.frequency
    }

    #[cfg(test)]
    pub(crate) fn test_square1_enabled(&self) -> bool {
        self.square1.enabled
    }

    #[cfg(test)]
    pub(crate) fn test_square2_enabled(&self) -> bool {
        self.square2.enabled
    }

    #[cfg(test)]
    pub(crate) fn test_wave_position(&self) -> u8 {
        self.wave.wave_position
    }

    #[cfg(test)]
    pub(crate) fn test_wave_sample_buffer(&self) -> u8 {
        self.wave.sample_buffer
    }

    #[cfg(test)]
    pub(crate) fn test_noise_lfsr(&self) -> u16 {
        self.noise.lfsr
    }

    #[cfg(test)]
    pub(crate) fn test_analog_hpf_coeff(&self) -> f32 {
        self.analog_profile.hpf_coeff
    }

    #[cfg(test)]
    pub(crate) fn test_analog_low_pass_alpha(&self) -> f32 {
        self.analog_profile.low_pass_alpha
    }

    #[cfg(test)]
    pub(crate) fn test_analog_soft_clip_drive(&self) -> f32 {
        self.analog_profile.soft_clip_drive
    }
}
