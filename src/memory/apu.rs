use super::Bus;

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
const DMG_HPF_COEFF: f32 = 0.999_958;
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
            hpf_input_prev_left: 0.0,
            hpf_output_prev_left: 0.0,
            hpf_input_prev_right: 0.0,
            hpf_output_prev_right: 0.0,
            pending_tcycle_samples: Vec::new(),
            capture_tcycle_stream: false,
        }
    }
}

impl ApuState {
    fn from_boot_registers(io: &[u8; 0x80]) -> Self {
        let nr52 = io[NR52_INDEX];
        let mut state = Self {
            enabled: (nr52 & 0x80) != 0,
            channel_on_mask: nr52 & 0x0F,
            ..Self::default()
        };
        if state.enabled {
            let mask = state.channel_on_mask;
            state.square1.enabled = (mask & 0x01) != 0;
            state.square2.enabled = (mask & 0x02) != 0;
            state.wave.enabled = (mask & 0x04) != 0;
            state.noise.enabled = (mask & 0x08) != 0;
        }
        state
    }

    fn clock_frame_sequencer(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_sequencer_ticks = self.frame_sequencer_ticks.saturating_add(1);
        let step = self.frame_sequencer_step;
        if (step & 0x01) == 0 {
            self.length_tick_count = self.length_tick_count.saturating_add(1);
            self.square1.clock_length();
            self.square2.clock_length();
            self.wave.clock_length();
            self.noise.clock_length();
        }
        if step == 2 || step == 6 {
            self.sweep_tick_count = self.sweep_tick_count.saturating_add(1);
            self.square1.clock_sweep();
        }
        if step == 7 {
            self.envelope_tick_count = self.envelope_tick_count.saturating_add(1);
            self.square1.clock_envelope();
            self.square2.clock_envelope();
            self.noise.clock_envelope();
        }
        self.frame_sequencer_step = (self.frame_sequencer_step + 1) & 0x07;
        self.refresh_channel_on_mask();
    }

    fn length_clocks_on_next_frame_step(&self) -> bool {
        (self.frame_sequencer_step & 0x01) == 0
    }

    fn envelope_clocks_on_next_frame_step(&self) -> bool {
        self.frame_sequencer_step == 7
    }

    fn reset_after_power_toggle(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.channel_on_mask = 0;
        self.frame_sequencer_step = 0;
        self.frame_sequencer_ticks = 0;
        self.length_tick_count = 0;
        self.sweep_tick_count = 0;
        self.envelope_tick_count = 0;
        self.square1.reset(true);
        self.square2.reset(false);
        self.wave = WaveChannel::default();
        self.noise = NoiseChannel::default();
        self.last_mixed_sample_left = 0.0;
        self.last_mixed_sample_right = 0.0;
        self.last_mixed_sample = 0.0;
        self.hpf_input_prev_left = 0.0;
        self.hpf_output_prev_left = 0.0;
        self.hpf_input_prev_right = 0.0;
        self.hpf_output_prev_right = 0.0;
    }

    fn step_tcycle(&mut self, io: &[u8; 0x80]) {
        if !self.enabled {
            self.last_mixed_sample_left = 0.0;
            self.last_mixed_sample_right = 0.0;
            self.last_mixed_sample = 0.0;
            if self.capture_tcycle_stream {
                self.push_tcycle_sample(0.0, 0.0);
            }
            return;
        }

        self.square1.step_tcycle();
        self.square2.step_tcycle();
        self.wave.step_tcycle(io);
        self.noise.step_tcycle();
        self.refresh_channel_on_mask();
        let should_mix_sample = self.capture_tcycle_stream || cfg!(test);
        if !should_mix_sample {
            return;
        }
        let (mixed_left, mixed_right) = self.mix_sample(io);
        let (filtered_left, filtered_right) = self.apply_hpf(mixed_left, mixed_right);
        self.last_mixed_sample_left = filtered_left.clamp(-1.0, 1.0);
        self.last_mixed_sample_right = filtered_right.clamp(-1.0, 1.0);
        self.last_mixed_sample = (self.last_mixed_sample_left + self.last_mixed_sample_right) * 0.5;
        if self.capture_tcycle_stream {
            self.push_tcycle_sample(self.last_mixed_sample_left, self.last_mixed_sample_right);
        }
    }

    fn push_tcycle_sample(&mut self, left: f32, right: f32) {
        let max_scalars = MAX_PENDING_AUDIO_TCYCLE_FRAMES.saturating_mul(2);
        if self.pending_tcycle_samples.len().saturating_add(2) > max_scalars {
            return;
        }
        self.pending_tcycle_samples.push(left);
        self.pending_tcycle_samples.push(right);
    }

    fn refresh_channel_on_mask(&mut self) {
        let mut mask = 0u8;
        if self.square1.enabled {
            mask |= 1 << 0;
        }
        if self.square2.enabled {
            mask |= 1 << 1;
        }
        if self.wave.enabled {
            mask |= 1 << 2;
        }
        if self.noise.enabled {
            mask |= 1 << 3;
        }
        self.channel_on_mask = mask;
    }

    fn mix_sample(&self, io: &[u8; 0x80]) -> (f32, f32) {
        let nr50 = io[NR50_INDEX];
        let nr51 = io[NR51_INDEX];
        let channel_output = [
            self.square1.output_amplitude(),
            self.square2.output_amplitude(),
            self.wave.output_amplitude(io),
            self.noise.output_amplitude(),
        ];

        let mut right = 0.0f32;
        let mut left = 0.0f32;
        for (index, amplitude) in channel_output.iter().enumerate().take(CHANNEL_COUNT) {
            let normalized = (*amplitude as f32) / 15.0;
            if (nr51 & (1 << index)) != 0 {
                right += normalized;
            }
            if (nr51 & (1 << (index + 4))) != 0 {
                left += normalized;
            }
        }

        let right_volume = (((nr50 & 0x07) as f32) + 1.0) / 8.0;
        let left_volume = ((((nr50 >> 4) & 0x07) as f32) + 1.0) / 8.0;
        let right = (right / CHANNEL_COUNT as f32) * right_volume;
        let left = (left / CHANNEL_COUNT as f32) * left_volume;
        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }

    fn apply_hpf(&mut self, left_input: f32, right_input: f32) -> (f32, f32) {
        let left_output =
            left_input - self.hpf_input_prev_left + self.hpf_output_prev_left * DMG_HPF_COEFF;
        let right_output =
            right_input - self.hpf_input_prev_right + self.hpf_output_prev_right * DMG_HPF_COEFF;
        self.hpf_input_prev_left = left_input;
        self.hpf_output_prev_left = left_output;
        self.hpf_input_prev_right = right_input;
        self.hpf_output_prev_right = right_output;
        (left_output, right_output)
    }
}

impl Bus {
    pub(super) fn sync_apu_boot_state(&mut self) {
        self.apu = ApuState::from_boot_registers(&self.io);
    }

    pub(super) fn read_nr52(&self) -> u8 {
        ((self.apu.enabled as u8) << 7) | (self.apu.channel_on_mask & 0x0F)
    }

    pub(super) fn write_nr50(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR50_INDEX] = value;
    }

    pub(super) fn write_nr51(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR51_INDEX] = value;
    }

    pub(super) fn write_nr52(&mut self, value: u8) {
        let request_enabled = (value & 0x80) != 0;

        if self.apu.enabled && !request_enabled {
            self.clear_apu_registers();
            self.apu.reset_after_power_toggle(false);
            self.io[NR52_INDEX] = 0x00;
            return;
        }

        if !self.apu.enabled && request_enabled {
            self.clear_apu_registers();
            self.apu.reset_after_power_toggle(true);
            self.io[NR52_INDEX] = 0x80;
            return;
        }

        if self.apu.enabled {
            self.io[NR52_INDEX] = 0x80 | (self.apu.channel_on_mask & 0x0F);
        } else {
            self.io[NR52_INDEX] = 0x00;
        }
    }

    pub(super) fn write_apu_register(&mut self, addr: u16, value: u8) {
        let index = (addr - 0xFF00) as usize;
        if (WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX).contains(&index) {
            self.io[index] = value;
            return;
        }
        if !self.apu.enabled {
            return;
        }

        self.io[index] = value;
        let length_clocks_next = self.apu.length_clocks_on_next_frame_step();
        let envelope_clocks_next = self.apu.envelope_clocks_on_next_frame_step();
        match index {
            NR10_INDEX => {
                if self.apu.square1.sweep.write_register(value) {
                    self.apu.square1.enabled = false;
                }
            }
            NR11_INDEX => self.apu.square1.write_duty_length(value),
            NR12_INDEX => self.apu.square1.write_envelope(value),
            NR13_INDEX => self.apu.square1.write_frequency_low(value),
            NR14_INDEX => self.apu.square1.write_frequency_high(
                value,
                length_clocks_next,
                envelope_clocks_next,
            ),
            NR21_INDEX => self.apu.square2.write_duty_length(value),
            NR22_INDEX => self.apu.square2.write_envelope(value),
            NR23_INDEX => self.apu.square2.write_frequency_low(value),
            NR24_INDEX => self.apu.square2.write_frequency_high(
                value,
                length_clocks_next,
                envelope_clocks_next,
            ),
            NR30_INDEX => self.apu.wave.write_dac_enable(value),
            NR31_INDEX => self.apu.wave.write_length(value),
            NR32_INDEX => self.apu.wave.write_output_level(value),
            NR33_INDEX => self.apu.wave.write_frequency_low(value),
            NR34_INDEX => self
                .apu
                .wave
                .write_frequency_high(value, length_clocks_next),
            NR41_INDEX => self.apu.noise.write_length(value),
            NR42_INDEX => self.apu.noise.write_envelope(value),
            NR43_INDEX => self.apu.noise.write_polynomial(value),
            NR44_INDEX => {
                self.apu
                    .noise
                    .write_control(value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }
        self.apu.refresh_channel_on_mask();
    }

    pub(super) fn step_apu_frame_sequencer_from_divider(&mut self, old_div: u16, new_div: u16) {
        if !self.apu.enabled {
            return;
        }

        let old_high = (old_div & DIV_APU_BIT) != 0;
        let new_high = (new_div & DIV_APU_BIT) != 0;
        if old_high && !new_high {
            self.apu.clock_frame_sequencer();
        }
    }

    pub(super) fn step_apu_tcycle(&mut self) {
        self.apu.step_tcycle(&self.io);
    }

    pub fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        if self.apu.pending_tcycle_samples.is_empty() {
            return Vec::new();
        }
        self.apu.pending_tcycle_samples.drain(..).collect()
    }

    pub fn set_audio_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.apu.capture_tcycle_stream = enabled;
        if !enabled {
            self.apu.pending_tcycle_samples.clear();
        }
    }

    fn clear_apu_registers(&mut self) {
        for index in NR10_INDEX..=NR51_INDEX {
            self.io[index] = 0x00;
        }
    }

    #[cfg(test)]
    pub(super) fn apu_frame_sequencer_step(&self) -> u8 {
        self.apu.frame_sequencer_step
    }

    #[cfg(test)]
    pub(super) fn apu_frame_sequencer_ticks(&self) -> u64 {
        self.apu.frame_sequencer_ticks
    }

    #[cfg(test)]
    pub(super) fn apu_length_tick_count(&self) -> u64 {
        self.apu.length_tick_count
    }

    #[cfg(test)]
    pub(super) fn apu_sweep_tick_count(&self) -> u64 {
        self.apu.sweep_tick_count
    }

    #[cfg(test)]
    pub(super) fn apu_envelope_tick_count(&self) -> u64 {
        self.apu.envelope_tick_count
    }

    #[cfg(test)]
    pub(super) fn apu_last_mixed_sample(&self) -> f32 {
        self.apu.last_mixed_sample
    }

    #[cfg(test)]
    pub(super) fn apu_last_mixed_sample_stereo(&self) -> (f32, f32) {
        (
            self.apu.last_mixed_sample_left,
            self.apu.last_mixed_sample_right,
        )
    }

    #[cfg(test)]
    pub(super) fn apu_square2_envelope_volume(&self) -> u8 {
        self.apu.square2.envelope.volume
    }

    #[cfg(test)]
    pub(super) fn apu_square2_envelope_timer(&self) -> u8 {
        self.apu.square2.envelope.timer
    }

    #[cfg(test)]
    pub(super) fn apu_square2_length_counter(&self) -> u8 {
        self.apu.square2.length_counter
    }

    #[cfg(test)]
    pub(super) fn apu_square1_frequency(&self) -> u16 {
        self.apu.square1.frequency
    }

    #[cfg(test)]
    pub(super) fn apu_square1_enabled(&self) -> bool {
        self.apu.square1.enabled
    }

    #[cfg(test)]
    pub(super) fn apu_square2_enabled(&self) -> bool {
        self.apu.square2.enabled
    }

    #[cfg(test)]
    pub(super) fn apu_wave_position(&self) -> u8 {
        self.apu.wave.wave_position
    }

    #[cfg(test)]
    pub(super) fn apu_wave_sample_buffer(&self) -> u8 {
        self.apu.wave.sample_buffer
    }

    #[cfg(test)]
    pub(super) fn apu_noise_lfsr(&self) -> u16 {
        self.apu.noise.lfsr
    }
}
