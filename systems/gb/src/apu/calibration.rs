use crate::hardware::HardwareModel;

const APU_MIX_CHANNELS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalogCalibrationProfile {
    pub hpf_coeff: f32,
    pub low_pass_alpha: f32,
    pub channel_gain: [f32; APU_MIX_CHANNELS],
    pub channel_nonlinearity: [f32; APU_MIX_CHANNELS],
    pub channel_bias: [f32; APU_MIX_CHANNELS],
    pub routing_left: [f32; APU_MIX_CHANNELS],
    pub routing_right: [f32; APU_MIX_CHANNELS],
    pub left_gain: f32,
    pub right_gain: f32,
    pub output_headroom: f32,
    pub soft_clip_drive: f32,
    pub crossfeed: f32,
    pub output_bias_left: f32,
    pub output_bias_right: f32,
}

impl AnalogCalibrationProfile {
    pub fn for_model(model: HardwareModel) -> Self {
        match model {
            HardwareModel::Dmg0 => Self {
                hpf_coeff: 0.999_958,
                low_pass_alpha: 0.004_1,
                channel_gain: [0.280, 0.270, 0.245, 0.250],
                channel_nonlinearity: [0.16, 0.14, 0.09, 0.11],
                channel_bias: [0.0, 0.0, 0.0, 0.0],
                routing_left: [1.00, 0.99, 1.02, 1.00],
                routing_right: [1.00, 1.00, 1.01, 0.99],
                left_gain: 1.0,
                right_gain: 1.0,
                output_headroom: 0.97,
                soft_clip_drive: 1.7,
                crossfeed: 0.0,
                output_bias_left: 0.0,
                output_bias_right: 0.0,
            },
            HardwareModel::Dmg => Self {
                hpf_coeff: 0.999_958,
                low_pass_alpha: 0.004_6,
                channel_gain: [0.270, 0.260, 0.235, 0.245],
                channel_nonlinearity: [0.14, 0.12, 0.08, 0.10],
                channel_bias: [0.0, 0.0, 0.0, 0.0],
                routing_left: [1.00, 1.00, 1.00, 1.00],
                routing_right: [1.00, 1.00, 1.00, 1.00],
                left_gain: 1.0,
                right_gain: 1.0,
                output_headroom: 0.97,
                soft_clip_drive: 1.6,
                crossfeed: 0.0,
                output_bias_left: 0.0,
                output_bias_right: 0.0,
            },
            HardwareModel::Mgb => Self {
                hpf_coeff: 0.999_935,
                low_pass_alpha: 0.006_4,
                channel_gain: [0.255, 0.245, 0.220, 0.230],
                channel_nonlinearity: [0.10, 0.09, 0.06, 0.07],
                channel_bias: [0.0, 0.0, 0.0, 0.0],
                routing_left: [1.00, 0.99, 1.00, 1.00],
                routing_right: [1.00, 0.99, 1.00, 1.00],
                left_gain: 0.98,
                right_gain: 0.98,
                output_headroom: 0.98,
                soft_clip_drive: 1.45,
                crossfeed: 0.0,
                output_bias_left: 0.0,
                output_bias_right: 0.0,
            },
            HardwareModel::Sgb => Self {
                hpf_coeff: 0.999_910,
                low_pass_alpha: 0.007_8,
                channel_gain: [0.245, 0.235, 0.210, 0.220],
                channel_nonlinearity: [0.08, 0.07, 0.05, 0.06],
                channel_bias: [0.0, 0.0, 0.0, 0.0],
                routing_left: [0.99, 0.98, 0.99, 0.99],
                routing_right: [0.99, 0.98, 0.99, 0.99],
                left_gain: 0.95,
                right_gain: 0.95,
                output_headroom: 0.99,
                soft_clip_drive: 1.35,
                crossfeed: 0.0,
                output_bias_left: 0.0,
                output_bias_right: 0.0,
            },
            HardwareModel::Sgb2 => Self {
                hpf_coeff: 0.999_915,
                low_pass_alpha: 0.007_5,
                channel_gain: [0.248, 0.238, 0.212, 0.222],
                channel_nonlinearity: [0.08, 0.07, 0.05, 0.06],
                channel_bias: [0.0, 0.0, 0.0, 0.0],
                routing_left: [0.99, 0.99, 1.00, 0.99],
                routing_right: [0.99, 0.99, 1.00, 0.99],
                left_gain: 0.96,
                right_gain: 0.96,
                output_headroom: 0.99,
                soft_clip_drive: 1.3,
                crossfeed: 0.0,
                output_bias_left: 0.0,
                output_bias_right: 0.0,
            },
        }
    }

    pub fn normalized(mut self) -> Self {
        let sanitize = |value: f32, min: f32, max: f32, fallback: f32| -> f32 {
            if value.is_nan() {
                fallback
            } else {
                value.clamp(min, max)
            }
        };

        self.hpf_coeff = sanitize(self.hpf_coeff, 0.0, 0.999_999, 0.999_958);
        self.low_pass_alpha = sanitize(self.low_pass_alpha, 0.0, 1.0, 0.004_6);
        self.left_gain = sanitize(self.left_gain, 0.0, 2.0, 1.0);
        self.right_gain = sanitize(self.right_gain, 0.0, 2.0, 1.0);
        self.output_headroom = sanitize(self.output_headroom, 0.1, 1.0, 1.0);
        self.soft_clip_drive = sanitize(self.soft_clip_drive, 0.1, 8.0, 1.0);
        self.crossfeed = sanitize(self.crossfeed, 0.0, 0.5, 0.0);
        self.output_bias_left = sanitize(self.output_bias_left, -1.0, 1.0, 0.0);
        self.output_bias_right = sanitize(self.output_bias_right, -1.0, 1.0, 0.0);
        for index in 0..APU_MIX_CHANNELS {
            self.channel_gain[index] = sanitize(self.channel_gain[index], 0.0, 4.0, 0.25);
            self.channel_nonlinearity[index] =
                sanitize(self.channel_nonlinearity[index], 0.0, 1.0, 0.0);
            self.channel_bias[index] = sanitize(self.channel_bias[index], -1.0, 1.0, 0.0);
            self.routing_left[index] = sanitize(self.routing_left[index], 0.0, 4.0, 1.0);
            self.routing_right[index] = sanitize(self.routing_right[index], 0.0, 4.0, 1.0);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_differ_between_models() {
        let dmg = AnalogCalibrationProfile::for_model(HardwareModel::Dmg);
        let mgb = AnalogCalibrationProfile::for_model(HardwareModel::Mgb);
        assert_ne!(dmg.hpf_coeff, mgb.hpf_coeff);
        assert_ne!(dmg.low_pass_alpha, mgb.low_pass_alpha);
        assert_ne!(dmg.channel_gain, mgb.channel_gain);
    }

    #[test]
    fn normalization_clamps_extreme_values() {
        let mut profile = AnalogCalibrationProfile::for_model(HardwareModel::Dmg);
        profile.hpf_coeff = 2.0;
        profile.low_pass_alpha = -1.0;
        profile.channel_gain = [9.0, -2.0, 1.0, 1.0];
        profile.channel_nonlinearity = [2.0, -1.0, 0.5, 0.5];
        profile.channel_bias = [2.0, -2.0, 0.0, 0.0];
        profile.routing_left = [9.0, -2.0, 1.0, 1.0];
        profile.routing_right = [9.0, -2.0, 1.0, 1.0];
        profile.left_gain = 10.0;
        profile.right_gain = -2.0;
        profile.output_headroom = 9.0;
        profile.soft_clip_drive = 0.0;
        profile.crossfeed = 9.0;
        profile.output_bias_left = 2.0;
        profile.output_bias_right = -2.0;

        let normalized = profile.normalized();
        assert!((0.0..1.0).contains(&normalized.hpf_coeff));
        assert!((0.0..=1.0).contains(&normalized.low_pass_alpha));
        assert_eq!(normalized.channel_gain[0], 4.0);
        assert_eq!(normalized.channel_gain[1], 0.0);
        assert_eq!(normalized.channel_nonlinearity[0], 1.0);
        assert_eq!(normalized.channel_nonlinearity[1], 0.0);
        assert_eq!(normalized.channel_bias[0], 1.0);
        assert_eq!(normalized.channel_bias[1], -1.0);
        assert_eq!(normalized.routing_left[0], 4.0);
        assert_eq!(normalized.routing_left[1], 0.0);
        assert_eq!(normalized.left_gain, 2.0);
        assert_eq!(normalized.right_gain, 0.0);
        assert_eq!(normalized.output_headroom, 1.0);
        assert_eq!(normalized.soft_clip_drive, 0.1);
        assert_eq!(normalized.crossfeed, 0.5);
        assert_eq!(normalized.output_bias_left, 1.0);
        assert_eq!(normalized.output_bias_right, -1.0);
    }

    #[test]
    fn normalization_replaces_nan_values() {
        let mut profile = AnalogCalibrationProfile::for_model(HardwareModel::Dmg);
        profile.hpf_coeff = f32::NAN;
        profile.low_pass_alpha = f32::NAN;
        profile.left_gain = f32::NAN;
        profile.right_gain = f32::NAN;
        profile.output_headroom = f32::NAN;
        profile.soft_clip_drive = f32::NAN;
        profile.crossfeed = f32::NAN;
        profile.output_bias_left = f32::NAN;
        profile.output_bias_right = f32::NAN;
        profile.channel_gain[0] = f32::NAN;
        profile.channel_nonlinearity[1] = f32::NAN;
        profile.channel_bias[2] = f32::NAN;
        profile.routing_left[3] = f32::NAN;
        profile.routing_right[0] = f32::NAN;

        let normalized = profile.normalized();
        assert_eq!(normalized.hpf_coeff, 0.999_958);
        assert_eq!(normalized.low_pass_alpha, 0.004_6);
        assert_eq!(normalized.left_gain, 1.0);
        assert_eq!(normalized.right_gain, 1.0);
        assert_eq!(normalized.output_headroom, 1.0);
        assert_eq!(normalized.soft_clip_drive, 1.0);
        assert_eq!(normalized.crossfeed, 0.0);
        assert_eq!(normalized.output_bias_left, 0.0);
        assert_eq!(normalized.output_bias_right, 0.0);
        assert_eq!(normalized.channel_gain[0], 0.25);
        assert_eq!(normalized.channel_nonlinearity[1], 0.0);
        assert_eq!(normalized.channel_bias[2], 0.0);
        assert_eq!(normalized.routing_left[3], 1.0);
        assert_eq!(normalized.routing_right[0], 1.0);

        assert!(normalized.hpf_coeff.is_finite());
        assert!(normalized.low_pass_alpha.is_finite());
        assert!(normalized.left_gain.is_finite());
        assert!(normalized.right_gain.is_finite());
        assert!(normalized.output_headroom.is_finite());
        assert!(normalized.soft_clip_drive.is_finite());
    }
}
