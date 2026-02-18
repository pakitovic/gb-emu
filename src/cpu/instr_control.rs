use super::{Cpu, get_flag_c, set_flag_c, set_flag_h, set_flag_n, set_flag_z};
use crate::memory::Bus;

impl Cpu {
    pub(super) fn execute_instr_control(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
            // NOP
            0x00 => 4,
            0xCB => self.execute_cb(bus),
            0x10 => {
                let _ = self.fetch_d8(bus);
                4
            }
            0x76 => {
                if self.ime || bus.pending_interrupts() == 0 {
                    self.halted = true;
                } else {
                    self.halt_bug = true;
                }
                4
            }

            // RLCA
            0x07 => {
                let c = (self.registers.a & 0x80) != 0;
                self.registers.a = self.registers.a.rotate_left(1);
                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, c);
                4
            }
            // RRCA
            0x0F => {
                let c = (self.registers.a & 0x01) != 0;
                self.registers.a = self.registers.a.rotate_right(1);
                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, c);
                4
            }
            // RLA
            0x17 => {
                let c_in = if get_flag_c(self.registers.f) { 1 } else { 0 };
                let c_out = (self.registers.a & 0x80) != 0;
                self.registers.a = (self.registers.a << 1) | c_in;
                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, c_out);
                4
            }
            // RRA
            0x1F => {
                let c_in = if get_flag_c(self.registers.f) {
                    0x80
                } else {
                    0
                };
                let c_out = (self.registers.a & 0x01) != 0;
                self.registers.a = (self.registers.a >> 1) | c_in;
                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, c_out);
                4
            }

            // PUSH rr
            0xC5 => {
                let value = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.push_u16(bus, value);
                16
            }
            0xD5 => {
                let value = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.push_u16(bus, value);
                16
            }
            0xE5 => {
                let value = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.push_u16(bus, value);
                16
            }
            0xF5 => {
                let value = ((self.registers.a as u16) << 8) | self.registers.f as u16;
                self.push_u16(bus, value);
                16
            }

            // POP rr
            0xC1 => {
                let value = self.pop_u16(bus);
                self.registers.b = (value >> 8) as u8;
                self.registers.c = (value & 0xFF) as u8;
                12
            }
            0xD1 => {
                let value = self.pop_u16(bus);
                self.registers.d = (value >> 8) as u8;
                self.registers.e = (value & 0xFF) as u8;
                12
            }
            0xE1 => {
                let value = self.pop_u16(bus);
                self.registers.h = (value >> 8) as u8;
                self.registers.l = (value & 0xFF) as u8;
                12
            }
            0xF1 => {
                let value = self.pop_u16(bus);
                self.registers.a = (value >> 8) as u8;
                self.registers.f = (value & 0xF0) as u8;
                12
            }

            // DI
            0xF3 => {
                self.ime = false;
                self.ime_enable_pending = false;
                4
            }

            // EI
            0xFB => {
                self.ime_enable_pending = true;
                4
            }

            // RETI
            0xD9 => {
                self.registers.pc = self.pop_u16(bus);
                self.ime = true;
                self.ime_enable_pending = false;
                16
            }

            _ => return None,
        };

        Some(cycles)
    }
}
