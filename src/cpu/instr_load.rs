use super::{Cpu, set_flag_c, set_flag_h, set_flag_n, set_flag_z};
use crate::memory::Bus;

impl Cpu {
    pub(super) fn execute_instr_load(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
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

            // LD r1, r2
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

            // LD (HL-), A / LD A, (HL-)
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

            // LD (HL+), A / LD A, (HL+)
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

            // LD rr, d16
            0x01 => {
                let value = self.fetch_d16(bus);
                self.set_bc(value);
                12
            }
            0x21 => {
                let value = self.read_word(bus, self.registers.pc);
                self.registers.h = (value >> 8) as u8;
                self.registers.l = (value & 0xFF) as u8;
                self.registers.l = (value & 0xFF) as u8;
                self.registers.pc = self.registers.pc.wrapping_add(2);
                12
            }
            0x31 => {
                self.registers.sp = self.fetch_d16(bus);
                12
            }
            0x11 => {
                let value = self.fetch_d16(bus);
                self.registers.d = (value >> 8) as u8;
                self.registers.e = (value & 0xFF) as u8;
                12
            }

            // INC/DEC rr
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

            // ADD HL, rr
            0x09 => self.add_hl(self.bc()),
            0x19 => self.add_hl(self.de()),
            0x29 => self.add_hl(self.hl()),
            0x39 => self.add_hl(self.registers.sp),

            // LD A, (a16)/(a8)/(C)
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

            // LD (a16)/(a8)/(C), A
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

            // LD (a16), SP / LD SP, HL / ADD SP, r8 / LD HL, SP+r8
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

            // Fallback for remaining LD r, r opcodes not handled above
            0x40..=0x7F => {
                let dst = (opcode >> 3) & 0x07;
                let src = opcode & 0x07;
                let value = self.read_r8_by_index(src, bus);
                self.write_r8_by_index(dst, value, bus);

                if src == 6 || dst == 6 { 8 } else { 4 }
            }

            _ => return None,
        };

        Some(cycles)
    }
}
