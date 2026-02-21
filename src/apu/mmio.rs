use super::*;

mod decode;
mod power;
pub(super) use decode::{ApuRegister, decode_register};

impl ApuState {
    pub(super) fn read_io_register_mmio(&self, addr: u16) -> Option<u8> {
        match decode_register(addr)? {
            ApuRegister::Nr52 => Some(self.read_nr52()),
            _ => None,
        }
    }

    pub(super) fn write_io_register_mmio(&mut self, io: &mut [u8; 0x80], addr: u16, value: u8) {
        let Some(register) = decode_register(addr) else {
            return;
        };
        match register {
            ApuRegister::Nr50 => self.write_nr50(io, value),
            ApuRegister::Nr51 => self.write_nr51(io, value),
            ApuRegister::Nr52 => self.write_nr52_power(io, value),
            ApuRegister::WaveRam(index) => io[index] = value,
            _ => self.write_channel_register(io, register, value),
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

    fn write_channel_register(&mut self, io: &mut [u8; 0x80], register: ApuRegister, value: u8) {
        if !self.enabled {
            return;
        }

        io[register.io_index()] = value;
        let length_clocks_next = self.length_clocks_on_next_frame_step();
        let envelope_clocks_next = self.envelope_clocks_on_next_frame_step();
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
            ApuRegister::Nr21 => self.square2.write_duty_length(value),
            ApuRegister::Nr22 => self.square2.write_envelope(value),
            ApuRegister::Nr23 => self.square2.write_frequency_low(value),
            ApuRegister::Nr24 => {
                self.square2
                    .write_frequency_high(value, length_clocks_next, envelope_clocks_next)
            }
            ApuRegister::Nr30 => self.wave.write_dac_enable(value),
            ApuRegister::Nr31 => self.wave.write_length(value),
            ApuRegister::Nr32 => self.wave.write_output_level(value),
            ApuRegister::Nr33 => self.wave.write_frequency_low(value),
            ApuRegister::Nr34 => self.wave.write_frequency_high(value, length_clocks_next),
            ApuRegister::Nr41 => self.noise.write_length(value),
            ApuRegister::Nr42 => self.noise.write_envelope(value),
            ApuRegister::Nr43 => self.noise.write_polynomial(value),
            ApuRegister::Nr44 => {
                self.noise
                    .write_control(value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }
        self.refresh_channel_on_mask();
    }
}
