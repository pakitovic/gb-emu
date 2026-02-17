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

    // Execute one CPU step
    pub fn step(&mut self) -> u8 {
        self.cpu.step(&mut self.bus)
    }

    // Main execution loop
    pub fn run(&mut self, trace: bool) {
        println!("ROM: {}", self.bus.rom_title());
        loop {
            let cycles = self.step();
            if trace {
                println!(
                    "PC: {:04X}, A: {:02X}, cycles: {}",
                    self.cpu.registers.pc, self.cpu.registers.a, cycles
                );
            }
        }
    }

    pub fn run_blargg(&mut self, max_steps: usize, trace: bool) -> Option<String> {
        println!("ROM: {}", self.bus.rom_title());
        for _ in 0..max_steps {
            let cycles = self.step();
            if trace {
                println!(
                    "PC: {:04X}, A: {:02X}, cycles: {}",
                    self.cpu.registers.pc, self.cpu.registers.a, cycles
                );
            }

            let serial = self.bus.serial_output();
            if serial.contains("Passed") {
                return Some("Passed".to_string());
            }
            if serial.contains("Failed") {
                return Some("Failed".to_string());
            }

            // Blargg memory protocol fallback:
            // A001..A003 == DE B0 61, A000 == status (0 pass, non-zero fail, 0x80 running).
            let sig_ok = self.bus.read_byte(0xA001) == 0xDE
                && self.bus.read_byte(0xA002) == 0xB0
                && self.bus.read_byte(0xA003) == 0x61;
            if sig_ok {
                let status = self.bus.read_byte(0xA000);
                if status == 0x00 {
                    return Some("Passed".to_string());
                }
                if status != 0x80 {
                    return Some("Failed".to_string());
                }
            }
        }
        None
    }
}
