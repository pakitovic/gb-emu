use super::*;

mod channel_writes;
mod decode;
mod power;
pub(super) use decode::{ApuRegister, decode_register};

impl ApuState {
    pub(super) fn read_io_register_mmio(&self, addr: u16) -> Option<u8> {
        match decode_register(addr)? {
            ApuRegister::Nr52 => Some(self.read_nr52()),
            ApuRegister::WaveRam(index) => Some(self.read_wave_ram(index)),
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
            ApuRegister::WaveRam(index) => self.write_wave_ram(io, index, value),
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
        self.write_register_mirror(io, NR50_INDEX, value);
    }

    fn write_nr51(&mut self, io: &mut [u8; 0x80], value: u8) {
        if !self.enabled {
            return;
        }
        self.write_register_mirror(io, NR51_INDEX, value);
    }

    pub(super) fn write_register_mirror(&mut self, io: &mut [u8; 0x80], index: usize, value: u8) {
        self.registers.write_index(index, value);
        io[index] = value;
    }

    fn read_wave_ram(&self, index: usize) -> u8 {
        if self.wave.cpu_can_access_wave_ram_now() {
            self.registers.wave_ram_byte(index - WAVE_RAM_START_INDEX)
        } else {
            0xFF
        }
    }

    fn write_wave_ram(&mut self, io: &mut [u8; 0x80], index: usize, value: u8) {
        if !self.wave.cpu_can_access_wave_ram_now() {
            return;
        }
        self.write_register_mirror(io, index, value);
    }
}
