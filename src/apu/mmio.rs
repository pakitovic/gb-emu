use super::*;

mod channel_writes;
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
}
