use super::Cpu;
use crate::memory::Bus;

impl Cpu {
    pub(super) fn tick_t(&mut self, bus: &mut Bus, tcycles: u8) {
        self.step_tcycles = self.step_tcycles.wrapping_add(tcycles);
        bus.tick(tcycles);
    }

    pub(super) fn read_byte(&mut self, bus: &mut Bus, addr: u16) -> u8 {
        let value = bus.read_byte(addr);
        self.tick_t(bus, 4);
        value
    }

    pub(super) fn write_byte(&mut self, bus: &mut Bus, addr: u16, value: u8) {
        bus.write_byte(addr, value);
        self.tick_t(bus, 4);
    }

    pub(super) fn read_word(&mut self, bus: &mut Bus, addr: u16) -> u16 {
        let low = self.read_byte(bus, addr) as u16;
        let high = self.read_byte(bus, addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    pub(super) fn fetch_d8(&mut self, bus: &mut Bus) -> u8 {
        let value = self.read_byte(bus, self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    pub(super) fn fetch_d16(&mut self, bus: &mut Bus) -> u16 {
        let low = self.fetch_d8(bus) as u16;
        let high = self.fetch_d8(bus) as u16;
        (high << 8) | low
    }

    pub(super) fn push_u16(&mut self, bus: &mut Bus, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, (value & 0xFF) as u8);
    }

    pub(super) fn pop_u16(&mut self, bus: &mut Bus) -> u16 {
        let low = self.read_byte(bus, self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let high = self.read_byte(bus, self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        (high << 8) | low
    }
}
