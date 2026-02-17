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

    // Lee un byte inmediato y avanza PC
    fn fetch_d8(&mut self, bus: &Bus) -> u8 {
        let value = bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    // Lee una palabra inmediata y avanza PC
    fn fetch_d16(&mut self, bus: &Bus) -> u16 {
        let value = bus.read_word(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(2);
        value
    }

    // Incremento de 8 bits con actualización de flags
    fn inc_r(&mut self, value: u8) -> u8 {
        let old = value;
        let result = old.wrapping_add(1);

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, (old & 0x0F) + 1 > 0x0F);

        result
    }

    // Decremento de 8 bits con actualización de flags
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

    // ---------------- Step ----------------
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let opcode = bus.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1); // fetch increment

        match opcode {
            // NOP
            0x00 => 4,

            // LD r, d8
            0x3E => { self.registers.a = self.fetch_d8(bus); 8 },
            0x06 => { self.registers.b = self.fetch_d8(bus); 8 },
            0x0E => { self.registers.c = self.fetch_d8(bus); 8 },
            0x16 => { self.registers.d = self.fetch_d8(bus); 8 },
            0x1E => { self.registers.e = self.fetch_d8(bus); 8 },
            0x26 => { self.registers.h = self.fetch_d8(bus); 8 },
            0x2E => { self.registers.l = self.fetch_d8(bus); 8 },

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
            0x3C => { self.registers.a = self.inc_r(self.registers.a); 4 },
            0x04 => { self.registers.b = self.inc_r(self.registers.b); 4 },
            0x0C => { self.registers.c = self.inc_r(self.registers.c); 4 },
            0x14 => { self.registers.d = self.inc_r(self.registers.d); 4 },
            0x1C => { self.registers.e = self.inc_r(self.registers.e); 4 },
            0x24 => { self.registers.h = self.inc_r(self.registers.h); 4 },
            0x2C => { self.registers.l = self.inc_r(self.registers.l); 4 },

            // DEC r
            0x3D => { self.registers.a = self.dec_r(self.registers.a); 4 },
            0x05 => { self.registers.b = self.dec_r(self.registers.b); 4 },
            0x0D => { self.registers.c = self.dec_r(self.registers.c); 4 },
            0x15 => { self.registers.d = self.dec_r(self.registers.d); 4 },
            0x1D => { self.registers.e = self.dec_r(self.registers.e); 4 },
            0x25 => { self.registers.h = self.dec_r(self.registers.h); 4 },
            0x2D => { self.registers.l = self.dec_r(self.registers.l); 4 },

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
            0x31 => { self.registers.sp = self.fetch_d16(bus); 12 },

            // Default → opcode no implementado
            _ => panic!("Opcode not implemented: {:02X}", opcode),
        }
    }
}
