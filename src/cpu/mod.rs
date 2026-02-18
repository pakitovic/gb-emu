mod alu_helpers;
mod cb;
mod instr_alu;
mod instr_control;
mod instr_jump;
mod instr_load;
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

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;

    fn make_test_bus() -> Bus {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00; // ROM-only
        rom[0x0148] = 0x00; // 32KB
        let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
        Bus::new(cart)
    }

    #[test]
    fn push_and_pop_u16_roundtrip() {
        let mut cpu = Cpu::new();
        let mut bus = make_test_bus();
        cpu.registers.sp = 0xD000;

        cpu.push_u16(&mut bus, 0xBEEF);
        assert_eq!(cpu.registers.sp, 0xCFFE);

        let value = cpu.pop_u16(&mut bus);
        assert_eq!(value, 0xBEEF);
        assert_eq!(cpu.registers.sp, 0xD000);
    }

    #[test]
    fn fetch_d16_reads_little_endian_and_advances_pc() {
        let mut cpu = Cpu::new();
        let mut bus = make_test_bus();
        cpu.registers.pc = 0xC100;

        bus.write_byte(0xC100, 0x34);
        bus.write_byte(0xC101, 0x12);

        let value = cpu.fetch_d16(&mut bus);
        assert_eq!(value, 0x1234);
        assert_eq!(cpu.registers.pc, 0xC102);
    }

    #[test]
    fn pop_hl_pops_once_and_updates_sp_by_two() {
        let mut cpu = Cpu::new();
        let mut bus = make_test_bus();

        cpu.registers.pc = 0xC000;
        cpu.registers.sp = 0xD000;
        bus.write_byte(0xC000, 0xE1); // POP HL
        bus.write_byte(0xD000, 0x34); // low byte
        bus.write_byte(0xD001, 0x12); // high byte

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(cpu.hl(), 0x1234);
        assert_eq!(cpu.registers.sp, 0xD002);
        assert_eq!(cpu.registers.pc, 0xC001);
    }
}
