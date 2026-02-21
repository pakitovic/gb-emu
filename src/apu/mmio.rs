use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ApuRegister {
    Nr10,
    Nr11,
    Nr12,
    Nr13,
    Nr14,
    Nr21,
    Nr22,
    Nr23,
    Nr24,
    Nr30,
    Nr31,
    Nr32,
    Nr33,
    Nr34,
    Nr41,
    Nr42,
    Nr43,
    Nr44,
    Nr50,
    Nr51,
    Nr52,
    WaveRam(usize),
    Other(usize),
}

impl ApuRegister {
    fn from_addr(addr: u16) -> Option<Self> {
        let index = addr.checked_sub(0xFF00)? as usize;
        if index > 0x7F {
            return None;
        }

        Some(match index {
            NR10_INDEX => Self::Nr10,
            NR11_INDEX => Self::Nr11,
            NR12_INDEX => Self::Nr12,
            NR13_INDEX => Self::Nr13,
            NR14_INDEX => Self::Nr14,
            NR21_INDEX => Self::Nr21,
            NR22_INDEX => Self::Nr22,
            NR23_INDEX => Self::Nr23,
            NR24_INDEX => Self::Nr24,
            NR30_INDEX => Self::Nr30,
            NR31_INDEX => Self::Nr31,
            NR32_INDEX => Self::Nr32,
            NR33_INDEX => Self::Nr33,
            NR34_INDEX => Self::Nr34,
            NR41_INDEX => Self::Nr41,
            NR42_INDEX => Self::Nr42,
            NR43_INDEX => Self::Nr43,
            NR44_INDEX => Self::Nr44,
            NR50_INDEX => Self::Nr50,
            NR51_INDEX => Self::Nr51,
            NR52_INDEX => Self::Nr52,
            WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX => Self::WaveRam(index),
            _ => Self::Other(index),
        })
    }

    fn io_index(self) -> usize {
        match self {
            Self::Nr10 => NR10_INDEX,
            Self::Nr11 => NR11_INDEX,
            Self::Nr12 => NR12_INDEX,
            Self::Nr13 => NR13_INDEX,
            Self::Nr14 => NR14_INDEX,
            Self::Nr21 => NR21_INDEX,
            Self::Nr22 => NR22_INDEX,
            Self::Nr23 => NR23_INDEX,
            Self::Nr24 => NR24_INDEX,
            Self::Nr30 => NR30_INDEX,
            Self::Nr31 => NR31_INDEX,
            Self::Nr32 => NR32_INDEX,
            Self::Nr33 => NR33_INDEX,
            Self::Nr34 => NR34_INDEX,
            Self::Nr41 => NR41_INDEX,
            Self::Nr42 => NR42_INDEX,
            Self::Nr43 => NR43_INDEX,
            Self::Nr44 => NR44_INDEX,
            Self::Nr50 => NR50_INDEX,
            Self::Nr51 => NR51_INDEX,
            Self::Nr52 => NR52_INDEX,
            Self::WaveRam(index) | Self::Other(index) => index,
        }
    }
}

pub(super) fn decode_register(addr: u16) -> Option<ApuRegister> {
    ApuRegister::from_addr(addr)
}

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
            ApuRegister::Nr52 => self.write_nr52(io, value),
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

    fn clear_registers(io: &mut [u8; 0x80]) {
        for register in io.iter_mut().take(NR51_INDEX + 1).skip(NR10_INDEX) {
            *register = 0x00;
        }
    }
}
