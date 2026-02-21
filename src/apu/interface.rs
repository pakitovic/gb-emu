use super::*;

impl ApuState {
    pub(crate) fn from_boot_state(io: &[u8; 0x80], model: HardwareModel) -> Self {
        Self::from_boot_registers(io, model)
    }

    pub(crate) fn set_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.analog_profile = calibration.normalized();
        self.reset_analog_filter_state();
    }

    pub(crate) fn read_io_register(&self, addr: u16) -> Option<u8> {
        match addr {
            0xFF26 => Some(self.read_nr52()),
            _ => None,
        }
    }

    pub(crate) fn write_io_register(&mut self, io: &mut [u8; 0x80], addr: u16, value: u8) {
        match addr {
            0xFF24 => self.write_nr50(io, value),
            0xFF25 => self.write_nr51(io, value),
            0xFF26 => self.write_nr52(io, value),
            _ => self.write_channel_register(io, addr, value),
        }
    }

    fn read_nr52(&self) -> u8 {
        ((self.enabled as u8) << 7) | (self.channel_on_mask & 0x0F)
    }

    fn write_nr50(&mut self, io: &mut [u8; 0x80], value: u8) {
        if !self.enabled {
            return;
        }
        io[NR50_INDEX] = value;
    }

    fn write_nr51(&mut self, io: &mut [u8; 0x80], value: u8) {
        if !self.enabled {
            return;
        }
        io[NR51_INDEX] = value;
    }

    fn write_nr52(&mut self, io: &mut [u8; 0x80], value: u8) {
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

    fn write_channel_register(&mut self, io: &mut [u8; 0x80], addr: u16, value: u8) {
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
}
