use super::*;

impl ApuState {
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
