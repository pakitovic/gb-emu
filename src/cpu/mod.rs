mod alu_helpers;
mod cb;
mod instr_alu;
mod instr_control;
mod instr_jump;
mod instr_load;
mod instructions;
pub mod registers;
mod timing_helpers;

use crate::hardware::HardwareModel;
use crate::memory::Bus;
use registers::Registers;

pub struct Cpu {
    pub registers: Registers,
    ime: bool,
    ime_enable_delay: u8,
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
    fn select_interrupt(pending: u8) -> Option<(u8, u16)> {
        if (pending & 0x01) != 0 {
            Some((0, 0x0040))
        } else if (pending & 0x02) != 0 {
            Some((1, 0x0048))
        } else if (pending & 0x04) != 0 {
            Some((2, 0x0050))
        } else if (pending & 0x08) != 0 {
            Some((3, 0x0058))
        } else if (pending & 0x10) != 0 {
            Some((4, 0x0060))
        } else {
            None
        }
    }

    pub fn new() -> Self {
        Self::new_with_model(HardwareModel::default())
    }

    pub fn new_with_model(model: HardwareModel) -> Self {
        let registers = match model {
            HardwareModel::Dmg0 => Registers {
                a: 0x01,
                f: 0x00,
                b: 0xFF,
                c: 0x13,
                d: 0x00,
                e: 0xC1,
                h: 0x84,
                l: 0x03,
                sp: 0xFFFE,
                pc: 0x0100,
            },
            HardwareModel::Dmg => Registers {
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
            },
            HardwareModel::Mgb => Registers {
                a: 0xFF,
                f: 0xB0,
                b: 0x00,
                c: 0x13,
                d: 0x00,
                e: 0xD8,
                h: 0x01,
                l: 0x4D,
                sp: 0xFFFE,
                pc: 0x0100,
            },
            HardwareModel::Sgb => Registers {
                a: 0x01,
                f: 0x00,
                b: 0x00,
                c: 0x14,
                d: 0x00,
                e: 0x00,
                h: 0xC0,
                l: 0x60,
                sp: 0xFFFE,
                pc: 0x0100,
            },
            HardwareModel::Sgb2 => Registers {
                a: 0xFF,
                f: 0x00,
                b: 0x00,
                c: 0x14,
                d: 0x00,
                e: 0x00,
                h: 0xC0,
                l: 0x60,
                sp: 0xFFFE,
                pc: 0x0100,
            },
        };

        Self {
            registers,
            ime: false,
            ime_enable_delay: 0,
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

    fn service_interrupt(&mut self, bus: &mut Bus, _pending: u8) -> u8 {
        self.ime = false;
        self.halted = false;

        // Interrupt dispatch takes 5 M-cycles on DMG:
        // 2 idle cycles, then PC high/low push, then vector/jump cycle.
        let pc = self.registers.pc;
        let pc_high = (pc >> 8) as u8;
        let pc_low = (pc & 0x00FF) as u8;

        self.tick_t(bus, 8);

        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, pc_high);

        // IE may have been changed by the upper-byte push (SP hitting $FFFF).
        // Re-evaluate pending interrupts at this point.
        let selected = Self::select_interrupt(bus.pending_interrupts());

        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, pc_low);

        if let Some((bit, vector)) = selected {
            let mut iflags = bus.interrupt_flags();
            iflags &= !(1 << bit);
            bus.set_interrupt_flags(iflags);
            self.registers.pc = vector;
        } else {
            // If IE push cancels the dispatch, execution continues from $0000.
            self.registers.pc = 0x0000;
        }

        self.tick_t(bus, 4);
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
    fn new_with_model_sets_expected_boot_registers() {
        let assert_model = |model: HardwareModel, expected: (u8, u8, u8, u8, u8, u8, u8, u8)| {
            let cpu = Cpu::new_with_model(model);
            assert_eq!(
                (
                    cpu.registers.a,
                    cpu.registers.f,
                    cpu.registers.b,
                    cpu.registers.c,
                    cpu.registers.d,
                    cpu.registers.e,
                    cpu.registers.h,
                    cpu.registers.l
                ),
                expected
            );
            assert_eq!(cpu.registers.sp, 0xFFFE);
            assert_eq!(cpu.registers.pc, 0x0100);
        };

        assert_model(
            HardwareModel::Dmg0,
            (0x01, 0x00, 0xFF, 0x13, 0x00, 0xC1, 0x84, 0x03),
        );
        assert_model(
            HardwareModel::Dmg,
            (0x01, 0xB0, 0x00, 0x13, 0x00, 0xD8, 0x01, 0x4D),
        );
        assert_model(
            HardwareModel::Mgb,
            (0xFF, 0xB0, 0x00, 0x13, 0x00, 0xD8, 0x01, 0x4D),
        );
        assert_model(
            HardwareModel::Sgb,
            (0x01, 0x00, 0x00, 0x14, 0x00, 0x00, 0xC0, 0x60),
        );
        assert_model(
            HardwareModel::Sgb2,
            (0xFF, 0x00, 0x00, 0x14, 0x00, 0x00, 0xC0, 0x60),
        );
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

    #[test]
    fn interrupt_ie_push_upper_byte_can_cancel_dispatch() {
        let mut cpu = Cpu::new();
        let mut bus = make_test_bus();

        cpu.ime = true;
        cpu.registers.pc = 0x0235;
        cpu.registers.sp = 0x0000;

        bus.write_byte(0xFFFF, 0x04);
        bus.set_interrupt_flags(0x04);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 20);
        assert!(!cpu.ime);
        assert_eq!(cpu.registers.pc, 0x0000);
        assert_eq!(bus.interrupt_enable(), 0x02);
        assert_eq!(bus.interrupt_flags() & 0x1F, 0x04);
    }

    #[test]
    fn interrupt_ie_push_upper_byte_can_change_selected_vector() {
        let mut cpu = Cpu::new();
        let mut bus = make_test_bus();

        cpu.ime = true;
        cpu.registers.pc = 0x0235;
        cpu.registers.sp = 0x0000;

        bus.write_byte(0xFFFF, 0x03);
        bus.set_interrupt_flags(0x03);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 20);
        assert_eq!(cpu.registers.pc, 0x0048);
        assert_eq!(bus.interrupt_enable(), 0x02);
        assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);
    }

    #[test]
    fn interrupt_ie_push_lower_byte_is_too_late_to_cancel_dispatch() {
        let mut cpu = Cpu::new();
        let mut bus = make_test_bus();

        cpu.ime = true;
        cpu.registers.pc = 0x0235;
        cpu.registers.sp = 0x0001;

        bus.write_byte(0xFFFF, 0x08);
        bus.set_interrupt_flags(0x08);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 20);
        assert_eq!(cpu.registers.pc, 0x0058);
        assert_eq!(bus.interrupt_enable(), 0x35);
        assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
    }
}
