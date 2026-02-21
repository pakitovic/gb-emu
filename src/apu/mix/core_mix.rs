use super::super::*;

impl ApuState {
    pub(super) fn mix_sample(&self, io: &[u8; 0x80]) -> (f32, f32) {
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
}
