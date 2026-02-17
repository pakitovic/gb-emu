mod alu_helpers;
mod cb;
mod instructions;
pub mod registers;
mod timing_helpers;

use crate::memory::Bus;
use registers::Registers;

pub struct Cpu {
    pub registers: Registers,
    ime: bool,
    ime_enable_pending: bool,
    halted: bool,
    halt_bug: bool,
    step_tcycles: u8,
}

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
            step_tcycles: 0,
        }
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
        self.tick_t(bus, 12);
        20
    }
}
