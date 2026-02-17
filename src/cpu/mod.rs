pub mod registers;

use crate::memory::Bus;
use registers::Registers;

pub struct Cpu {
    pub registers: Registers,
}

// ---------------- Flags helpers ----------------
fn set_flag_z(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 7;
    } else {
        *f &= !(1 << 7);
    }
}
fn set_flag_n(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 6;
    } else {
        *f &= !(1 << 6);
    }
}
fn set_flag_h(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 5;
    } else {
        *f &= !(1 << 5);
    }
}
fn set_flag_c(f: &mut u8, cond: bool) {
    if cond {
        *f |= 1 << 4;
    } else {
        *f &= !(1 << 4);
    }
}
fn get_flag_z(f: u8) -> bool {
    (f & (1 << 7)) != 0
}
fn get_flag_c(f: u8) -> bool {
    (f & (1 << 4)) != 0
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            registers: Registers::default(),
        }
    }

    // ---------------- Basic operations ----------------

    // Read immediate byte and advance PC
    fn fetch_d8(&mut self, bus: &Bus) -> u8 {
        let value = bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    // Read immediate word and advance PC
    fn fetch_d16(&mut self, bus: &Bus) -> u16 {
        let value = bus.read_word(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(2);
        value
    }

    // 8-bit increment with flag updates
    fn inc_r(&mut self, value: u8) -> u8 {
        let old = value;
        let result = old.wrapping_add(1);

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, (old & 0x0F) + 1 > 0x0F);

        result
    }

    // 8-bit decrement with flag updates
    fn dec_r(&mut self, value: u8) -> u8 {
        let old = value;
        let result = old.wrapping_sub(1);

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (old & 0x0F) == 0);

        result
    }

    // ADD A, r
    fn add_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let result = a.wrapping_add(value);
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, (a & 0x0F) + (value & 0x0F) > 0x0F);
        set_flag_c(&mut self.registers.f, (a as u16 + value as u16) > 0xFF);

        4
    }

    // ADC A, r
    fn adc_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let carry = if get_flag_c(self.registers.f) { 1 } else { 0 };
        let result = a.wrapping_add(value).wrapping_add(carry);
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(
            &mut self.registers.f,
            (a & 0x0F) + (value & 0x0F) + carry > 0x0F,
        );
        set_flag_c(
            &mut self.registers.f,
            (a as u16 + value as u16 + carry as u16) > 0xFF,
        );

        4
    }

    // SUB A, r
    fn sub_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (a & 0x0F) < (value & 0x0F));
        set_flag_c(&mut self.registers.f, a < value);

        4
    }

    // SBC A, r
    fn sbc_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let carry = if get_flag_c(self.registers.f) { 1 } else { 0 };
        let result = a.wrapping_sub(value).wrapping_sub(carry);
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (a & 0x0F) < ((value & 0x0F) + carry));
        set_flag_c(
            &mut self.registers.f,
            (a as u16) < (value as u16 + carry as u16),
        );

        4
    }

    // AND A, r
    fn and_a(&mut self, value: u8) -> u8 {
        let result = self.registers.a & value;
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, true);
        set_flag_c(&mut self.registers.f, false);

        4
    }

    // OR A, r
    fn or_a(&mut self, value: u8) -> u8 {
        let result = self.registers.a | value;
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, false);
        set_flag_c(&mut self.registers.f, false);

        4
    }

    // CP A, r
    fn cp_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (a & 0x0F) < (value & 0x0F));
        set_flag_c(&mut self.registers.f, a < value);

        4
    }

    fn push_u16(&mut self, bus: &mut Bus, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        bus.write_byte(self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        bus.write_byte(self.registers.sp, (value & 0xFF) as u8);
    }

    fn pop_u16(&mut self, bus: &Bus) -> u16 {
        let low = bus.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let high = bus.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        (high << 8) | low
    }

    // ---------------- Step ----------------
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let opcode = bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1); // fetch increment

        match opcode {
            // NOP
            0x00 => 4,

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
            0x7E => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = bus.read_byte(hl);
                8
            }

            // LD (HL), r
            0x77 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                bus.write_byte(hl, self.registers.a);
                8
            }

            // LD (HL-), A
            0x32 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                bus.write_byte(hl, self.registers.a);
                let hl_new = hl.wrapping_sub(1);
                self.registers.h = (hl_new >> 8) as u8;
                self.registers.l = (hl_new & 0xFF) as u8;
                8
            }

            // LD (HL+), A
            0x22 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                bus.write_byte(hl, self.registers.a);
                let hl_new = hl.wrapping_add(1);
                self.registers.h = (hl_new >> 8) as u8;
                self.registers.l = (hl_new & 0xFF) as u8;
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

            // XOR A
            0xAF => {
                self.registers.a ^= self.registers.a;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
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
                let value = bus.read_byte(hl);
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
                let value = bus.read_byte(hl);
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
                let addr = bus.read_word(self.registers.pc);
                self.registers.pc = addr;
                16
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
                let offset = bus.read_byte(self.registers.pc) as i8;
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
            0x21 => {
                // LD HL,d16
                let value = bus.read_word(self.registers.pc);
                self.registers.h = (value >> 8) as u8;
                self.registers.l = (value & 0xFF) as u8;
                self.registers.pc = self.registers.pc.wrapping_add(2);
                12
            }
            0x31 => {
                self.registers.sp = self.fetch_d16(bus);
                12
            }
            0xFA => {
                let addr = self.fetch_d16(bus);
                self.registers.a = bus.read_byte(addr);
                16
            }
            0xEA => {
                let addr = self.fetch_d16(bus);
                bus.write_byte(addr, self.registers.a);
                16
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
                let value = bus.read_byte(hl);
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
                let value = bus.read_byte(hl);
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
                let value = bus.read_byte(hl);
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
                let value = bus.read_byte(hl);
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
                let value = bus.read_byte(hl);
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

            // Default: opcode not implemented
            _ => panic!("Opcode not implemented: {:02X}", opcode),
        }
    }
}
