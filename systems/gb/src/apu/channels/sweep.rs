use super::super::MAX_FREQUENCY;

#[derive(Clone, Copy, Default)]
pub(in crate::apu) struct SweepState {
    pub(in crate::apu) period: u8,
    pub(in crate::apu) negate: bool,
    pub(in crate::apu) shift: u8,
    pub(in crate::apu) timer: u8,
    pub(in crate::apu) enabled: bool,
    pub(in crate::apu) shadow_frequency: u16,
    pub(in crate::apu) subtraction_since_trigger: bool,
}

impl SweepState {
    pub(in crate::apu) fn write_register(&mut self, value: u8) -> bool {
        let old_negate = self.negate;
        self.period = (value >> 4) & 0x07;
        self.negate = (value & 0x08) != 0;
        self.shift = value & 0x07;
        old_negate && !self.negate && self.subtraction_since_trigger
    }

    pub(in crate::apu) fn trigger(&mut self, frequency: u16) {
        self.shadow_frequency = frequency.min(MAX_FREQUENCY);
        self.timer = if self.period == 0 { 8 } else { self.period };
        self.enabled = self.period != 0 || self.shift != 0;
        self.subtraction_since_trigger = false;
    }

    pub(in crate::apu) fn clock_timer_and_should_step(&mut self) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer > 0 {
            return false;
        }
        self.timer = if self.period == 0 { 8 } else { self.period };
        self.enabled && self.shift > 0
    }

    pub(in crate::apu) fn calculate_next_frequency(&self) -> Option<u16> {
        let delta = self.shadow_frequency >> self.shift;
        if self.negate {
            self.shadow_frequency.checked_sub(delta)
        } else {
            self.shadow_frequency.checked_add(delta)
        }
    }

    pub(in crate::apu) fn calculate_next_frequency_tracking(&mut self) -> Option<u16> {
        if self.negate {
            self.subtraction_since_trigger = true;
        }
        self.calculate_next_frequency()
    }
}
