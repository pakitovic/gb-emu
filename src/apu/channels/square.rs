use super::super::{DUTY_PATTERNS, MAX_FREQUENCY, MAX_SQUARE_LENGTH};
use super::{EnvelopeState, SweepState};

#[derive(Clone, Copy, Default)]
pub(in crate::apu) struct SquareChannel {
    pub(in crate::apu) enabled: bool,
    pub(in crate::apu) dac_enabled: bool,
    pub(in crate::apu) duty: u8,
    pub(in crate::apu) duty_position: u8,
    pub(in crate::apu) length_counter: u8,
    pub(in crate::apu) length_enabled: bool,
    pub(in crate::apu) frequency: u16,
    pub(in crate::apu) frequency_timer: u16,
    pub(in crate::apu) envelope: EnvelopeState,
    pub(in crate::apu) sweep: SweepState,
    pub(in crate::apu) has_sweep: bool,
}

impl SquareChannel {
    pub(in crate::apu) fn with_sweep() -> Self {
        Self {
            has_sweep: true,
            ..Self::default()
        }
    }

    pub(in crate::apu) fn reset(&mut self, has_sweep: bool) {
        *self = if has_sweep {
            Self::with_sweep()
        } else {
            Self::default()
        };
    }

    pub(in crate::apu) fn write_duty_length(&mut self, value: u8) {
        self.duty = (value >> 6) & 0x03;
        self.length_counter = MAX_SQUARE_LENGTH - (value & 0x3F);
    }

    pub(in crate::apu) fn write_envelope(&mut self, value: u8) {
        self.envelope.write_register(value);
        self.dac_enabled = (value & 0xF8) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub(in crate::apu) fn write_frequency_low(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x0700) | (value as u16);
    }

    pub(in crate::apu) fn write_frequency_high(
        &mut self,
        value: u8,
        length_clocks_next: bool,
        envelope_clocks_next: bool,
    ) {
        self.frequency = (self.frequency & 0x00FF) | (((value & 0x07) as u16) << 8);
        let old_length_enabled = self.length_enabled;
        self.length_enabled = (value & 0x40) != 0;
        let trigger_requested = (value & 0x80) != 0;
        self.apply_length_enable_edge(old_length_enabled, length_clocks_next, trigger_requested);
        if trigger_requested {
            self.trigger(length_clocks_next, envelope_clocks_next);
        }
    }

    pub(in crate::apu) fn apply_length_enable_edge(
        &mut self,
        old_length_enabled: bool,
        length_clocks_next: bool,
        trigger_requested: bool,
    ) {
        if old_length_enabled
            || !self.length_enabled
            || length_clocks_next
            || self.length_counter == 0
        {
            return;
        }
        self.length_counter -= 1;
        if self.length_counter == 0 && !trigger_requested {
            self.enabled = false;
        }
    }

    pub(in crate::apu) fn trigger(&mut self, length_clocks_next: bool, envelope_clocks_next: bool) {
        if !self.dac_enabled {
            self.enabled = false;
            return;
        }

        self.enabled = true;
        if self.length_counter == 0 {
            self.length_counter = MAX_SQUARE_LENGTH;
            if self.length_enabled && !length_clocks_next {
                self.length_counter = self.length_counter.saturating_sub(1);
            }
        }
        self.frequency_timer = self.period_from_frequency();
        self.duty_position = 0;
        self.envelope.trigger(envelope_clocks_next);
        if self.has_sweep {
            self.sweep.trigger(self.frequency);
            if self.sweep.shift > 0
                && self
                    .sweep
                    .calculate_next_frequency_tracking()
                    .is_none_or(|next| next > MAX_FREQUENCY)
            {
                self.enabled = false;
            }
        }
    }

    pub(in crate::apu) fn period_from_frequency(&self) -> u16 {
        let freq = self.frequency.min(MAX_FREQUENCY);
        let period = (MAX_FREQUENCY + 1 - freq).saturating_mul(4);
        period.max(4)
    }

    pub(in crate::apu) fn step_tcycle(&mut self) {
        if self.frequency_timer <= 1 {
            self.frequency_timer = self.period_from_frequency();
            self.duty_position = (self.duty_position + 1) & 0x07;
        } else {
            self.frequency_timer -= 1;
        }
    }

    pub(in crate::apu) fn clock_length(&mut self) {
        if !self.length_enabled || self.length_counter == 0 {
            return;
        }

        self.length_counter -= 1;
        if self.length_counter == 0 {
            self.enabled = false;
        }
    }

    pub(in crate::apu) fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    pub(in crate::apu) fn clock_sweep(&mut self) {
        if !self.has_sweep {
            return;
        }
        if !self.sweep.clock_timer_and_should_step() {
            return;
        }

        let Some(next_frequency) = self.sweep.calculate_next_frequency_tracking() else {
            self.enabled = false;
            return;
        };
        if next_frequency > MAX_FREQUENCY {
            self.enabled = false;
            return;
        }

        self.sweep.shadow_frequency = next_frequency;
        self.frequency = next_frequency;

        if self
            .sweep
            .calculate_next_frequency_tracking()
            .is_none_or(|future| future > MAX_FREQUENCY)
        {
            self.enabled = false;
        }
    }

    pub(in crate::apu) fn output_amplitude(&self) -> i16 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }
        let bit = DUTY_PATTERNS[self.duty as usize][self.duty_position as usize];
        let volume = self.envelope.volume as i16;
        if bit == 0 { -volume } else { volume }
    }
}
