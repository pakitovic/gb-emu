use super::{
    Cpu, get_flag_c, get_flag_h, get_flag_n, get_flag_z, set_flag_c, set_flag_h, set_flag_n,
    set_flag_z,
};
use crate::memory::Bus;

impl Cpu {
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        self.step_tcycles = 0;
        let pending = bus.pending_interrupts();

        if self.halted {
            if pending == 0 {
                self.tick_t(bus, 4);
                return 4;
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
            self.registers.pc = self.registers.pc.wrapping_add(1); // fetch increment
            op
        };

        let cycles = match opcode {
            // NOP
            0x00 => 4,
            0xCB => self.execute_cb(bus),
            0x10 => {
                // STOP is two bytes (0x10 0x00). For now, consume the padding byte.
                let _ = self.fetch_d8(bus);
                4
            }
            0x76 => {
                if self.ime || bus.pending_interrupts() == 0 {
                    self.halted = true;
                } else {
                    // HALT bug: next opcode is fetched without incrementing PC.
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

            // LD r, d8
            0x3E => {
                self.registers.a = self.fetch_d8(bus);
                8
            }
            0x06 => {
                self.registers.b = self.fetch_d8(bus);
                8
            }
            0x0E => {
                self.registers.c = self.fetch_d8(bus);
                8
            }
            0x16 => {
                self.registers.d = self.fetch_d8(bus);
                8
            }
            0x1E => {
                self.registers.e = self.fetch_d8(bus);
                8
            }
            0x26 => {
                self.registers.h = self.fetch_d8(bus);
                8
            }
            0x2E => {
                self.registers.l = self.fetch_d8(bus);
                8
            }

            // LD r1, r2 → 8-bit copy
            0x78 => {
                self.registers.a = self.registers.b;
                4
            }
            0x79 => {
                self.registers.a = self.registers.c;
                4
            }
            0x7A => {
                self.registers.a = self.registers.d;
                4
            }
            0x7B => {
                self.registers.a = self.registers.e;
                4
            }
            0x7C => {
                self.registers.a = self.registers.h;
                4
            }
            0x7D => {
                self.registers.a = self.registers.l;
                4
            }
            0x47 => {
                self.registers.b = self.registers.a;
                4
            }
            0x7E => {
                self.registers.a = self.read_byte(bus, self.hl());
                8
            }
            0x46 => {
                self.registers.b = self.read_byte(bus, self.hl());
                8
            }
            0x4E => {
                self.registers.c = self.read_byte(bus, self.hl());
                8
            }
            0x56 => {
                self.registers.d = self.read_byte(bus, self.hl());
                8
            }
            0x5E => {
                self.registers.e = self.read_byte(bus, self.hl());
                8
            }
            0x66 => {
                self.registers.h = self.read_byte(bus, self.hl());
                8
            }
            0x6E => {
                self.registers.l = self.read_byte(bus, self.hl());
                8
            }
            // LD (HL), r
            0x70 => {
                self.write_byte(bus, self.hl(), self.registers.b);
                8
            }
            0x71 => {
                self.write_byte(bus, self.hl(), self.registers.c);
                8
            }
            0x72 => {
                self.write_byte(bus, self.hl(), self.registers.d);
                8
            }
            0x73 => {
                self.write_byte(bus, self.hl(), self.registers.e);
                8
            }
            0x74 => {
                self.write_byte(bus, self.hl(), self.registers.h);
                8
            }
            0x75 => {
                self.write_byte(bus, self.hl(), self.registers.l);
                8
            }
            0x77 => {
                self.write_byte(bus, self.hl(), self.registers.a);
                8
            }
            0x36 => {
                let value = self.fetch_d8(bus);
                self.write_byte(bus, self.hl(), value);
                12
            }
            0x12 => {
                self.write_byte(bus, self.de(), self.registers.a);
                8
            }
            0x02 => {
                self.write_byte(bus, self.bc(), self.registers.a);
                8
            }

            // LD (HL-), A
            0x32 => {
                let hl = self.hl();
                self.write_byte(bus, hl, self.registers.a);
                self.set_hl(hl.wrapping_sub(1));
                8
            }
            0x3A => {
                let hl = self.hl();
                self.registers.a = self.read_byte(bus, hl);
                self.set_hl(hl.wrapping_sub(1));
                8
            }

            // LD (HL+), A
            0x22 => {
                let hl = self.hl();
                self.write_byte(bus, hl, self.registers.a);
                self.set_hl(hl.wrapping_add(1));
                8
            }
            0x2A => {
                let hl = self.hl();
                self.registers.a = self.read_byte(bus, hl);
                self.set_hl(hl.wrapping_add(1));
                8
            }
            0x1A => {
                self.registers.a = self.read_byte(bus, self.de());
                8
            }
            0x0A => {
                self.registers.a = self.read_byte(bus, self.bc());
                8
            }

            // INC r
            0x3C => {
                self.registers.a = self.inc_r(self.registers.a);
                4
            }
            0x04 => {
                self.registers.b = self.inc_r(self.registers.b);
                4
            }
            0x0C => {
                self.registers.c = self.inc_r(self.registers.c);
                4
            }
            0x14 => {
                self.registers.d = self.inc_r(self.registers.d);
                4
            }
            0x1C => {
                self.registers.e = self.inc_r(self.registers.e);
                4
            }
            0x24 => {
                self.registers.h = self.inc_r(self.registers.h);
                4
            }
            0x2C => {
                self.registers.l = self.inc_r(self.registers.l);
                4
            }
            0x34 => {
                let addr = self.hl();
                let old = self.read_byte(bus, addr);
                let result = old.wrapping_add(1);
                self.write_byte(bus, addr, result);

                set_flag_z(&mut self.registers.f, result == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, (old & 0x0F) + 1 > 0x0F);
                12
            }

            // DEC r
            0x3D => {
                self.registers.a = self.dec_r(self.registers.a);
                4
            }
            0x05 => {
                self.registers.b = self.dec_r(self.registers.b);
                4
            }
            0x0D => {
                self.registers.c = self.dec_r(self.registers.c);
                4
            }
            0x15 => {
                self.registers.d = self.dec_r(self.registers.d);
                4
            }
            0x1D => {
                self.registers.e = self.dec_r(self.registers.e);
                4
            }
            0x25 => {
                self.registers.h = self.dec_r(self.registers.h);
                4
            }
            0x2D => {
                self.registers.l = self.dec_r(self.registers.l);
                4
            }
            0x35 => {
                let addr = self.hl();
                let old = self.read_byte(bus, addr);
                let result = old.wrapping_sub(1);
                self.write_byte(bus, addr, result);

                set_flag_z(&mut self.registers.f, result == 0);
                set_flag_n(&mut self.registers.f, true);
                set_flag_h(&mut self.registers.f, (old & 0x0F) == 0);
                12
            }

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

            // JP a16
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

            // JR r8
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

            // LD rr, d16
            0x01 => {
                let value = self.fetch_d16(bus);
                self.set_bc(value);
                12
            }
            0x21 => {
                // LD HL,d16
                let value = self.read_word(bus, self.registers.pc);
                self.registers.h = (value >> 8) as u8;
                self.registers.l = (value & 0xFF) as u8;
                self.registers.pc = self.registers.pc.wrapping_add(2);
                12
            }
            0x31 => {
                self.registers.sp = self.fetch_d16(bus);
                12
            }
            0x03 => {
                self.set_bc(self.bc().wrapping_add(1));
                8
            }
            0x0B => {
                self.set_bc(self.bc().wrapping_sub(1));
                8
            }
            0x23 => {
                self.set_hl(self.hl().wrapping_add(1));
                8
            }
            0x2B => {
                self.set_hl(self.hl().wrapping_sub(1));
                8
            }
            0x33 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                8
            }
            0x3B => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                8
            }
            0x09 => self.add_hl(self.bc()),
            0x19 => self.add_hl(self.de()),
            0x29 => self.add_hl(self.hl()),
            0x39 => self.add_hl(self.registers.sp),
            0x11 => {
                let value = self.fetch_d16(bus);
                self.registers.d = (value >> 8) as u8;
                self.registers.e = (value & 0xFF) as u8;
                12
            }
            0x13 => {
                let value = self.de().wrapping_add(1);
                self.set_de(value);
                8
            }
            0x1B => {
                let value = self.de().wrapping_sub(1);
                self.set_de(value);
                8
            }
            0xFA => {
                let addr = self.fetch_d16(bus);
                self.registers.a = self.read_byte(bus, addr);
                16
            }
            0xF0 => {
                let addr = 0xFF00u16.wrapping_add(self.fetch_d8(bus) as u16);
                self.registers.a = self.read_byte(bus, addr);
                12
            }
            0xF2 => {
                let addr = 0xFF00u16.wrapping_add(self.registers.c as u16);
                self.registers.a = self.read_byte(bus, addr);
                8
            }
            0xEA => {
                let addr = self.fetch_d16(bus);
                self.write_byte(bus, addr, self.registers.a);
                16
            }
            0xE0 => {
                let addr = 0xFF00u16.wrapping_add(self.fetch_d8(bus) as u16);
                self.write_byte(bus, addr, self.registers.a);
                12
            }
            0xE2 => {
                let addr = 0xFF00u16.wrapping_add(self.registers.c as u16);
                self.write_byte(bus, addr, self.registers.a);
                8
            }
            0x08 => {
                let addr = self.fetch_d16(bus);
                let sp = self.registers.sp;
                self.write_byte(bus, addr, (sp & 0x00FF) as u8);
                self.write_byte(bus, addr.wrapping_add(1), (sp >> 8) as u8);
                20
            }
            0xF9 => {
                self.registers.sp = self.hl();
                8
            }
            0xE8 => {
                let offset = self.fetch_d8(bus) as i8;
                let sp = self.registers.sp;
                let n = offset as i16 as u16;
                let result = sp.wrapping_add(n);

                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(
                    &mut self.registers.f,
                    ((sp & 0x000F) + (n & 0x000F)) > 0x000F,
                );
                set_flag_c(
                    &mut self.registers.f,
                    ((sp & 0x00FF) + (n & 0x00FF)) > 0x00FF,
                );

                self.registers.sp = result;
                16
            }
            0xF8 => {
                let offset = self.fetch_d8(bus) as i8;
                let sp = self.registers.sp;
                let n = offset as i16 as u16;
                let result = sp.wrapping_add(n);

                set_flag_z(&mut self.registers.f, false);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(
                    &mut self.registers.f,
                    ((sp & 0x000F) + (n & 0x000F)) > 0x000F,
                );
                set_flag_c(
                    &mut self.registers.f,
                    ((sp & 0x00FF) + (n & 0x00FF)) > 0x00FF,
                );

                self.set_hl(result);
                12
            }

            // CALL a16
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

            // RET
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
            0xD9 => {
                self.registers.pc = self.pop_u16(bus);
                self.ime = true;
                self.ime_enable_pending = false;
                16
            }

            // RST 38h
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

            // Fallback for remaining LD r, r opcodes not handled above
            0x40..=0x7F => {
                let dst = (opcode >> 3) & 0x07;
                let src = opcode & 0x07;
                let value = self.read_r8_by_index(src, bus);
                self.write_r8_by_index(dst, value, bus);

                if src == 6 || dst == 6 { 8 } else { 4 }
            }

            // Default: opcode not implemented
            _ => panic!("Opcode not implemented: {:02X}", opcode),
        };

        if self.step_tcycles < cycles {
            self.tick_t(bus, cycles - self.step_tcycles);
        }

        if self.ime_enable_pending && opcode != 0xFB {
            self.ime = true;
            self.ime_enable_pending = false;
        }

        cycles
    }
}
