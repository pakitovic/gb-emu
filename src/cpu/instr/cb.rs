use crate::cpu::{Cpu, get_flag_c, set_flag_c, set_flag_h, set_flag_n, set_flag_z};
use crate::memory::Bus;

impl Cpu {
    pub(in crate::cpu) fn read_r8_by_index(&mut self, idx: u8, bus: &mut Bus) -> u8 {
        match idx {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.read_byte(bus, self.hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        }
    }

    pub(in crate::cpu) fn write_r8_by_index(&mut self, idx: u8, value: u8, bus: &mut Bus) {
        match idx {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => self.write_byte(bus, self.hl(), value),
            7 => self.registers.a = value,
            _ => unreachable!(),
        }
    }

    pub(in crate::cpu) fn execute_cb(&mut self, bus: &mut Bus) -> u8 {
        let cb_opcode = self.fetch_d8(bus);
        let group = cb_opcode >> 6;
        let y = (cb_opcode >> 3) & 0x07;
        let z = cb_opcode & 0x07;

        match group {
            0 => {
                let value = self.read_r8_by_index(z, bus);
                let carry_in = if get_flag_c(self.registers.f) { 1 } else { 0 };
                let (result, carry_out) = match y {
                    0 => {
                        let c = (value & 0x80) != 0;
                        (value.rotate_left(1), c)
                    }
                    1 => {
                        let c = (value & 0x01) != 0;
                        (value.rotate_right(1), c)
                    }
                    2 => {
                        let c = (value & 0x80) != 0;
                        ((value << 1) | carry_in, c)
                    }
                    3 => {
                        let c = (value & 0x01) != 0;
                        ((value >> 1) | (carry_in << 7), c)
                    }
                    4 => {
                        let c = (value & 0x80) != 0;
                        (value << 1, c)
                    }
                    5 => {
                        let c = (value & 0x01) != 0;
                        ((value >> 1) | (value & 0x80), c)
                    }
                    6 => (((value & 0x0F) << 4) | ((value & 0xF0) >> 4), false),
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
            1 => {
                let value = self.read_r8_by_index(z, bus);
                let bit_set = (value & (1 << y)) != 0;
                set_flag_z(&mut self.registers.f, !bit_set);
                set_flag_n(&mut self.registers.f, false);
                set_flag_h(&mut self.registers.f, true);
                if z == 6 { 12 } else { 8 }
            }
            2 => {
                let value = self.read_r8_by_index(z, bus);
                let result = value & !(1 << y);
                self.write_r8_by_index(z, result, bus);
                if z == 6 { 16 } else { 8 }
            }
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
