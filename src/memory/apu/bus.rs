use super::super::Bus;
use super::*;

impl Bus {
    pub(in crate::memory) fn sync_apu_boot_state(&mut self, model: HardwareModel) {
        self.apu = ApuState::from_boot_registers(&self.io, model);
    }

    pub fn set_apu_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.apu.analog_profile = calibration.normalized();
        self.apu.reset_analog_filter_state();
    }

    pub(in crate::memory) fn read_nr52(&self) -> u8 {
        ((self.apu.enabled as u8) << 7) | (self.apu.channel_on_mask & 0x0F)
    }

    pub(in crate::memory) fn write_nr50(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR50_INDEX] = value;
    }

    pub(in crate::memory) fn write_nr51(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR51_INDEX] = value;
    }

    pub(in crate::memory) fn write_nr52(&mut self, value: u8) {
        let request_enabled = (value & 0x80) != 0;

        if self.apu.enabled && !request_enabled {
            self.clear_apu_registers();
            self.apu.reset_after_power_toggle(false);
            self.io[NR52_INDEX] = 0x00;
            return;
        }

        if !self.apu.enabled && request_enabled {
            self.clear_apu_registers();
            self.apu.reset_after_power_toggle(true);
            self.io[NR52_INDEX] = 0x80;
            return;
        }

        if self.apu.enabled {
            self.io[NR52_INDEX] = 0x80 | (self.apu.channel_on_mask & 0x0F);
        } else {
            self.io[NR52_INDEX] = 0x00;
        }
    }

    pub(in crate::memory) fn write_apu_register(&mut self, addr: u16, value: u8) {
        let index = (addr - 0xFF00) as usize;
        if (WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX).contains(&index) {
            self.io[index] = value;
            return;
        }
        if !self.apu.enabled {
            return;
        }

        self.io[index] = value;
        let length_clocks_next = self.apu.length_clocks_on_next_frame_step();
        let envelope_clocks_next = self.apu.envelope_clocks_on_next_frame_step();
        match index {
            NR10_INDEX => {
                if self.apu.square1.sweep.write_register(value) {
                    self.apu.square1.enabled = false;
                }
            }
            NR11_INDEX => self.apu.square1.write_duty_length(value),
            NR12_INDEX => self.apu.square1.write_envelope(value),
            NR13_INDEX => self.apu.square1.write_frequency_low(value),
            NR14_INDEX => self.apu.square1.write_frequency_high(
                value,
                length_clocks_next,
                envelope_clocks_next,
            ),
            NR21_INDEX => self.apu.square2.write_duty_length(value),
            NR22_INDEX => self.apu.square2.write_envelope(value),
            NR23_INDEX => self.apu.square2.write_frequency_low(value),
            NR24_INDEX => self.apu.square2.write_frequency_high(
                value,
                length_clocks_next,
                envelope_clocks_next,
            ),
            NR30_INDEX => self.apu.wave.write_dac_enable(value),
            NR31_INDEX => self.apu.wave.write_length(value),
            NR32_INDEX => self.apu.wave.write_output_level(value),
            NR33_INDEX => self.apu.wave.write_frequency_low(value),
            NR34_INDEX => self
                .apu
                .wave
                .write_frequency_high(value, length_clocks_next),
            NR41_INDEX => self.apu.noise.write_length(value),
            NR42_INDEX => self.apu.noise.write_envelope(value),
            NR43_INDEX => self.apu.noise.write_polynomial(value),
            NR44_INDEX => {
                self.apu
                    .noise
                    .write_control(value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }
        self.apu.refresh_channel_on_mask();
    }

    pub(in crate::memory) fn step_apu_frame_sequencer_from_divider(
        &mut self,
        old_div: u16,
        new_div: u16,
    ) {
        if !self.apu.enabled {
            return;
        }

        let old_high = (old_div & DIV_APU_BIT) != 0;
        let new_high = (new_div & DIV_APU_BIT) != 0;
        if old_high && !new_high {
            self.apu.clock_frame_sequencer();
        }
    }

    pub(in crate::memory) fn step_apu_tcycle(&mut self) {
        self.apu.step_tcycle(&self.io);
    }

    pub fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        if self.apu.pending_tcycle_samples.is_empty() {
            return Vec::new();
        }
        self.apu.pending_tcycle_samples.drain(..).collect()
    }

    pub fn set_audio_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.apu.capture_tcycle_stream = enabled;
        if !enabled {
            self.apu.pending_tcycle_samples.clear();
        }
    }

    fn clear_apu_registers(&mut self) {
        for index in NR10_INDEX..=NR51_INDEX {
            self.io[index] = 0x00;
        }
    }

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
