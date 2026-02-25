use crate::cpu::CpuContext;
use crate::cpu::{Cpu, get_flag_c, get_flag_z};

impl Cpu {
    pub(in crate::cpu) fn execute_instr_jump(
        &mut self,
        opcode: u8,
        bus: &mut impl CpuContext,
    ) -> Option<u8> {
        let cycles = match opcode {
            // JP a16 / JP HL / JP cc,a16
            0xC3 => {
                let addr = self.read_word(bus, self.registers.pc);
                self.registers.pc = addr;
                self.ret_m(bus, 4)
            }
            0xE9 => {
                self.registers.pc = self.hl();
                self.ret_m(bus, 1)
            }
            0xC2 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_z(self.registers.f) {
                    self.registers.pc = addr;
                    self.ret_m(bus, 4)
                } else {
                    self.ret_m(bus, 3)
                }
            }
            0xCA => {
                let addr = self.fetch_d16(bus);
                if get_flag_z(self.registers.f) {
                    self.registers.pc = addr;
                    self.ret_m(bus, 4)
                } else {
                    self.ret_m(bus, 3)
                }
            }
            0xD2 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_c(self.registers.f) {
                    self.registers.pc = addr;
                    self.ret_m(bus, 4)
                } else {
                    self.ret_m(bus, 3)
                }
            }
            0xDA => {
                let addr = self.fetch_d16(bus);
                if get_flag_c(self.registers.f) {
                    self.registers.pc = addr;
                    self.ret_m(bus, 4)
                } else {
                    self.ret_m(bus, 3)
                }
            }

            // JR r8 / JR cc,r8
            0x18 => {
                let offset = self.read_byte(bus, self.registers.pc) as i8;
                self.registers.pc = self
                    .registers
                    .pc
                    .wrapping_add(1)
                    .wrapping_add(offset as u16);
                self.ret_m(bus, 3)
            }
            0x20 => {
                let offset = self.fetch_d8(bus) as i8;
                if !get_flag_z(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.ret_m(bus, 3)
                } else {
                    self.ret_m(bus, 2)
                }
            }
            0x28 => {
                let offset = self.fetch_d8(bus) as i8;
                if get_flag_z(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.ret_m(bus, 3)
                } else {
                    self.ret_m(bus, 2)
                }
            }
            0x30 => {
                let offset = self.fetch_d8(bus) as i8;
                if !get_flag_c(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.ret_m(bus, 3)
                } else {
                    self.ret_m(bus, 2)
                }
            }
            0x38 => {
                let offset = self.fetch_d8(bus) as i8;
                if get_flag_c(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    self.ret_m(bus, 3)
                } else {
                    self.ret_m(bus, 2)
                }
            }

            // CALL a16 / CALL cc,a16
            0xCD => {
                let addr = self.fetch_d16(bus);
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay before stack push
                self.push_u16(bus, ret_addr);
                self.registers.pc = addr;
                self.ret_m(bus, 6)
            }
            0xC4 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_z(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.tick_m(bus, 1); // internal delay before stack push
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    self.ret_m(bus, 6)
                } else {
                    self.ret_m(bus, 3)
                }
            }
            0xCC => {
                let addr = self.fetch_d16(bus);
                if get_flag_z(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.tick_m(bus, 1); // internal delay before stack push
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    self.ret_m(bus, 6)
                } else {
                    self.ret_m(bus, 3)
                }
            }
            0xD4 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_c(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.tick_m(bus, 1); // internal delay before stack push
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    self.ret_m(bus, 6)
                } else {
                    self.ret_m(bus, 3)
                }
            }
            0xDC => {
                let addr = self.fetch_d16(bus);
                if get_flag_c(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.tick_m(bus, 1); // internal delay before stack push
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    self.ret_m(bus, 6)
                } else {
                    self.ret_m(bus, 3)
                }
            }

            // RET / RET cc
            0xC9 => {
                self.registers.pc = self.pop_u16(bus);
                self.ret_m(bus, 4)
            }
            0xC0 => {
                if !get_flag_z(self.registers.f) {
                    self.tick_m(bus, 1); // internal delay before stack read
                    self.registers.pc = self.pop_u16(bus);
                    self.tick_m(bus, 1); // final internal delay
                    self.ret_m(bus, 5)
                } else {
                    self.ret_m(bus, 2)
                }
            }
            0xC8 => {
                if get_flag_z(self.registers.f) {
                    self.tick_m(bus, 1); // internal delay before stack read
                    self.registers.pc = self.pop_u16(bus);
                    self.tick_m(bus, 1); // final internal delay
                    self.ret_m(bus, 5)
                } else {
                    self.ret_m(bus, 2)
                }
            }
            0xD0 => {
                if !get_flag_c(self.registers.f) {
                    self.tick_m(bus, 1); // internal delay before stack read
                    self.registers.pc = self.pop_u16(bus);
                    self.tick_m(bus, 1); // final internal delay
                    self.ret_m(bus, 5)
                } else {
                    self.ret_m(bus, 2)
                }
            }
            0xD8 => {
                if get_flag_c(self.registers.f) {
                    self.tick_m(bus, 1); // internal delay before stack read
                    self.registers.pc = self.pop_u16(bus);
                    self.tick_m(bus, 1); // final internal delay
                    self.ret_m(bus, 5)
                } else {
                    self.ret_m(bus, 2)
                }
            }

            // RST n
            0xC7 => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0000;
                self.ret_m(bus, 4)
            }
            0xCF => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0008;
                self.ret_m(bus, 4)
            }
            0xD7 => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0010;
                self.ret_m(bus, 4)
            }
            0xDF => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0018;
                self.ret_m(bus, 4)
            }
            0xE7 => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0020;
                self.ret_m(bus, 4)
            }
            0xEF => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0028;
                self.ret_m(bus, 4)
            }
            0xF7 => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0030;
                self.ret_m(bus, 4)
            }
            0xFF => {
                let ret_addr = self.registers.pc;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0038;
                self.ret_m(bus, 4)
            }

            _ => return None,
        };

        Some(cycles)
    }
}
