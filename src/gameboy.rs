mod cpu;
mod memory;

use crate::cpu::Cpu;
use crate::memory::Bus;
use std::fs;

pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new(rom_data: Vec<u8>) -> Self {
        Self {
            cpu: Cpu::new(),
            bus: Bus::new(rom_data),
        }
    }

    // Ejecuta un ciclo
    pub fn step(&mut self) -> u8 {
        self.cpu.step(&mut self.bus)
    }

    // Bucle de ejecución
    pub fn run(&mut self) {
        loop {
            let cycles = self.step();
            println!(
                "PC: {:04X}, A: {:02X}, cycles: {}",
                self.cpu.registers.pc, self.cpu.registers.a, cycles
            );
        }
    }
}

pub fn load_rom(path: &str) -> Vec<u8> {
    fs::read(path).expect("Failed to read ROM")
}
