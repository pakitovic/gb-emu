mod instructions;
pub mod registers;

use crate::memory::Bus;
use registers::Registers;

pub struct Cpu {
    pub registers: Registers,
    ime: bool,
    ime_enable_pending: bool,
    halted: bool,
    halt_bug: bool,
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
fn get_flag_n(f: u8) -> bool {
    (f & (1 << 6)) != 0
}
fn get_flag_h(f: u8) -> bool {
    (f & (1 << 5)) != 0
}
fn get_flag_c(f: u8) -> bool {
    (f & (1 << 4)) != 0
}

impl Cpu {
    pub fn new() -> Self {
        // DMG post-boot register state (when boot ROM is skipped).
        let registers = Registers {
            a: 0x01,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
        };

        Self {
            registers,
            ime: false,
            ime_enable_pending: false,
            halted: false,
            halt_bug: false,
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

    fn hl(&self) -> u16 {
        ((self.registers.h as u16) << 8) | self.registers.l as u16
    }

    fn set_hl(&mut self, value: u16) {
        self.registers.h = (value >> 8) as u8;
        self.registers.l = (value & 0xFF) as u8;
    }

    fn de(&self) -> u16 {
        ((self.registers.d as u16) << 8) | self.registers.e as u16
    }

    fn set_de(&mut self, value: u16) {
        self.registers.d = (value >> 8) as u8;
        self.registers.e = (value & 0xFF) as u8;
    }

    fn bc(&self) -> u16 {
        ((self.registers.b as u16) << 8) | self.registers.c as u16
    }

    fn set_bc(&mut self, value: u16) {
        self.registers.b = (value >> 8) as u8;
        self.registers.c = (value & 0xFF) as u8;
    }

    fn add_hl(&mut self, value: u16) -> u8 {
        let hl = self.hl();
        let result = hl.wrapping_add(value);
        self.set_hl(result);

        set_flag_n(&mut self.registers.f, false);
        set_flag_h(
            &mut self.registers.f,
            ((hl & 0x0FFF) + (value & 0x0FFF)) > 0x0FFF,
        );
        set_flag_c(&mut self.registers.f, (hl as u32 + value as u32) > 0xFFFF);

        8
    }

    fn service_interrupt(&mut self, bus: &mut Bus, pending: u8) -> u8 {
        let (bit, vector) = if (pending & 0x01) != 0 {
            (0, 0x0040)
        } else if (pending & 0x02) != 0 {
            (1, 0x0048)
        } else if (pending & 0x04) != 0 {
            (2, 0x0050)
        } else if (pending & 0x08) != 0 {
            (3, 0x0058)
        } else {
            (4, 0x0060)
        };

        let mut iflags = bus.interrupt_flags();
        iflags &= !(1 << bit);
        bus.set_interrupt_flags(iflags);

        self.ime = false;
        self.halted = false;
        self.push_u16(bus, self.registers.pc);
        self.registers.pc = vector;
        20
    }

    fn read_r8_by_index(&self, idx: u8, bus: &Bus) -> u8 {
        match idx {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => bus.read_byte(self.hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        }
    }

    fn write_r8_by_index(&mut self, idx: u8, value: u8, bus: &mut Bus) {
        match idx {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => bus.write_byte(self.hl(), value),
            7 => self.registers.a = value,
            _ => unreachable!(),
        }
    }

    fn execute_cb(&mut self, bus: &mut Bus) -> u8 {
        let cb_opcode = self.fetch_d8(bus);
        let group = cb_opcode >> 6;
        let y = (cb_opcode >> 3) & 0x07;
        let z = cb_opcode & 0x07;

        match group {
            // Rotates / shifts / swap
            0 => {
                let value = self.read_r8_by_index(z, bus);
                let carry_in = if get_flag_c(self.registers.f) { 1 } else { 0 };
                let (result, carry_out) = match y {
                    // RLC
                    0 => {
                        let c = (value & 0x80) != 0;
                        (value.rotate_left(1), c)
                    }
                    // RRC
                    1 => {
                        let c = (value & 0x01) != 0;
                        (value.rotate_right(1), c)
                    }
                    // RL
                    2 => {
                        let c = (value & 0x80) != 0;
                        ((value << 1) | carry_in, c)
                    }
                    // RR
                    3 => {
                        let c = (value & 0x01) != 0;
                        ((value >> 1) | (carry_in << 7), c)
                    }
                    // SLA
                    4 => {
                        let c = (value & 0x80) != 0;
                        (value << 1, c)
                    }
                    // SRA
                    5 => {
                        let c = (value & 0x01) != 0;
                        ((value >> 1) | (value & 0x80), c)
                    }
                    // SWAP
                    6 => (((value & 0x0F) << 4) | ((value & 0xF0) >> 4), false),
                    // SRL
                    7 => {
                        let c = (value & 0x01) != 0;
                        (value >> 1, c)
                    }
                    _ => unreachable!(),
                };

                self.write_r8_by_index(z, result, bus);
                set_flag_z(&mut self.registers.f, result == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, carry_out);

                if z == 6 { 16 } else { 8 }
            }

            // BIT b, r
            1 => {
                let value = self.read_r8_by_index(z, bus);
                let bit_set = (value & (1 << y)) != 0;
                set_flag_z(&mut self.registers.f, !bit_set);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, true);
                if z == 6 { 12 } else { 8 }
            }

            // RES b, r
            2 => {
                let value = self.read_r8_by_index(z, bus);
                let result = value & !(1 << y);
                self.write_r8_by_index(z, result, bus);
                if z == 6 { 16 } else { 8 }
            }

            // SET b, r
            3 => {
                let value = self.read_r8_by_index(z, bus);
                let result = value | (1 << y);
                self.write_r8_by_index(z, result, bus);
                if z == 6 { 16 } else { 8 }
            }

            _ => unreachable!(),
        }
    }
}
