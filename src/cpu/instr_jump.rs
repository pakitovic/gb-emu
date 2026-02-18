use super::{Cpu, get_flag_c, get_flag_z};
use crate::memory::Bus;

impl Cpu {
    pub(super) fn execute_instr_jump(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
            // JP a16 / JP HL / JP cc,a16
            0xC3 => {
                let addr = self.read_word(bus, self.registers.pc);
                self.registers.pc = addr;
                16
            }
            0xE9 => {
                self.registers.pc = self.hl();
                4
            }
            0xC2 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_z(self.registers.f) {
                    self.registers.pc = addr;
                    16
                } else {
                    12
                }
            }
            0xCA => {
                let addr = self.fetch_d16(bus);
                if get_flag_z(self.registers.f) {
                    self.registers.pc = addr;
                    16
                } else {
                    12
                }
            }
            0xD2 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_c(self.registers.f) {
                    self.registers.pc = addr;
                    16
                } else {
                    12
                }
            }
            0xDA => {
                let addr = self.fetch_d16(bus);
                if get_flag_c(self.registers.f) {
                    self.registers.pc = addr;
                    16
                } else {
                    12
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
                12
            }
            0x20 => {
                let offset = self.fetch_d8(bus) as i8;
                if !get_flag_z(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    12
                } else {
                    8
                }
            }
            0x28 => {
                let offset = self.fetch_d8(bus) as i8;
                if get_flag_z(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    12
                } else {
                    8
                }
            }
            0x30 => {
                let offset = self.fetch_d8(bus) as i8;
                if !get_flag_c(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    12
                } else {
                    8
                }
            }
            0x38 => {
                let offset = self.fetch_d8(bus) as i8;
                if get_flag_c(self.registers.f) {
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    12
                } else {
                    8
                }
            }

            // CALL a16 / CALL cc,a16
            0xCD => {
                let addr = self.fetch_d16(bus);
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = addr;
                24
            }
            0xC4 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_z(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    24
                } else {
                    12
                }
            }
            0xCC => {
                let addr = self.fetch_d16(bus);
                if get_flag_z(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    24
                } else {
                    12
                }
            }
            0xD4 => {
                let addr = self.fetch_d16(bus);
                if !get_flag_c(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    24
                } else {
                    12
                }
            }
            0xDC => {
                let addr = self.fetch_d16(bus);
                if get_flag_c(self.registers.f) {
                    let ret_addr = self.registers.pc;
                    self.push_u16(bus, ret_addr);
                    self.registers.pc = addr;
                    24
                } else {
                    12
                }
            }

            // RET / RET cc
            0xC9 => {
                self.registers.pc = self.pop_u16(bus);
                16
            }
            0xC0 => {
                if !get_flag_z(self.registers.f) {
                    self.registers.pc = self.pop_u16(bus);
                    20
                } else {
                    8
                }
            }
            0xC8 => {
                if get_flag_z(self.registers.f) {
                    self.registers.pc = self.pop_u16(bus);
                    20
                } else {
                    8
                }
            }
            0xD0 => {
                if !get_flag_c(self.registers.f) {
                    self.registers.pc = self.pop_u16(bus);
                    20
                } else {
                    8
                }
            }
            0xD8 => {
                if get_flag_c(self.registers.f) {
                    self.registers.pc = self.pop_u16(bus);
                    20
                } else {
                    8
                }
            }

            // RST n
            0xC7 => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0000;
                16
            }
            0xCF => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0008;
                16
            }
            0xD7 => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0010;
                16
            }
            0xDF => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0018;
                16
            }
            0xE7 => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0020;
                16
            }
            0xEF => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0028;
                16
            }
            0xF7 => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0030;
                16
            }
            0xFF => {
                let ret_addr = self.registers.pc;
                self.push_u16(bus, ret_addr);
                self.registers.pc = 0x0038;
                16
            }

            _ => return None,
        };

        Some(cycles)
    }
}
