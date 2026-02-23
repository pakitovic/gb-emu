use crate::cpu::{Cpu, get_flag_c, set_flag_c, set_flag_h, set_flag_n, set_flag_z};

impl Cpu {
    pub(in crate::cpu) fn inc_r(&mut self, value: u8) -> u8 {
        let old = value;
        let result = old.wrapping_add(1);

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, (old & 0x0F) + 1 > 0x0F);

        result
    }

    pub(in crate::cpu) fn dec_r(&mut self, value: u8) -> u8 {
        let old = value;
        let result = old.wrapping_sub(1);

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (old & 0x0F) == 0);

        result
    }

    pub(in crate::cpu) fn add_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let result = a.wrapping_add(value);
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, (a & 0x0F) + (value & 0x0F) > 0x0F);
        set_flag_c(&mut self.registers.f, (a as u16 + value as u16) > 0xFF);

        4
    }

    pub(in crate::cpu) fn adc_a(&mut self, value: u8) -> u8 {
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

    pub(in crate::cpu) fn sub_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (a & 0x0F) < (value & 0x0F));
        set_flag_c(&mut self.registers.f, a < value);

        4
    }

    pub(in crate::cpu) fn sbc_a(&mut self, value: u8) -> u8 {
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

    pub(in crate::cpu) fn and_a(&mut self, value: u8) -> u8 {
        let result = self.registers.a & value;
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, true);
        set_flag_c(&mut self.registers.f, false);

        4
    }

    pub(in crate::cpu) fn or_a(&mut self, value: u8) -> u8 {
        let result = self.registers.a | value;
        self.registers.a = result;

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, false);
        set_flag_h(&mut self.registers.f, false);
        set_flag_c(&mut self.registers.f, false);

        4
    }

    pub(in crate::cpu) fn cp_a(&mut self, value: u8) -> u8 {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);

        set_flag_z(&mut self.registers.f, result == 0);
        set_flag_n(&mut self.registers.f, true);
        set_flag_h(&mut self.registers.f, (a & 0x0F) < (value & 0x0F));
        set_flag_c(&mut self.registers.f, a < value);

        4
    }

    pub(in crate::cpu) fn add_hl(&mut self, value: u16) -> u8 {
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
}
