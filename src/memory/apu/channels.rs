use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct EnvelopeState {
    pub(super) initial_volume: u8,
    pub(super) volume: u8,
    pub(super) period: u8,
    pub(super) increase: bool,
    pub(super) timer: u8,
}

impl EnvelopeState {
    pub(super) fn write_register(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.period = value & 0x07;
        self.increase = (value & 0x08) != 0;
    }

    pub(super) fn trigger(&mut self, envelope_clocks_next: bool) {
        self.volume = self.initial_volume;
        let base_timer = if self.period == 0 { 8 } else { self.period };
        self.timer = if envelope_clocks_next {
            base_timer.saturating_add(1)
        } else {
            base_timer
        };
    }

    pub(super) fn clock(&mut self) {
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

#[derive(Clone, Copy, Default)]
pub(super) struct SweepState {
    pub(super) period: u8,
    pub(super) negate: bool,
    pub(super) shift: u8,
    pub(super) timer: u8,
    pub(super) enabled: bool,
    pub(super) shadow_frequency: u16,
    pub(super) subtraction_since_trigger: bool,
}

impl SweepState {
    pub(super) fn write_register(&mut self, value: u8) -> bool {
        let old_negate = self.negate;
        self.period = (value >> 4) & 0x07;
        self.negate = (value & 0x08) != 0;
        self.shift = value & 0x07;
        old_negate && !self.negate && self.subtraction_since_trigger
    }

    pub(super) fn trigger(&mut self, frequency: u16) {
        self.shadow_frequency = frequency.min(MAX_FREQUENCY);
        self.timer = if self.period == 0 { 8 } else { self.period };
        self.enabled = self.period != 0 || self.shift != 0;
        self.subtraction_since_trigger = false;
    }

    pub(super) fn clock_timer_and_should_step(&mut self) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer > 0 {
            return false;
        }
        self.timer = if self.period == 0 { 8 } else { self.period };
        self.enabled && self.shift > 0
    }

    pub(super) fn calculate_next_frequency(&self) -> Option<u16> {
        let delta = self.shadow_frequency >> self.shift;
        if self.negate {
            self.shadow_frequency.checked_sub(delta)
        } else {
            self.shadow_frequency.checked_add(delta)
        }
    }

    pub(super) fn calculate_next_frequency_tracking(&mut self) -> Option<u16> {
        if self.negate {
            self.subtraction_since_trigger = true;
        }
        self.calculate_next_frequency()
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct SquareChannel {
    pub(super) enabled: bool,
    pub(super) dac_enabled: bool,
    pub(super) duty: u8,
    pub(super) duty_position: u8,
    pub(super) length_counter: u8,
    pub(super) length_enabled: bool,
    pub(super) frequency: u16,
    pub(super) frequency_timer: u16,
    pub(super) envelope: EnvelopeState,
    pub(super) sweep: SweepState,
    pub(super) has_sweep: bool,
}

impl SquareChannel {
    pub(super) fn with_sweep() -> Self {
        Self {
            has_sweep: true,
            ..Self::default()
        }
    }

    pub(super) fn reset(&mut self, has_sweep: bool) {
        *self = if has_sweep {
            Self::with_sweep()
        } else {
            Self::default()
        };
    }

    pub(super) fn write_duty_length(&mut self, value: u8) {
        self.duty = (value >> 6) & 0x03;
        self.length_counter = MAX_SQUARE_LENGTH - (value & 0x3F);
    }

    pub(super) fn write_envelope(&mut self, value: u8) {
        self.envelope.write_register(value);
        self.dac_enabled = (value & 0xF8) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub(super) fn write_frequency_low(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x0700) | (value as u16);
    }

    pub(super) fn write_frequency_high(
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

    pub(super) fn apply_length_enable_edge(
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

    pub(super) fn trigger(&mut self, length_clocks_next: bool, envelope_clocks_next: bool) {
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

    pub(super) fn period_from_frequency(&self) -> u16 {
        let freq = self.frequency.min(MAX_FREQUENCY);
        let period = (MAX_FREQUENCY + 1 - freq).saturating_mul(4);
        period.max(4)
    }

    pub(super) fn step_tcycle(&mut self) {
        if self.frequency_timer <= 1 {
            self.frequency_timer = self.period_from_frequency();
            self.duty_position = (self.duty_position + 1) & 0x07;
        } else {
            self.frequency_timer -= 1;
        }
    }

    pub(super) fn clock_length(&mut self) {
        if !self.length_enabled || self.length_counter == 0 {
            return;
        }

        self.length_counter -= 1;
        if self.length_counter == 0 {
            self.enabled = false;
        }
    }

    pub(super) fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    pub(super) fn clock_sweep(&mut self) {
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

    pub(super) fn output_amplitude(&self) -> i16 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }
        let bit = DUTY_PATTERNS[self.duty as usize][self.duty_position as usize];
        let volume = self.envelope.volume as i16;
        if bit == 0 { -volume } else { volume }
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct WaveChannel {
    pub(super) enabled: bool,
    pub(super) dac_enabled: bool,
    pub(super) length_counter: u16,
    pub(super) length_enabled: bool,
    pub(super) output_level: u8,
    pub(super) frequency: u16,
    pub(super) frequency_timer: u16,
    pub(super) wave_position: u8,
    pub(super) sample_buffer: u8,
}

impl WaveChannel {
    pub(super) fn write_dac_enable(&mut self, value: u8) {
        self.dac_enabled = (value & 0x80) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub(super) fn write_length(&mut self, value: u8) {
        self.length_counter = MAX_WAVE_LENGTH - (value as u16);
    }

    pub(super) fn write_output_level(&mut self, value: u8) {
        self.output_level = (value >> 5) & 0x03;
    }

    pub(super) fn write_frequency_low(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x0700) | (value as u16);
    }

    pub(super) fn write_frequency_high(&mut self, value: u8, length_clocks_next: bool) {
        self.frequency = (self.frequency & 0x00FF) | (((value & 0x07) as u16) << 8);
        let old_length_enabled = self.length_enabled;
        self.length_enabled = (value & 0x40) != 0;
        let trigger_requested = (value & 0x80) != 0;
        self.apply_length_enable_edge(old_length_enabled, length_clocks_next, trigger_requested);
        if trigger_requested {
            self.trigger(length_clocks_next);
        }
    }

    pub(super) fn apply_length_enable_edge(
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

    pub(super) fn trigger(&mut self, length_clocks_next: bool) {
        if !self.dac_enabled {
            self.enabled = false;
            return;
        }

        self.enabled = true;
        if self.length_counter == 0 {
            self.length_counter = MAX_WAVE_LENGTH;
            if self.length_enabled && !length_clocks_next {
                self.length_counter = self.length_counter.saturating_sub(1);
            }
        }
        self.frequency_timer = self.period_from_frequency();
        self.wave_position = 0;
    }

    pub(super) fn period_from_frequency(&self) -> u16 {
        let freq = self.frequency.min(MAX_FREQUENCY);
        let period = (MAX_FREQUENCY + 1 - freq).saturating_mul(2);
        period.max(2)
    }

    pub(super) fn step_tcycle(&mut self, io: &[u8; 0x80]) {
        if self.frequency_timer <= 1 {
            self.frequency_timer = self.period_from_frequency();
            self.wave_position = (self.wave_position + 1) & 0x1F;
            self.sample_buffer = io[WAVE_RAM_START_INDEX + (self.wave_position as usize / 2)];
        } else {
            self.frequency_timer -= 1;
        }
    }

    pub(super) fn clock_length(&mut self) {
        if !self.length_enabled || self.length_counter == 0 {
            return;
        }

        self.length_counter -= 1;
        if self.length_counter == 0 {
            self.enabled = false;
        }
    }

    pub(super) fn output_amplitude(&self, _io: &[u8; 0x80]) -> i16 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }

        let byte = self.sample_buffer;
        let nibble = if (self.wave_position & 1) == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        };
        let sample = match self.output_level {
            0 => 0,
            1 => nibble,
            2 => nibble >> 1,
            3 => nibble >> 2,
            _ => 0,
        };
        if self.output_level == 0 {
            0
        } else {
            (sample as i16) * 2 - 15
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct NoiseChannel {
    pub(super) enabled: bool,
    pub(super) dac_enabled: bool,
    pub(super) length_counter: u8,
    pub(super) length_enabled: bool,
    pub(super) envelope: EnvelopeState,
    pub(super) clock_shift: u8,
    pub(super) width_mode_7bit: bool,
    pub(super) divisor_code: u8,
    pub(super) frequency_timer: u16,
    pub(super) lfsr: u16,
}

impl NoiseChannel {
    pub(super) fn write_length(&mut self, value: u8) {
        self.length_counter = MAX_NOISE_LENGTH - (value & 0x3F);
    }

    pub(super) fn write_envelope(&mut self, value: u8) {
        self.envelope.write_register(value);
        self.dac_enabled = (value & 0xF8) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub(super) fn write_polynomial(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & 0x0F;
        self.width_mode_7bit = (value & 0x08) != 0;
        self.divisor_code = value & 0x07;
    }

    pub(super) fn write_control(
        &mut self,
        value: u8,
        length_clocks_next: bool,
        envelope_clocks_next: bool,
    ) {
        let old_length_enabled = self.length_enabled;
        self.length_enabled = (value & 0x40) != 0;
        let trigger_requested = (value & 0x80) != 0;
        self.apply_length_enable_edge(old_length_enabled, length_clocks_next, trigger_requested);
        if trigger_requested {
            self.trigger(length_clocks_next, envelope_clocks_next);
        }
    }

    pub(super) fn apply_length_enable_edge(
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

    pub(super) fn trigger(&mut self, length_clocks_next: bool, envelope_clocks_next: bool) {
        if !self.dac_enabled {
            self.enabled = false;
            return;
        }

        self.enabled = true;
        if self.length_counter == 0 {
            self.length_counter = MAX_NOISE_LENGTH;
            if self.length_enabled && !length_clocks_next {
                self.length_counter = self.length_counter.saturating_sub(1);
            }
        }
        self.frequency_timer = self.period_from_registers();
        self.lfsr = 0x7FFF;
        self.envelope.trigger(envelope_clocks_next);
    }

    pub(super) fn period_from_registers(&self) -> u16 {
        let divisor = NOISE_DIVISORS[self.divisor_code as usize] as u32;
        let period = divisor << self.clock_shift;
        period.min(u16::MAX as u32) as u16
    }

    pub(super) fn step_tcycle(&mut self) {
        if self.frequency_timer <= 1 {
            self.frequency_timer = self.period_from_registers();
            if self.clock_shift >= 14 {
                return;
            }
            let xor_bit = ((self.lfsr & 0x0001) ^ ((self.lfsr >> 1) & 0x0001)) & 0x0001;
            self.lfsr = (self.lfsr >> 1) | (xor_bit << 14);
            if self.width_mode_7bit {
                self.lfsr = (self.lfsr & !(1 << 6)) | (xor_bit << 6);
            }
        } else {
            self.frequency_timer -= 1;
        }
    }

    pub(super) fn clock_length(&mut self) {
        if !self.length_enabled || self.length_counter == 0 {
            return;
        }

        self.length_counter -= 1;
        if self.length_counter == 0 {
            self.enabled = false;
        }
    }

    pub(super) fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    pub(super) fn output_amplitude(&self) -> i16 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }
        let volume = self.envelope.volume as i16;
        if (self.lfsr & 0x0001) == 0 {
            volume
        } else {
            -volume
        }
    }
}
