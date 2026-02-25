use super::Bus;
use crate::cpu::CpuContext;

impl CpuContext for Bus {
    fn read_byte(&self, addr: u16) -> u8 {
        Bus::read_byte(self, addr)
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        Bus::write_byte(self, addr, value);
    }

    fn tick(&mut self, tcycles: u8) {
        Bus::tick(self, tcycles);
    }

    fn cpu_tcycles_for_mcycles(&self, mcycles: u8) -> u8 {
        Bus::cpu_tcycles_for_mcycles(self, mcycles)
    }

    fn pending_interrupts(&self) -> u8 {
        Bus::pending_interrupts(self)
    }

    fn interrupt_flags(&self) -> u8 {
        Bus::interrupt_flags(self)
    }

    fn set_interrupt_flags(&mut self, value: u8) {
        Bus::set_interrupt_flags(self, value);
    }
}
