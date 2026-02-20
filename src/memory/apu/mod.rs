use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

mod bus;
mod core;

const NR10_INDEX: usize = 0x10;
const NR11_INDEX: usize = 0x11;
const NR12_INDEX: usize = 0x12;
const NR13_INDEX: usize = 0x13;
const NR14_INDEX: usize = 0x14;
const NR21_INDEX: usize = 0x16;
const NR22_INDEX: usize = 0x17;
const NR23_INDEX: usize = 0x18;
const NR24_INDEX: usize = 0x19;
const NR30_INDEX: usize = 0x1A;
const NR31_INDEX: usize = 0x1B;
const NR32_INDEX: usize = 0x1C;
const NR33_INDEX: usize = 0x1D;
const NR34_INDEX: usize = 0x1E;
const NR41_INDEX: usize = 0x20;
const NR42_INDEX: usize = 0x21;
const NR43_INDEX: usize = 0x22;
const NR44_INDEX: usize = 0x23;
const NR50_INDEX: usize = 0x24;
const NR51_INDEX: usize = 0x25;
const NR52_INDEX: usize = 0x26;
const WAVE_RAM_START_INDEX: usize = 0x30;
const WAVE_RAM_END_INDEX: usize = 0x3F;
const MAX_PENDING_AUDIO_TCYCLE_FRAMES: usize = 262_144;

const DIV_APU_BIT: u16 = 1 << 12;
const CHANNEL_COUNT: usize = 4;
const MAX_SQUARE_LENGTH: u8 = 64;
const MAX_NOISE_LENGTH: u8 = 64;
const MAX_WAVE_LENGTH: u16 = 256;
const MAX_FREQUENCY: u16 = 2_047;

const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

const NOISE_DIVISORS: [u16; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

#[derive(Clone, Copy, Default)]
struct EnvelopeState {
    initial_volume: u8,
    volume: u8,
    period: u8,
    increase: bool,
    timer: u8,
}

impl EnvelopeState {
    fn write_register(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.period = value & 0x07;
        self.increase = (value & 0x08) != 0;
    }

    fn trigger(&mut self, envelope_clocks_next: bool) {
        self.volume = self.initial_volume;
        let base_timer = if self.period == 0 { 8 } else { self.period };
        self.timer = if envelope_clocks_next {
            base_timer.saturating_add(1)
        } else {
            base_timer
        };
    }

    fn clock(&mut self) {
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
struct SweepState {
    period: u8,
    negate: bool,
    shift: u8,
    timer: u8,
    enabled: bool,
    shadow_frequency: u16,
    subtraction_since_trigger: bool,
}

impl SweepState {
    fn write_register(&mut self, value: u8) -> bool {
        let old_negate = self.negate;
        self.period = (value >> 4) & 0x07;
        self.negate = (value & 0x08) != 0;
        self.shift = value & 0x07;
        old_negate && !self.negate && self.subtraction_since_trigger
    }

    fn trigger(&mut self, frequency: u16) {
        self.shadow_frequency = frequency.min(MAX_FREQUENCY);
        self.timer = if self.period == 0 { 8 } else { self.period };
        self.enabled = self.period != 0 || self.shift != 0;
        self.subtraction_since_trigger = false;
    }

    fn clock_timer_and_should_step(&mut self) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer > 0 {
            return false;
        }
        self.timer = if self.period == 0 { 8 } else { self.period };
        self.enabled && self.shift > 0
    }

    fn calculate_next_frequency(&self) -> Option<u16> {
        let delta = self.shadow_frequency >> self.shift;
        if self.negate {
            self.shadow_frequency.checked_sub(delta)
        } else {
            self.shadow_frequency.checked_add(delta)
        }
    }

    fn calculate_next_frequency_tracking(&mut self) -> Option<u16> {
        if self.negate {
            self.subtraction_since_trigger = true;
        }
        self.calculate_next_frequency()
    }
}

#[derive(Clone, Copy, Default)]
struct SquareChannel {
    enabled: bool,
    dac_enabled: bool,
    duty: u8,
    duty_position: u8,
    length_counter: u8,
    length_enabled: bool,
    frequency: u16,
    frequency_timer: u16,
    envelope: EnvelopeState,
    sweep: SweepState,
    has_sweep: bool,
}

impl SquareChannel {
    fn with_sweep() -> Self {
        Self {
            has_sweep: true,
            ..Self::default()
        }
    }

    fn reset(&mut self, has_sweep: bool) {
        *self = if has_sweep {
            Self::with_sweep()
        } else {
            Self::default()
        };
    }

    fn write_duty_length(&mut self, value: u8) {
        self.duty = (value >> 6) & 0x03;
        self.length_counter = MAX_SQUARE_LENGTH - (value & 0x3F);
    }

    fn write_envelope(&mut self, value: u8) {
        self.envelope.write_register(value);
        self.dac_enabled = (value & 0xF8) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    fn write_frequency_low(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x0700) | (value as u16);
    }

    fn write_frequency_high(
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

    fn apply_length_enable_edge(
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

    fn trigger(&mut self, length_clocks_next: bool, envelope_clocks_next: bool) {
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

    fn period_from_frequency(&self) -> u16 {
        let freq = self.frequency.min(MAX_FREQUENCY);
        let period = (MAX_FREQUENCY + 1 - freq).saturating_mul(4);
        period.max(4)
    }

    fn step_tcycle(&mut self) {
        if self.frequency_timer <= 1 {
            self.frequency_timer = self.period_from_frequency();
            self.duty_position = (self.duty_position + 1) & 0x07;
        } else {
            self.frequency_timer -= 1;
        }
    }

    fn clock_length(&mut self) {
        if !self.length_enabled || self.length_counter == 0 {
            return;
        }

        self.length_counter -= 1;
        if self.length_counter == 0 {
            self.enabled = false;
        }
    }

    fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    fn clock_sweep(&mut self) {
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

    fn output_amplitude(&self) -> i16 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }
        let bit = DUTY_PATTERNS[self.duty as usize][self.duty_position as usize];
        let volume = self.envelope.volume as i16;
        if bit == 0 { -volume } else { volume }
    }
}

#[derive(Clone, Copy, Default)]
struct WaveChannel {
    enabled: bool,
    dac_enabled: bool,
    length_counter: u16,
    length_enabled: bool,
    output_level: u8,
    frequency: u16,
    frequency_timer: u16,
    wave_position: u8,
    sample_buffer: u8,
}

impl WaveChannel {
    fn write_dac_enable(&mut self, value: u8) {
        self.dac_enabled = (value & 0x80) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    fn write_length(&mut self, value: u8) {
        self.length_counter = MAX_WAVE_LENGTH - (value as u16);
    }

    fn write_output_level(&mut self, value: u8) {
        self.output_level = (value >> 5) & 0x03;
    }

    fn write_frequency_low(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x0700) | (value as u16);
    }

    fn write_frequency_high(&mut self, value: u8, length_clocks_next: bool) {
        self.frequency = (self.frequency & 0x00FF) | (((value & 0x07) as u16) << 8);
        let old_length_enabled = self.length_enabled;
        self.length_enabled = (value & 0x40) != 0;
        let trigger_requested = (value & 0x80) != 0;
        self.apply_length_enable_edge(old_length_enabled, length_clocks_next, trigger_requested);
        if trigger_requested {
            self.trigger(length_clocks_next);
        }
    }

    fn apply_length_enable_edge(
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

    fn trigger(&mut self, length_clocks_next: bool) {
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

    fn period_from_frequency(&self) -> u16 {
        let freq = self.frequency.min(MAX_FREQUENCY);
        let period = (MAX_FREQUENCY + 1 - freq).saturating_mul(2);
        period.max(2)
    }

    fn step_tcycle(&mut self, io: &[u8; 0x80]) {
        if self.frequency_timer <= 1 {
            self.frequency_timer = self.period_from_frequency();
            self.wave_position = (self.wave_position + 1) & 0x1F;
            self.sample_buffer = io[WAVE_RAM_START_INDEX + (self.wave_position as usize / 2)];
        } else {
            self.frequency_timer -= 1;
        }
    }

    fn clock_length(&mut self) {
        if !self.length_enabled || self.length_counter == 0 {
            return;
        }

        self.length_counter -= 1;
        if self.length_counter == 0 {
            self.enabled = false;
        }
    }

    fn output_amplitude(&self, _io: &[u8; 0x80]) -> i16 {
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
struct NoiseChannel {
    enabled: bool,
    dac_enabled: bool,
    length_counter: u8,
    length_enabled: bool,
    envelope: EnvelopeState,
    clock_shift: u8,
    width_mode_7bit: bool,
    divisor_code: u8,
    frequency_timer: u16,
    lfsr: u16,
}

impl NoiseChannel {
    fn write_length(&mut self, value: u8) {
        self.length_counter = MAX_NOISE_LENGTH - (value & 0x3F);
    }

    fn write_envelope(&mut self, value: u8) {
        self.envelope.write_register(value);
        self.dac_enabled = (value & 0xF8) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    fn write_polynomial(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & 0x0F;
        self.width_mode_7bit = (value & 0x08) != 0;
        self.divisor_code = value & 0x07;
    }

    fn write_control(&mut self, value: u8, length_clocks_next: bool, envelope_clocks_next: bool) {
        let old_length_enabled = self.length_enabled;
        self.length_enabled = (value & 0x40) != 0;
        let trigger_requested = (value & 0x80) != 0;
        self.apply_length_enable_edge(old_length_enabled, length_clocks_next, trigger_requested);
        if trigger_requested {
            self.trigger(length_clocks_next, envelope_clocks_next);
        }
    }

    fn apply_length_enable_edge(
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

    fn trigger(&mut self, length_clocks_next: bool, envelope_clocks_next: bool) {
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

    fn period_from_registers(&self) -> u16 {
        let divisor = NOISE_DIVISORS[self.divisor_code as usize] as u32;
        let period = divisor << self.clock_shift;
        period.min(u16::MAX as u32) as u16
    }

    fn step_tcycle(&mut self) {
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

    fn clock_length(&mut self) {
        if !self.length_enabled || self.length_counter == 0 {
            return;
        }

        self.length_counter -= 1;
        if self.length_counter == 0 {
            self.enabled = false;
        }
    }

    fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    fn output_amplitude(&self) -> i16 {
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

pub(super) struct ApuState {
    analog_profile: AnalogCalibrationProfile,
    enabled: bool,
    channel_on_mask: u8,
    frame_sequencer_step: u8,
    frame_sequencer_ticks: u64,
    length_tick_count: u64,
    sweep_tick_count: u64,
    envelope_tick_count: u64,
    square1: SquareChannel,
    square2: SquareChannel,
    wave: WaveChannel,
    noise: NoiseChannel,
    last_mixed_sample_left: f32,
    last_mixed_sample_right: f32,
    last_mixed_sample: f32,
    lpf_output_prev_left: f32,
    lpf_output_prev_right: f32,
    hpf_input_prev_left: f32,
    hpf_output_prev_left: f32,
    hpf_input_prev_right: f32,
    hpf_output_prev_right: f32,
    pending_tcycle_samples: Vec<f32>,
    capture_tcycle_stream: bool,
}

impl Default for ApuState {
    fn default() -> Self {
        Self {
            analog_profile: AnalogCalibrationProfile::for_model(HardwareModel::Dmg).normalized(),
            enabled: false,
            channel_on_mask: 0,
            frame_sequencer_step: 0,
            frame_sequencer_ticks: 0,
            length_tick_count: 0,
            sweep_tick_count: 0,
            envelope_tick_count: 0,
            square1: SquareChannel::with_sweep(),
            square2: SquareChannel::default(),
            wave: WaveChannel::default(),
            noise: NoiseChannel::default(),
            last_mixed_sample_left: 0.0,
            last_mixed_sample_right: 0.0,
            last_mixed_sample: 0.0,
            lpf_output_prev_left: 0.0,
            lpf_output_prev_right: 0.0,
            hpf_input_prev_left: 0.0,
            hpf_output_prev_left: 0.0,
            hpf_input_prev_right: 0.0,
            hpf_output_prev_right: 0.0,
            pending_tcycle_samples: Vec::new(),
            capture_tcycle_stream: false,
        }
    }
}
