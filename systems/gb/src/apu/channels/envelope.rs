#[derive(Clone, Copy, Default)]
pub(in crate::apu) struct EnvelopeState {
    pub(in crate::apu) initial_volume: u8,
    pub(in crate::apu) volume: u8,
    pub(in crate::apu) period: u8,
    pub(in crate::apu) increase: bool,
    pub(in crate::apu) timer: u8,
}

impl EnvelopeState {
    pub(in crate::apu) fn apply_zombie_mode_on_write(&mut self, new_value: u8) {
        let old_period_was_zero = self.period == 0;
        let old_increase = self.increase;
        let new_increase = (new_value & 0x08) != 0;

        let mut volume = self.volume;
        if old_period_was_zero && self.timer != 0 {
            volume = volume.wrapping_add(1);
        } else if !old_increase {
            volume = volume.wrapping_add(2);
        }

        if old_increase != new_increase {
            volume = 16u8.wrapping_sub(volume);
        }

        self.volume = volume & 0x0F;
    }

    pub(in crate::apu) fn write_register(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.period = value & 0x07;
        self.increase = (value & 0x08) != 0;
    }

    pub(in crate::apu) fn trigger(&mut self, envelope_clocks_next: bool) {
        self.volume = self.initial_volume;
        let base_timer = if self.period == 0 { 8 } else { self.period };
        self.timer = if envelope_clocks_next {
            base_timer.saturating_add(1)
        } else {
            base_timer
        };
    }

    pub(in crate::apu) fn clock(&mut self) {
        if self.period == 0 {
            return;
        }

        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer > 0 {
            return;
        }

        self.timer = self.period;
        if self.increase {
            if self.volume < 15 {
                self.volume += 1;
            }
        } else if self.volume > 0 {
            self.volume -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombie_mode_period_zero_increment_wraps_low_nibble() {
        let mut envelope = EnvelopeState {
            volume: 0x0F,
            period: 0,
            increase: true,
            timer: 8,
            ..EnvelopeState::default()
        };

        envelope.apply_zombie_mode_on_write(0x08);

        assert_eq!(envelope.volume, 0x00);
    }

    #[test]
    fn zombie_mode_decrease_write_then_direction_flip_reflects_volume() {
        let mut envelope = EnvelopeState {
            volume: 5,
            period: 3,
            increase: false,
            timer: 2,
            ..EnvelopeState::default()
        };

        envelope.apply_zombie_mode_on_write(0x09); // period=1, increase mode

        assert_eq!(envelope.volume, 9);
    }

    #[test]
    fn zombie_mode_period_zero_without_running_timer_does_not_increment() {
        let mut envelope = EnvelopeState {
            volume: 4,
            period: 0,
            increase: true,
            timer: 0,
            ..EnvelopeState::default()
        };

        envelope.apply_zombie_mode_on_write(0x08);

        assert_eq!(envelope.volume, 4);
    }
}
