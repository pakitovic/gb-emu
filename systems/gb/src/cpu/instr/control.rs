use crate::cpu::CpuContext;
use crate::cpu::{Cpu, get_flag_c, set_flag_c, set_flag_h, set_flag_n, set_flag_z};

impl Cpu {
    pub(in crate::cpu) fn execute_instr_control(
        &mut self,
        opcode: u8,
        bus: &mut impl CpuContext,
    ) -> Option<u8> {
        let cycles = match opcode {
            // NOP
            0x00 => self.ret_m(bus, 1),
            0xCB => self.execute_cb(bus),
            0x10 => {
                let _ = self.fetch_d8(bus);
                self.ret_m(bus, 1)
            }
            0x76 => {
                if self.ime || bus.pending_interrupts() == 0 {
                    self.halted = true;
                } else {
                    self.halt_bug = true;
                }
                self.ret_m(bus, 1)
            }

            // RLCA
            0x07 => {
                let c = (self.registers.a & 0x80) != 0;
                self.registers.a = self.registers.a.rotate_left(1);
                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, c);
                self.ret_m(bus, 1)
            }
            // RRCA
            0x0F => {
                let c = (self.registers.a & 0x01) != 0;
                self.registers.a = self.registers.a.rotate_right(1);
                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, c);
                self.ret_m(bus, 1)
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
                self.ret_m(bus, 1)
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
                self.ret_m(bus, 1)
            }

            // PUSH rr
            0xC5 => {
                let value = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, value);
                self.ret_m(bus, 4)
            }
            0xD5 => {
                let value = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, value);
                self.ret_m(bus, 4)
            }
            0xE5 => {
                let value = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, value);
                self.ret_m(bus, 4)
            }
            0xF5 => {
                let value = ((self.registers.a as u16) << 8) | self.registers.f as u16;
                self.tick_m(bus, 1); // internal delay (M1)
                self.push_u16(bus, value);
                self.ret_m(bus, 4)
            }

            // POP rr
            0xC1 => {
                let value = self.pop_u16(bus);
                self.registers.b = (value >> 8) as u8;
                self.registers.c = (value & 0xFF) as u8;
                self.ret_m(bus, 3)
            }
            0xD1 => {
                let value = self.pop_u16(bus);
                self.registers.d = (value >> 8) as u8;
                self.registers.e = (value & 0xFF) as u8;
                self.ret_m(bus, 3)
            }
            0xE1 => {
                let value = self.pop_u16(bus);
                self.registers.h = (value >> 8) as u8;
                self.registers.l = (value & 0xFF) as u8;
                self.ret_m(bus, 3)
            }
            0xF1 => {
                let value = self.pop_u16(bus);
                self.registers.a = (value >> 8) as u8;
                self.registers.f = (value & 0xF0) as u8;
                self.ret_m(bus, 3)
            }

            // DI
            0xF3 => {
                self.ime = false;
                self.ime_enable_delay = 0;
                self.ret_m(bus, 1)
            }

            // EI
            0xFB => {
                if self.ime_enable_delay == 0 {
                    self.ime_enable_delay = 2;
                }
                self.ret_m(bus, 1)
            }

            // RETI
            0xD9 => {
                self.registers.pc = self.pop_u16(bus);
                self.ime = true;
                self.ime_enable_delay = 0;
                self.ret_m(bus, 4)
            }

            _ => return None,
        };

        Some(cycles)
    }
}
