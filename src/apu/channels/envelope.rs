#[derive(Clone, Copy, Default)]
pub(in crate::apu) struct EnvelopeState {
    pub(in crate::apu) initial_volume: u8,
    pub(in crate::apu) volume: u8,
    pub(in crate::apu) period: u8,
    pub(in crate::apu) increase: bool,
    pub(in crate::apu) timer: u8,
}

impl EnvelopeState {
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
