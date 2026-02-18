use super::{
    Cpu, get_flag_c, get_flag_h, get_flag_n, set_flag_c, set_flag_h, set_flag_n, set_flag_z,
};
use crate::memory::Bus;

impl Cpu {
    pub(super) fn execute_instr_alu(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
            // XOR A
            0xAF => {
                self.registers.a ^= self.registers.a;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            }
            0xA8 => {
                self.registers.a ^= self.registers.b;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            }
            0xA9 => {
                self.registers.a ^= self.registers.c;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            }
            0xAA => {
                self.registers.a ^= self.registers.d;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            }
            0xAB => {
                self.registers.a ^= self.registers.e;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            }
            0xAC => {
                self.registers.a ^= self.registers.h;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            }
            0xAD => {
                self.registers.a ^= self.registers.l;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            }
            0xAE => {
                self.registers.a ^= self.read_byte(bus, self.hl());
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                8
            }
            0xEE => {
                self.registers.a ^= self.fetch_d8(bus);
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                8
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
                4
            }

            // CPL
            0x2F => {
                self.registers.a = !self.registers.a;
                set_flag_n(&mut self.registers.f, true);
                set_flag_h(&mut self.registers.f, true);
                4
            }

            // SCF
            0x37 => {
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, true);
                4
            }

            // CCF
            0x3F => {
                let carry = !get_flag_c(self.registers.f);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, carry);
                4
            }

            // ADD A, r
            0x87 => self.add_a(self.registers.a),
            0x80 => self.add_a(self.registers.b),
            0x81 => self.add_a(self.registers.c),
            0x82 => self.add_a(self.registers.d),
            0x83 => self.add_a(self.registers.e),
            0x84 => self.add_a(self.registers.h),
            0x85 => self.add_a(self.registers.l),
            0x86 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.add_a(value);
                8
            }
            0xC6 => {
                let value = self.fetch_d8(bus);
                self.add_a(value);
                8
            }

            // ADC A, r
            0x8F => self.adc_a(self.registers.a),
            0x88 => self.adc_a(self.registers.b),
            0x89 => self.adc_a(self.registers.c),
            0x8A => self.adc_a(self.registers.d),
            0x8B => self.adc_a(self.registers.e),
            0x8C => self.adc_a(self.registers.h),
            0x8D => self.adc_a(self.registers.l),
            0x8E => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.adc_a(value);
                8
            }
            0xCE => {
                let value = self.fetch_d8(bus);
                self.adc_a(value);
                8
            }

            // SUB A, r
            0x97 => self.sub_a(self.registers.a),
            0x90 => self.sub_a(self.registers.b),
            0x91 => self.sub_a(self.registers.c),
            0x92 => self.sub_a(self.registers.d),
            0x93 => self.sub_a(self.registers.e),
            0x94 => self.sub_a(self.registers.h),
            0x95 => self.sub_a(self.registers.l),
            0x96 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.sub_a(value);
                8
            }
            0xD6 => {
                let value = self.fetch_d8(bus);
                self.sub_a(value);
                8
            }

            // SBC A, r
            0x9F => self.sbc_a(self.registers.a),
            0x98 => self.sbc_a(self.registers.b),
            0x99 => self.sbc_a(self.registers.c),
            0x9A => self.sbc_a(self.registers.d),
            0x9B => self.sbc_a(self.registers.e),
            0x9C => self.sbc_a(self.registers.h),
            0x9D => self.sbc_a(self.registers.l),
            0x9E => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.sbc_a(value);
                8
            }
            0xDE => {
                let value = self.fetch_d8(bus);
                self.sbc_a(value);
                8
            }

            // AND A, r
            0xA7 => self.and_a(self.registers.a),
            0xA0 => self.and_a(self.registers.b),
            0xA1 => self.and_a(self.registers.c),
            0xA2 => self.and_a(self.registers.d),
            0xA3 => self.and_a(self.registers.e),
            0xA4 => self.and_a(self.registers.h),
            0xA5 => self.and_a(self.registers.l),
            0xA6 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.and_a(value);
                8
            }
            0xE6 => {
                let value = self.fetch_d8(bus);
                self.and_a(value);
                8
            }

            // OR A, r
            0xB7 => self.or_a(self.registers.a),
            0xB0 => self.or_a(self.registers.b),
            0xB1 => self.or_a(self.registers.c),
            0xB2 => self.or_a(self.registers.d),
            0xB3 => self.or_a(self.registers.e),
            0xB4 => self.or_a(self.registers.h),
            0xB5 => self.or_a(self.registers.l),
            0xB6 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.or_a(value);
                8
            }
            0xF6 => {
                let value = self.fetch_d8(bus);
                self.or_a(value);
                8
            }

            // CP A, r
            0xBF => self.cp_a(self.registers.a),
            0xB8 => self.cp_a(self.registers.b),
            0xB9 => self.cp_a(self.registers.c),
            0xBA => self.cp_a(self.registers.d),
            0xBB => self.cp_a(self.registers.e),
            0xBC => self.cp_a(self.registers.h),
            0xBD => self.cp_a(self.registers.l),
            0xBE => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.read_byte(bus, hl);
                self.cp_a(value);
                8
            }
            0xFE => {
                let value = self.fetch_d8(bus);
                self.cp_a(value);
                8
            }

            _ => return None,
        };

        Some(cycles)
    }
}
