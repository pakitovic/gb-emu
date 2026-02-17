pub mod registers;

use crate::memory::Bus;
use registers::Registers;

pub struct Cpu {
    pub registers: Registers,
}

// ---------------- Flags helpers ----------------
fn set_flag_z(f: &mut u8, cond: bool) {
    if cond { *f |= 1 << 7; } else { *f &= !(1 << 7); }
}
fn set_flag_n(f: &mut u8, cond: bool) {
    if cond { *f |= 1 << 6; } else { *f &= !(1 << 6); }
}
fn set_flag_h(f: &mut u8, cond: bool) {
    if cond { *f |= 1 << 5; } else { *f &= !(1 << 5); }
}
fn set_flag_c(f: &mut u8, cond: bool) {
    if cond { *f |= 1 << 4; } else { *f &= !(1 << 4); }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            registers: Registers::default(),
        }
    }

    // ---------------- Operaciones básicas ----------------

    // Carga inmediata 8 bits
    fn load_r_d8(&mut self, reg: &mut u8, bus: &mut Bus) -> u8 {
        let pc = self.registers.pc;
        let value = bus.read_byte(pc);
        *reg = value;
        self.registers.pc = pc.wrapping_add(1);
        8
    }

    // Carga inmediata 16 bits
    fn load_rr_d16(&mut self, reg: &mut u16, bus: &mut Bus) -> u8 {
        let pc = self.registers.pc;
        let value = bus.read_word(pc);
        *reg = value;
        self.registers.pc = pc.wrapping_add(2);
        12
    }

    // Incremento de 8 bits con actualización de flags
    fn inc_r(&mut self, reg: &mut u8) -> u8 {
        let old = *reg;
        let result = old.wrapping_add(1);
        *reg = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, (old & 0x0F) + 1 > 0x0F);

        4
    }

    // Decremento de 8 bits con actualización de flags
    fn dec_r(&mut self, reg: &mut u8) -> u8 {
        let old = *reg;
        let result = old.wrapping_sub(1);
        *reg = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (old & 0x0F) == 0);

        4
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

    // ---------------- Step ----------------
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let opcode = bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1); // fetch increment

        match opcode {
            // NOP
            0x00 => 4,

            // LD r, d8
            0x3E => self.load_r_d8(&mut self.registers.a, bus),
            0x06 => self.load_r_d8(&mut self.registers.b, bus),
            0x0E => self.load_r_d8(&mut self.registers.c, bus),
            0x16 => self.load_r_d8(&mut self.registers.d, bus),
            0x1E => self.load_r_d8(&mut self.registers.e, bus),
            0x26 => self.load_r_d8(&mut self.registers.h, bus),
            0x2E => self.load_r_d8(&mut self.registers.l, bus),

            // LD r1, r2 → 8-bit copy
            0x78 => { self.registers.a = self.registers.b; 4 },
            0x79 => { self.registers.a = self.registers.c; 4 },
            0x7A => { self.registers.a = self.registers.d; 4 },
            0x7B => { self.registers.a = self.registers.e; 4 },
            0x7C => { self.registers.a = self.registers.h; 4 },
            0x7D => { self.registers.a = self.registers.l; 4 },

            // LD (HL), r
            0x77 => { 
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                bus.write_byte(hl, self.registers.a);
                8
            },

            // LD (HL-), A
            0x32 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                bus.write_byte(hl, self.registers.a);
                let hl_new = hl.wrapping_sub(1);
                self.registers.h = (hl_new >> 8) as u8;
                self.registers.l = (hl_new & 0xFF) as u8;
                8
            },

            // LD (HL+), A
            0x22 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                bus.write_byte(hl, self.registers.a);
                let hl_new = hl.wrapping_add(1);
                self.registers.h = (hl_new >> 8) as u8;
                self.registers.l = (hl_new & 0xFF) as u8;
                8
            },

            // INC r
            0x3C => self.inc_r(&mut self.registers.a),
            0x04 => self.inc_r(&mut self.registers.b),
            0x0C => self.inc_r(&mut self.registers.c),
            0x14 => self.inc_r(&mut self.registers.d),
            0x1C => self.inc_r(&mut self.registers.e),
            0x24 => self.inc_r(&mut self.registers.h),
            0x2C => self.inc_r(&mut self.registers.l),

            // DEC r
            0x3D => self.dec_r(&mut self.registers.a),
            0x05 => self.dec_r(&mut self.registers.b),
            0x0D => self.dec_r(&mut self.registers.c),
            0x15 => self.dec_r(&mut self.registers.d),
            0x1D => self.dec_r(&mut self.registers.e),
            0x25 => self.dec_r(&mut self.registers.h),
            0x2D => self.dec_r(&mut self.registers.l),

            // XOR A
            0xAF => {
                self.registers.a ^= self.registers.a;
                set_flag_z(&mut self.registers.f, self.registers.a == 0);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, false);
                set_flag_c(&mut self.registers.f, false);
                4
            },

            // ADD A, r
            0x87 => self.add_a(self.registers.a),

            // JP a16
            0xC3 => {
                let addr = bus.read_word(self.registers.pc);
                self.registers.pc = addr;
                16
            },

            // JR r8
            0x18 => {
                let offset = bus.read_byte(self.registers.pc) as i8;
                self.registers.pc = self.registers.pc.wrapping_add(1).wrapping_add(offset as u16);
                12
            },

            // LD rr, d16
            0x21 => { // LD HL,d16
                let value = bus.read_word(self.registers.pc);
                self.registers.h = (value >> 8) as u8;
                self.registers.l = (value & 0xFF) as u8;
                self.registers.pc = self.registers.pc.wrapping_add(2);
                12
            },
            0x31 => self.load_rr_d16(&mut self.registers.sp, bus),

            // Default → opcode no implementado
            _ => panic!("Opcode not implemented: {:02X}", opcode),
        }
    }
}
