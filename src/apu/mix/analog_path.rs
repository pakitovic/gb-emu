use super::super::*;

impl ApuState {
    pub(in crate::apu) fn reset_analog_filter_state(&mut self) {
        self.analog.lpf_output_prev_left = 0.0;
        self.analog.lpf_output_prev_right = 0.0;
        self.analog.hpf_input_prev_left = 0.0;
        self.analog.hpf_output_prev_left = 0.0;
        self.analog.hpf_input_prev_right = 0.0;
        self.analog.hpf_output_prev_right = 0.0;
    }

    pub(in crate::apu) fn apply_soft_clip(&self, sample: f32) -> f32 {
        let drive = self.analog_profile.soft_clip_drive.max(0.1);
        let normalized = drive.tanh();
        if normalized <= f32::EPSILON {
            return sample.clamp(-1.0, 1.0);
        }
        ((sample * drive).tanh() / normalized).clamp(-1.0, 1.0)
    }

    pub(in crate::apu) fn apply_analog_path(
        &mut self,
        left_input: f32,
        right_input: f32,
    ) -> (f32, f32) {
        let alpha = self.analog_profile.low_pass_alpha.clamp(0.0, 1.0);
        self.analog.lpf_output_prev_left += alpha * (left_input - self.analog.lpf_output_prev_left);
        self.analog.lpf_output_prev_right +=
            alpha * (right_input - self.analog.lpf_output_prev_right);
        self.apply_hpf(
            self.analog.lpf_output_prev_left,
            self.analog.lpf_output_prev_right,
        )
    }

    pub(in crate::apu) fn apply_output_stage(
        &self,
        left_input: f32,
        right_input: f32,
    ) -> (f32, f32) {
        (
            self.apply_output_headroom_soft_clip(left_input),
            self.apply_output_headroom_soft_clip(right_input),
        )
    }

    fn apply_hpf(&mut self, left_input: f32, right_input: f32) -> (f32, f32) {
        let hpf_coeff = self.analog_profile.hpf_coeff;
        let left_output = left_input - self.analog.hpf_input_prev_left
            + self.analog.hpf_output_prev_left * hpf_coeff;
        let right_output = right_input - self.analog.hpf_input_prev_right
            + self.analog.hpf_output_prev_right * hpf_coeff;
        self.analog.hpf_input_prev_left = left_input;
        self.analog.hpf_output_prev_left = left_output;
        self.analog.hpf_input_prev_right = right_input;
        self.analog.hpf_output_prev_right = right_output;
        (left_output, right_output)
    }

    fn apply_output_headroom_soft_clip(&self, sample: f32) -> f32 {
        if !sample.is_finite() {
            return 0.0;
        }
        let headroom = self.analog_profile.output_headroom.clamp(0.1, 1.0);
        let normalized = self.apply_soft_clip(sample / headroom) * headroom;
        if normalized.is_finite() {
            normalized.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }
}
