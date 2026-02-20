use super::*;

impl ApuState {
    pub(super) fn from_boot_registers(io: &[u8; 0x80], model: HardwareModel) -> Self {
        let nr52 = io[NR52_INDEX];
        let mut state = Self {
            analog_profile: AnalogCalibrationProfile::for_model(model).normalized(),
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

    pub(super) fn clock_frame_sequencer(&mut self) {
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

    pub(super) fn length_clocks_on_next_frame_step(&self) -> bool {
        (self.frame_sequencer_step & 0x01) == 0
    }

    pub(super) fn envelope_clocks_on_next_frame_step(&self) -> bool {
        self.frame_sequencer_step == 7
    }

    pub(super) fn reset_after_power_toggle(&mut self, enabled: bool) {
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
        self.reset_analog_filter_state();
    }

    pub(super) fn reset_analog_filter_state(&mut self) {
        self.lpf_output_prev_left = 0.0;
        self.lpf_output_prev_right = 0.0;
        self.hpf_input_prev_left = 0.0;
        self.hpf_output_prev_left = 0.0;
        self.hpf_input_prev_right = 0.0;
        self.hpf_output_prev_right = 0.0;
    }

    pub(super) fn step_tcycle(&mut self, io: &[u8; 0x80]) {
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
        let (filtered_left, filtered_right) = self.apply_analog_path(mixed_left, mixed_right);
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

    pub(super) fn refresh_channel_on_mask(&mut self) {
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
            let normalized = self.shape_channel_dac(index, *amplitude);
            if (nr51 & (1 << index)) != 0 {
                right += normalized * self.analog_profile.routing_right[index];
            }
            if (nr51 & (1 << (index + 4))) != 0 {
                left += normalized * self.analog_profile.routing_left[index];
            }
        }

        let right_volume = (((nr50 & 0x07) as f32) + 1.0) / 8.0;
        let left_volume = ((((nr50 >> 4) & 0x07) as f32) + 1.0) / 8.0;
        let post_volume_right = right * right_volume;
        let post_volume_left = left * left_volume;
        let mixed_left = post_volume_left
            + post_volume_right * self.analog_profile.crossfeed
            + self.analog_profile.output_bias_left;
        let mixed_right = post_volume_right
            + post_volume_left * self.analog_profile.crossfeed
            + self.analog_profile.output_bias_right;
        let right = self.apply_soft_clip(mixed_right * self.analog_profile.right_gain);
        let left = self.apply_soft_clip(mixed_left * self.analog_profile.left_gain);
        (left, right)
    }

    fn shape_channel_dac(&self, channel_index: usize, amplitude: i16) -> f32 {
        let normalized = (amplitude as f32) / 15.0;
        let cubic = normalized * normalized * normalized;
        (normalized - self.analog_profile.channel_nonlinearity[channel_index] * cubic)
            * self.analog_profile.channel_gain[channel_index]
            + self.analog_profile.channel_bias[channel_index]
    }

    fn apply_soft_clip(&self, sample: f32) -> f32 {
        let drive = self.analog_profile.soft_clip_drive.max(0.1);
        let normalized = drive.tanh();
        if normalized <= f32::EPSILON {
            return sample.clamp(-1.0, 1.0);
        }
        ((sample * drive).tanh() / normalized).clamp(-1.0, 1.0)
    }

    fn apply_analog_path(&mut self, left_input: f32, right_input: f32) -> (f32, f32) {
        let alpha = self.analog_profile.low_pass_alpha.clamp(0.0, 1.0);
        self.lpf_output_prev_left += alpha * (left_input - self.lpf_output_prev_left);
        self.lpf_output_prev_right += alpha * (right_input - self.lpf_output_prev_right);
        self.apply_hpf(self.lpf_output_prev_left, self.lpf_output_prev_right)
    }

    fn apply_hpf(&mut self, left_input: f32, right_input: f32) -> (f32, f32) {
        let hpf_coeff = self.analog_profile.hpf_coeff;
        let left_output =
            left_input - self.hpf_input_prev_left + self.hpf_output_prev_left * hpf_coeff;
        let right_output =
            right_input - self.hpf_input_prev_right + self.hpf_output_prev_right * hpf_coeff;
        self.hpf_input_prev_left = left_input;
        self.hpf_output_prev_left = left_output;
        self.hpf_input_prev_right = right_input;
        self.hpf_output_prev_right = right_output;
        (left_output, right_output)
    }
}
