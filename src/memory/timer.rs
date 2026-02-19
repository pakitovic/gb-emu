use super::Bus;

#[derive(Default)]
pub(super) struct TimerState {
    pub(super) div_counter: u16,
    pub(super) tima_reload_delay: u8,
    pub(super) tima_reload_block: u8,
}

impl TimerState {
    pub(super) fn read_div(bus: &Bus) -> u8 {
        (bus.timer.div_counter >> 8) as u8
    }

    pub(super) fn write_div(bus: &mut Bus, value: u8) {
        let _ = value;
        let old_input = bus.timer_input_high();
        bus.timer.div_counter = 0;
        let new_input = bus.timer_input_high();
        if old_input && !new_input {
            bus.increment_tima();
        }
    }

    pub(super) fn write_tac(bus: &mut Bus, value: u8) {
        let old_input = bus.timer_input_high();
        bus.io[0x07] = value;
        let new_input = bus.timer_input_high();
        if old_input && !new_input {
            bus.increment_tima();
        }
    }

    pub(super) fn write_tima(bus: &mut Bus, value: u8) {
        if bus.timer.tima_reload_block > 0 {
            // ignored
        } else if bus.timer.tima_reload_delay > 0 {
            bus.io[0x05] = value;
            bus.timer.tima_reload_delay = 0;
        } else {
            bus.io[0x05] = value;
        }
    }

    pub(super) fn write_tma(bus: &mut Bus, value: u8) {
        bus.io[0x06] = value;
        if bus.timer.tima_reload_block > 0 {
            bus.io[0x05] = value;
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct TimerDividerStep {
    pub(super) old_div: u16,
    pub(super) new_div: u16,
    old_input: bool,
    new_input: bool,
}

impl TimerDividerStep {
    fn had_falling_edge(self) -> bool {
        self.old_input && !self.new_input
    }
}

impl Bus {
    pub(super) fn read_div(&self) -> u8 {
        TimerState::read_div(self)
    }

    pub(super) fn write_div(&mut self, value: u8) {
        TimerState::write_div(self, value);
    }

    pub(super) fn write_tac(&mut self, value: u8) {
        TimerState::write_tac(self, value);
    }

    pub(super) fn write_tima(&mut self, value: u8) {
        TimerState::write_tima(self, value);
    }

    pub(super) fn write_tma(&mut self, value: u8) {
        TimerState::write_tma(self, value);
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

    pub(super) fn step_timer_divider(&mut self) -> TimerDividerStep {
        let old_div = self.timer.div_counter;
        let old_input = self.timer_input_high();
        self.timer.div_counter = self.timer.div_counter.wrapping_add(1);
        let new_div = self.timer.div_counter;
        let new_input = self.timer_input_high();
        TimerDividerStep {
            old_div,
            new_div,
            old_input,
            new_input,
        }
    }

    pub(super) fn step_timer_falling_edge(&mut self, divider_step: TimerDividerStep) {
        if divider_step.had_falling_edge() {
            self.increment_tima();
        }
    }

    pub(super) fn step_tima_reload_block(&mut self) {
        if self.timer.tima_reload_block > 0 {
            self.timer.tima_reload_block -= 1;
        }
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

    pub(super) fn step_tima_reload(&mut self) {
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
