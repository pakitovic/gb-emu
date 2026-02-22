use super::super::registers::ApuRegisters;
use super::super::{MAX_FREQUENCY, MAX_WAVE_LENGTH};

#[derive(Clone, Copy, Default)]
pub(in crate::apu) struct WaveChannel {
    pub(in crate::apu) enabled: bool,
    pub(in crate::apu) dac_enabled: bool,
    pub(in crate::apu) length_counter: u16,
    pub(in crate::apu) length_enabled: bool,
    pub(in crate::apu) output_level: u8,
    pub(in crate::apu) frequency: u16,
    pub(in crate::apu) frequency_timer: u16,
    pub(in crate::apu) wave_position: u8,
    pub(in crate::apu) sample_buffer: u8,
}

impl WaveChannel {
    pub(in crate::apu) fn write_dac_enable(&mut self, value: u8) {
        self.dac_enabled = (value & 0x80) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub(in crate::apu) fn write_length(&mut self, value: u8) {
        self.length_counter = MAX_WAVE_LENGTH - (value as u16);
    }

    pub(in crate::apu) fn write_output_level(&mut self, value: u8) {
        self.output_level = (value >> 5) & 0x03;
    }

    pub(in crate::apu) fn write_frequency_low(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x0700) | (value as u16);
    }

    pub(in crate::apu) fn write_frequency_high(&mut self, value: u8, length_clocks_next: bool) {
        self.frequency = (self.frequency & 0x00FF) | (((value & 0x07) as u16) << 8);
        let old_length_enabled = self.length_enabled;
        self.length_enabled = (value & 0x40) != 0;
        let trigger_requested = (value & 0x80) != 0;
        self.apply_length_enable_edge(old_length_enabled, length_clocks_next, trigger_requested);
        if trigger_requested {
            self.trigger(length_clocks_next);
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

    pub(in crate::apu) fn trigger(&mut self, length_clocks_next: bool) {
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

    pub(in crate::apu) fn period_from_frequency(&self) -> u16 {
        let freq = self.frequency.min(MAX_FREQUENCY);
        let period = (MAX_FREQUENCY + 1 - freq).saturating_mul(2);
        period.max(2)
    }

    pub(in crate::apu) fn step_tcycle(&mut self, registers: &ApuRegisters) {
        if self.frequency_timer <= 1 {
            self.frequency_timer = self.period_from_frequency();
            self.wave_position = (self.wave_position + 1) & 0x1F;
            self.sample_buffer = registers.wave_sample_byte(self.wave_position);
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

    pub(in crate::apu) fn output_amplitude(&self) -> i16 {
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
