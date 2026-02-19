use super::Bus;

#[derive(Default)]
pub(super) struct TimerState {
    pub(super) div_counter: u16,
    pub(super) tima_reload_delay: u8,
    pub(super) tima_reload_block: u8,
}

impl Bus {
    pub fn tick(&mut self, cycles: u8) {
        for _ in 0..cycles {
            self.step_tima_reload();
            self.step_ppu();
            self.step_oam_dma();

            let old_div = self.timer.div_counter;
            let old_input = self.timer_input_high();
            self.timer.div_counter = self.timer.div_counter.wrapping_add(1);
            let new_input = self.timer_input_high();
            self.step_serial(old_div, self.timer.div_counter);

            if old_input && !new_input {
                self.increment_tima();
            }

            if self.timer.tima_reload_block > 0 {
                self.timer.tima_reload_block -= 1;
            }
        }
    }

    pub(super) fn timer_input_high(&self) -> bool {
        let tac = self.io[0x07];
        if (tac & 0x04) == 0 {
            return false;
        }

        let bit = match tac & 0x03 {
            0x00 => 9, // 4096 Hz
            0x01 => 3, // 262144 Hz
            0x02 => 5, // 65536 Hz
            0x03 => 7, // 16384 Hz
            _ => unreachable!(),
        };

        ((self.timer.div_counter >> bit) & 1) != 0
    }

    pub(super) fn increment_tima(&mut self) {
        if self.timer.tima_reload_delay != 0 {
            return;
        }

        let (next_tima, overflow) = self.io[0x05].overflowing_add(1);
        if overflow {
            self.io[0x05] = 0x00;
            self.timer.tima_reload_delay = 4;
        } else {
            self.io[0x05] = next_tima;
        }
    }

    fn step_tima_reload(&mut self) {
        if self.timer.tima_reload_delay == 0 {
            return;
        }

        self.timer.tima_reload_delay -= 1;
        if self.timer.tima_reload_delay == 0 {
            self.io[0x05] = self.io[0x06];
            let iflags = self.interrupt_flags() | (1 << 2);
            self.set_interrupt_flags(iflags);
            self.timer.tima_reload_block = 4;
        }
    }
}
