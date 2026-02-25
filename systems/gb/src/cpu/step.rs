use super::Cpu;
use crate::cpu::CpuContext;

impl Cpu {
    pub fn step(&mut self, bus: &mut impl CpuContext) -> u8 {
        self.step_tcycles = 0;
        let pending = bus.pending_interrupts();

        if self.halted {
            if pending == 0 {
                self.tick_m(bus, 1);
                return self.step_tcycles;
            }
            self.halted = false;
        }

        if self.ime && pending != 0 {
            return self.service_interrupt(bus, pending);
        }

        let opcode = if self.halt_bug {
            self.halt_bug = false;
            self.read_byte(bus, self.registers.pc)
        } else {
            let op = self.read_byte(bus, self.registers.pc);
            self.registers.pc = self.registers.pc.wrapping_add(1);
            op
        };

        let cycles = self
            .execute_instr_control(opcode, bus)
            .or_else(|| self.execute_instr_load(opcode, bus))
            .or_else(|| self.execute_instr_alu(opcode, bus))
            .or_else(|| self.execute_instr_jump(opcode, bus))
            .unwrap_or_else(|| panic!("Opcode not implemented: {:02X}", opcode));

        if self.step_tcycles < cycles {
            self.tick_t(bus, cycles - self.step_tcycles);
        }

        if self.ime_enable_delay > 0 {
            self.ime_enable_delay -= 1;
            if self.ime_enable_delay == 0 {
                self.ime = true;
            }
        }

        cycles
    }
}
