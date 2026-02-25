use crate::cpu::Cpu;
use crate::cpu::CpuContext;

impl Cpu {
    pub(in crate::cpu) const HALT_IDLE_STEP_M_CYCLES: u8 = 1;
    pub(in crate::cpu) const INTERRUPT_SERVICE_M_CYCLES: u8 = 5;

    pub(in crate::cpu) fn ret_m(&self, bus: &impl CpuContext, mcycles: u8) -> u8 {
        bus.cpu_tcycles_for_mcycles(mcycles)
    }

    pub(in crate::cpu) fn ret_step_total_m(&self, bus: &impl CpuContext, total_mcycles: u8) -> u8 {
        let expected_tcycles = self.ret_m(bus, total_mcycles);
        debug_assert_eq!(self.step_tcycles, expected_tcycles);
        expected_tcycles
    }

    pub(in crate::cpu) fn tick_and_ret_m(&mut self, bus: &mut impl CpuContext, mcycles: u8) -> u8 {
        self.tick_m(bus, mcycles);
        self.ret_step_total_m(bus, mcycles)
    }

    pub(in crate::cpu) fn tick_t(&mut self, bus: &mut impl CpuContext, tcycles: u8) {
        self.step_tcycles = self.step_tcycles.wrapping_add(tcycles);
        bus.tick(tcycles);
    }

    pub(in crate::cpu) fn tick_m(&mut self, bus: &mut impl CpuContext, mcycles: u8) {
        let tcycles = bus.cpu_tcycles_for_mcycles(mcycles);
        self.tick_t(bus, tcycles);
    }

    pub(in crate::cpu) fn read_byte(&mut self, bus: &mut impl CpuContext, addr: u16) -> u8 {
        let value = bus.read_byte(addr);
        self.tick_m(bus, 1);
        value
    }

    pub(in crate::cpu) fn write_byte(&mut self, bus: &mut impl CpuContext, addr: u16, value: u8) {
        bus.write_byte(addr, value);
        self.tick_m(bus, 1);
    }

    pub(in crate::cpu) fn read_word(&mut self, bus: &mut impl CpuContext, addr: u16) -> u16 {
        let low = self.read_byte(bus, addr) as u16;
        let high = self.read_byte(bus, addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    pub(in crate::cpu) fn fetch_d8(&mut self, bus: &mut impl CpuContext) -> u8 {
        let value = self.read_byte(bus, self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    pub(in crate::cpu) fn fetch_d16(&mut self, bus: &mut impl CpuContext) -> u16 {
        let low = self.fetch_d8(bus) as u16;
        let high = self.fetch_d8(bus) as u16;
        (high << 8) | low
    }

    pub(in crate::cpu) fn push_u16(&mut self, bus: &mut impl CpuContext, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, (value & 0xFF) as u8);
    }

    pub(in crate::cpu) fn pop_u16(&mut self, bus: &mut impl CpuContext) -> u16 {
        let low = self.read_byte(bus, self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let high = self.read_byte(bus, self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        (high << 8) | low
    }
}
