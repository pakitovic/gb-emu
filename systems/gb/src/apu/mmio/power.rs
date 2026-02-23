use super::super::*;

impl ApuState {
    pub(super) fn write_nr52_power(&mut self, io: &mut [u8; 0x80], value: u8) {
        let request_enabled = (value & 0x80) != 0;

        if self.enabled && !request_enabled {
            self.clear_apu_register_window(io);
            self.reset_after_power_toggle(false);
            io[NR52_INDEX] = 0x00;
            return;
        }

        if !self.enabled && request_enabled {
            self.clear_apu_register_window(io);
            self.reset_after_power_toggle(true);
            io[NR52_INDEX] = 0x80;
            return;
        }

        if self.enabled {
            io[NR52_INDEX] = 0x80 | (self.channel_on_mask & 0x0F);
        } else {
            io[NR52_INDEX] = 0x00;
        }
    }

    fn clear_apu_register_window(&mut self, io: &mut [u8; 0x80]) {
        self.registers.clear_nr_window();
        for register in io.iter_mut().take(NR51_INDEX + 1).skip(NR10_INDEX) {
            *register = 0x00;
        }
    }
}
