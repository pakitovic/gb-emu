use super::super::Cpu;
use crate::cartridge::Cartridge;
use crate::memory::Bus;

pub(super) fn make_test_bus() -> Bus {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    Bus::new(cart)
}

pub(super) fn m_tcycles(bus: &Bus, mcycles: u8) -> u8 {
    bus.cpu_tcycles_for_mcycles(mcycles)
}

pub(super) fn halt_idle_tcycles(bus: &Bus) -> u8 {
    m_tcycles(bus, Cpu::HALT_IDLE_STEP_M_CYCLES)
}

pub(super) fn interrupt_service_tcycles(bus: &Bus) -> u8 {
    m_tcycles(bus, Cpu::INTERRUPT_SERVICE_M_CYCLES)
}
