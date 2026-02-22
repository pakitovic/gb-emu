use super::*;

mod analog_path;
mod core_mix;

impl ApuState {
    pub(super) fn step_tcycle(&mut self, _io: &[u8; 0x80]) {
        if !self.enabled {
            self.analog.last_mixed_sample_left = 0.0;
            self.analog.last_mixed_sample_right = 0.0;
            self.analog.last_mixed_sample = 0.0;
            if self.stream.capture_tcycle_stream {
                self.push_tcycle_sample(0.0, 0.0);
            }
            return;
        }

        self.square1.step_tcycle();
        self.square2.step_tcycle();
        self.wave.step_tcycle(&self.registers);
        self.noise.step_tcycle();
        self.refresh_channel_on_mask();
        let should_mix_sample = self.stream.capture_tcycle_stream || cfg!(test);
        if !should_mix_sample {
            return;
        }
        let (mixed_left, mixed_right) = self.mix_sample();
        let (filtered_left, filtered_right) = self.apply_analog_path(mixed_left, mixed_right);
        let (output_left, output_right) = self.apply_output_stage(filtered_left, filtered_right);
        self.analog.last_mixed_sample_left = output_left;
        self.analog.last_mixed_sample_right = output_right;
        self.analog.last_mixed_sample =
            (self.analog.last_mixed_sample_left + self.analog.last_mixed_sample_right) * 0.5;
        if self.stream.capture_tcycle_stream {
            self.push_tcycle_sample(
                self.analog.last_mixed_sample_left,
                self.analog.last_mixed_sample_right,
            );
        }
    }

    fn push_tcycle_sample(&mut self, left: f32, right: f32) {
        let max_scalars = MAX_PENDING_AUDIO_TCYCLE_FRAMES.saturating_mul(2);
        if self.stream.pending_tcycle_samples.len().saturating_add(2) > max_scalars {
            return;
        }
        self.stream.pending_tcycle_samples.push(left);
        self.stream.pending_tcycle_samples.push(right);
    }
}
