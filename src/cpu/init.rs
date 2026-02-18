use super::Cpu;
use super::registers::Registers;
use crate::hardware::HardwareModel;

impl Cpu {
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

    pub(super) fn hl(&self) -> u16 {
        ((self.registers.h as u16) << 8) | self.registers.l as u16
    }

    pub(super) fn set_hl(&mut self, value: u16) {
        self.registers.h = (value >> 8) as u8;
        self.registers.l = (value & 0xFF) as u8;
    }

    pub(super) fn de(&self) -> u16 {
        ((self.registers.d as u16) << 8) | self.registers.e as u16
    }

    pub(super) fn set_de(&mut self, value: u16) {
        self.registers.d = (value >> 8) as u8;
        self.registers.e = (value & 0xFF) as u8;
    }

    pub(super) fn bc(&self) -> u16 {
        ((self.registers.b as u16) << 8) | self.registers.c as u16
    }

    pub(super) fn set_bc(&mut self, value: u16) {
        self.registers.b = (value >> 8) as u8;
        self.registers.c = (value & 0xFF) as u8;
    }
}
