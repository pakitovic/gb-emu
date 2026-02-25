use crate::cpu::CpuContext;
use crate::cpu::{
    Cpu, get_flag_c, get_flag_h, get_flag_n, set_flag_c, set_flag_h, set_flag_n, set_flag_z,
};

impl Cpu {
    pub(in crate::cpu) fn execute_instr_alu(
        &mut self,
        opcode: u8,
        bus: &mut impl CpuContext,
    ) -> Option<u8> {
        let cycles = match opcode {
            // XOR A
            0xAF => {
                self.registers.a ^= self.registers.a;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 1)
            }
            0xA8 => {
                self.registers.a ^= self.registers.b;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 1)
            }
            0xA9 => {
                self.registers.a ^= self.registers.c;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 1)
            }
            0xAA => {
                self.registers.a ^= self.registers.d;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 1)
            }
            0xAB => {
                self.registers.a ^= self.registers.e;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 1)
            }
            0xAC => {
                self.registers.a ^= self.registers.h;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 1)
            }
            0xAD => {
                self.registers.a ^= self.registers.l;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 1)
            }
            0xAE => {
                self.registers.a ^= self.read_byte(bus, self.hl());
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 2)
            }
            0xEE => {
                self.registers.a ^= self.fetch_d8(bus);
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                self.ret_m(bus, 2)
            }

            // DAA
            0x27 => {
                let mut a = self.registers.a;
                let mut adjust = 0u8;
                let mut carry = get_flag_c(self.registers.f);

                if !get_flag_n(self.registers.f) {
                    if get_flag_c(self.registers.f) || a > 0x99 {
                        adjust |= 0x60;
                        carry = true;
                    }
                    if get_flag_h(self.registers.f) || (a & 0x0F) > 0x09 {
                        adjust |= 0x06;
                    }
                    a = a.wrapping_add(adjust);
                } else {
                    if get_flag_c(self.registers.f) {
                        adjust |= 0x60;
                    }
                    if get_flag_h(self.registers.f) {
                        adjust |= 0x06;
                    }
                    a = a.wrapping_sub(adjust);
                }

                self.registers.a = a;
                set_flag_z(&mut self.registers.f, a == 0);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, carry);
                self.ret_m(bus, 1)
            }

            // CPL
            0x2F => {
                self.registers.a = !self.registers.a;
                set_flag_n(&mut self.registers.f, true);
                set_flag_h(&mut self.registers.f, true);
                self.ret_m(bus, 1)
            }

            // SCF
            0x37 => {
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, true);
                self.ret_m(bus, 1)
            }

            // CCF
            0x3F => {
                let carry = !get_flag_c(self.registers.f);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, carry);
                self.ret_m(bus, 1)
            }

            // ADD A, r
            0x87 => {
                self.add_a(self.registers.a);
                self.ret_m(bus, 1)
            }
            0x80 => {
                self.add_a(self.registers.b);
                self.ret_m(bus, 1)
            }
            0x81 => {
                self.add_a(self.registers.c);
                self.ret_m(bus, 1)
            }
            0x82 => {
                self.add_a(self.registers.d);
                self.ret_m(bus, 1)
            }
            0x83 => {
                self.add_a(self.registers.e);
                self.ret_m(bus, 1)
            }
            0x84 => {
                self.add_a(self.registers.h);
                self.ret_m(bus, 1)
            }
            0x85 => {
                self.add_a(self.registers.l);
                self.ret_m(bus, 1)
            }
            0x86 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.add_a(value);
                self.ret_m(bus, 2)
            }
            0xC6 => {
                let value = self.fetch_d8(bus);
                self.add_a(value);
                self.ret_m(bus, 2)
            }

            // ADC A, r
            0x8F => {
                self.adc_a(self.registers.a);
                self.ret_m(bus, 1)
            }
            0x88 => {
                self.adc_a(self.registers.b);
                self.ret_m(bus, 1)
            }
            0x89 => {
                self.adc_a(self.registers.c);
                self.ret_m(bus, 1)
            }
            0x8A => {
                self.adc_a(self.registers.d);
                self.ret_m(bus, 1)
            }
            0x8B => {
                self.adc_a(self.registers.e);
                self.ret_m(bus, 1)
            }
            0x8C => {
                self.adc_a(self.registers.h);
                self.ret_m(bus, 1)
            }
            0x8D => {
                self.adc_a(self.registers.l);
                self.ret_m(bus, 1)
            }
            0x8E => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.adc_a(value);
                self.ret_m(bus, 2)
            }
            0xCE => {
                let value = self.fetch_d8(bus);
                self.adc_a(value);
                self.ret_m(bus, 2)
            }

            // SUB A, r
            0x97 => {
                self.sub_a(self.registers.a);
                self.ret_m(bus, 1)
            }
            0x90 => {
                self.sub_a(self.registers.b);
                self.ret_m(bus, 1)
            }
            0x91 => {
                self.sub_a(self.registers.c);
                self.ret_m(bus, 1)
            }
            0x92 => {
                self.sub_a(self.registers.d);
                self.ret_m(bus, 1)
            }
            0x93 => {
                self.sub_a(self.registers.e);
                self.ret_m(bus, 1)
            }
            0x94 => {
                self.sub_a(self.registers.h);
                self.ret_m(bus, 1)
            }
            0x95 => {
                self.sub_a(self.registers.l);
                self.ret_m(bus, 1)
            }
            0x96 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.sub_a(value);
                self.ret_m(bus, 2)
            }
            0xD6 => {
                let value = self.fetch_d8(bus);
                self.sub_a(value);
                self.ret_m(bus, 2)
            }

            // SBC A, r
            0x9F => {
                self.sbc_a(self.registers.a);
                self.ret_m(bus, 1)
            }
            0x98 => {
                self.sbc_a(self.registers.b);
                self.ret_m(bus, 1)
            }
            0x99 => {
                self.sbc_a(self.registers.c);
                self.ret_m(bus, 1)
            }
            0x9A => {
                self.sbc_a(self.registers.d);
                self.ret_m(bus, 1)
            }
            0x9B => {
                self.sbc_a(self.registers.e);
                self.ret_m(bus, 1)
            }
            0x9C => {
                self.sbc_a(self.registers.h);
                self.ret_m(bus, 1)
            }
            0x9D => {
                self.sbc_a(self.registers.l);
                self.ret_m(bus, 1)
            }
            0x9E => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.sbc_a(value);
                self.ret_m(bus, 2)
            }
            0xDE => {
                let value = self.fetch_d8(bus);
                self.sbc_a(value);
                self.ret_m(bus, 2)
            }

            // AND A, r
            0xA7 => {
                self.and_a(self.registers.a);
                self.ret_m(bus, 1)
            }
            0xA0 => {
                self.and_a(self.registers.b);
                self.ret_m(bus, 1)
            }
            0xA1 => {
                self.and_a(self.registers.c);
                self.ret_m(bus, 1)
            }
            0xA2 => {
                self.and_a(self.registers.d);
                self.ret_m(bus, 1)
            }
            0xA3 => {
                self.and_a(self.registers.e);
                self.ret_m(bus, 1)
            }
            0xA4 => {
                self.and_a(self.registers.h);
                self.ret_m(bus, 1)
            }
            0xA5 => {
                self.and_a(self.registers.l);
                self.ret_m(bus, 1)
            }
            0xA6 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.and_a(value);
                self.ret_m(bus, 2)
            }
            0xE6 => {
                let value = self.fetch_d8(bus);
                self.and_a(value);
                self.ret_m(bus, 2)
            }

            // OR A, r
            0xB7 => {
                self.or_a(self.registers.a);
                self.ret_m(bus, 1)
            }
            0xB0 => {
                self.or_a(self.registers.b);
                self.ret_m(bus, 1)
            }
            0xB1 => {
                self.or_a(self.registers.c);
                self.ret_m(bus, 1)
            }
            0xB2 => {
                self.or_a(self.registers.d);
                self.ret_m(bus, 1)
            }
            0xB3 => {
                self.or_a(self.registers.e);
                self.ret_m(bus, 1)
            }
            0xB4 => {
                self.or_a(self.registers.h);
                self.ret_m(bus, 1)
            }
            0xB5 => {
                self.or_a(self.registers.l);
                self.ret_m(bus, 1)
            }
            0xB6 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.or_a(value);
                self.ret_m(bus, 2)
            }
            0xF6 => {
                let value = self.fetch_d8(bus);
                self.or_a(value);
                self.ret_m(bus, 2)
            }

            // CP A, r
            0xBF => {
                self.cp_a(self.registers.a);
                self.ret_m(bus, 1)
            }
            0xB8 => {
                self.cp_a(self.registers.b);
                self.ret_m(bus, 1)
            }
            0xB9 => {
                self.cp_a(self.registers.c);
                self.ret_m(bus, 1)
            }
            0xBA => {
                self.cp_a(self.registers.d);
                self.ret_m(bus, 1)
            }
            0xBB => {
                self.cp_a(self.registers.e);
                self.ret_m(bus, 1)
            }
            0xBC => {
                self.cp_a(self.registers.h);
                self.ret_m(bus, 1)
            }
            0xBD => {
                self.cp_a(self.registers.l);
                self.ret_m(bus, 1)
            }
            0xBE => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.cp_a(value);
                self.ret_m(bus, 2)
            }
            0xFE => {
                let value = self.fetch_d8(bus);
                self.cp_a(value);
                self.ret_m(bus, 2)
            }

            _ => return None,
        };

        Some(cycles)
    }
}
