use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::memory::Bus;

pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cpu: Cpu::new(),
            bus: Bus::new(cartridge),
        }
    }

    // Ejecuta un ciclo
    pub fn step(&mut self) -> u8 {
        self.cpu.step(&mut self.bus)
    }

    // Bucle de ejecución
    pub fn run(&mut self) {
        println!("ROM: {}", self.bus.rom_title());
        loop {
            let cycles = self.step();
            println!(
                "PC: {:04X}, A: {:02X}, cycles: {}",
                self.cpu.registers.pc, self.cpu.registers.a, cycles
            );
        }
    }
}
