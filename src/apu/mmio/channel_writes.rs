use super::super::*;
use super::ApuRegister;

impl ApuState {
    pub(super) fn write_channel_register(
        &mut self,
        io: &mut [u8; 0x80],
        register: ApuRegister,
        value: u8,
    ) {
        if !self.enabled {
            return;
        }

        io[register.io_index()] = value;
        let length_clocks_next = self.timing.length_clocks_on_next_step();
        let envelope_clocks_next = self.timing.envelope_clocks_on_next_step();

        match register {
            ApuRegister::Nr10
            | ApuRegister::Nr11
            | ApuRegister::Nr12
            | ApuRegister::Nr13
            | ApuRegister::Nr14 => {
                self.write_square1_reg(register, value, length_clocks_next, envelope_clocks_next)
            }
            ApuRegister::Nr21 | ApuRegister::Nr22 | ApuRegister::Nr23 | ApuRegister::Nr24 => {
                self.write_square2_reg(register, value, length_clocks_next, envelope_clocks_next)
            }
            ApuRegister::Nr30
            | ApuRegister::Nr31
            | ApuRegister::Nr32
            | ApuRegister::Nr33
            | ApuRegister::Nr34 => self.write_wave_reg(register, value, length_clocks_next),
            ApuRegister::Nr41 | ApuRegister::Nr42 | ApuRegister::Nr43 | ApuRegister::Nr44 => {
                self.write_noise_reg(register, value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }

        self.refresh_channel_on_mask();
    }

    fn write_square1_reg(
        &mut self,
        register: ApuRegister,
        value: u8,
        length_clocks_next: bool,
        envelope_clocks_next: bool,
    ) {
        match register {
            ApuRegister::Nr10 => {
                if self.square1.sweep.write_register(value) {
                    self.square1.enabled = false;
                }
            }
            ApuRegister::Nr11 => self.square1.write_duty_length(value),
            ApuRegister::Nr12 => self.square1.write_envelope(value),
            ApuRegister::Nr13 => self.square1.write_frequency_low(value),
            ApuRegister::Nr14 => {
                self.square1
                    .write_frequency_high(value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }
    }

    fn write_square2_reg(
        &mut self,
        register: ApuRegister,
        value: u8,
        length_clocks_next: bool,
        envelope_clocks_next: bool,
    ) {
        match register {
            ApuRegister::Nr21 => self.square2.write_duty_length(value),
            ApuRegister::Nr22 => self.square2.write_envelope(value),
            ApuRegister::Nr23 => self.square2.write_frequency_low(value),
            ApuRegister::Nr24 => {
                self.square2
                    .write_frequency_high(value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }
    }

    fn write_wave_reg(&mut self, register: ApuRegister, value: u8, length_clocks_next: bool) {
        match register {
            ApuRegister::Nr30 => self.wave.write_dac_enable(value),
            ApuRegister::Nr31 => self.wave.write_length(value),
            ApuRegister::Nr32 => self.wave.write_output_level(value),
            ApuRegister::Nr33 => self.wave.write_frequency_low(value),
            ApuRegister::Nr34 => self.wave.write_frequency_high(value, length_clocks_next),
            _ => {}
        }
    }

    fn write_noise_reg(
        &mut self,
        register: ApuRegister,
        value: u8,
        length_clocks_next: bool,
        envelope_clocks_next: bool,
    ) {
        match register {
            ApuRegister::Nr41 => self.noise.write_length(value),
            ApuRegister::Nr42 => self.noise.write_envelope(value),
            ApuRegister::Nr43 => self.noise.write_polynomial(value),
            ApuRegister::Nr44 => {
                self.noise
                    .write_control(value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }
    }
}
