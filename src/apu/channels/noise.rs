use super::super::{MAX_NOISE_LENGTH, NOISE_DIVISORS};
use super::EnvelopeState;
use super::common::{
    apply_length_enable_edge_u8, clock_length_u8, reload_length_on_trigger_u8,
    write_envelope_and_update_dac_state,
};

#[derive(Clone, Copy, Default)]
pub(in crate::apu) struct NoiseChannel {
    pub(in crate::apu) enabled: bool,
    pub(in crate::apu) dac_enabled: bool,
    pub(in crate::apu) length_counter: u8,
    pub(in crate::apu) length_enabled: bool,
    pub(in crate::apu) envelope: EnvelopeState,
    pub(in crate::apu) clock_shift: u8,
    pub(in crate::apu) width_mode_7bit: bool,
    pub(in crate::apu) divisor_code: u8,
    pub(in crate::apu) frequency_timer: u16,
    pub(in crate::apu) lfsr: u16,
}

impl NoiseChannel {
    pub(in crate::apu) fn write_length(&mut self, value: u8) {
        self.length_counter = MAX_NOISE_LENGTH - (value & 0x3F);
    }

    pub(in crate::apu) fn write_envelope(&mut self, value: u8) {
        write_envelope_and_update_dac_state(
            &mut self.envelope,
            &mut self.enabled,
            &mut self.dac_enabled,
            value,
        );
    }

    pub(in crate::apu) fn write_polynomial(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & 0x0F;
        self.width_mode_7bit = (value & 0x08) != 0;
        self.divisor_code = value & 0x07;
    }

    pub(in crate::apu) fn write_control(
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

    pub(in crate::apu) fn apply_length_enable_edge(
        &mut self,
        old_length_enabled: bool,
        length_clocks_next: bool,
        trigger_requested: bool,
    ) {
        apply_length_enable_edge_u8(
            &mut self.enabled,
            &mut self.length_counter,
            self.length_enabled,
            old_length_enabled,
            length_clocks_next,
            trigger_requested,
        );
    }

    pub(in crate::apu) fn trigger(&mut self, length_clocks_next: bool, envelope_clocks_next: bool) {
        if !self.dac_enabled {
            self.enabled = false;
            return;
        }

        self.enabled = true;
        reload_length_on_trigger_u8(
            &mut self.length_counter,
            self.length_enabled,
            length_clocks_next,
            MAX_NOISE_LENGTH,
        );
        self.frequency_timer = self.period_from_registers();
        self.lfsr = 0x7FFF;
        self.envelope.trigger(envelope_clocks_next);
    }

    pub(in crate::apu) fn period_from_registers(&self) -> u16 {
        let divisor = NOISE_DIVISORS[self.divisor_code as usize] as u32;
        let period = divisor << self.clock_shift;
        period.min(u16::MAX as u32) as u16
    }

    pub(in crate::apu) fn step_tcycle(&mut self) {
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

    pub(in crate::apu) fn clock_length(&mut self) {
        clock_length_u8(
            &mut self.enabled,
            &mut self.length_counter,
            self.length_enabled,
        );
    }

    pub(in crate::apu) fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    pub(in crate::apu) fn output_amplitude(&self) -> i16 {
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
