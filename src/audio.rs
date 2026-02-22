use crate::hardware::HardwareModel;
use crate::timing::DMG_T_CYCLES_PER_SECOND;
use std::collections::VecDeque;
use std::time::Duration;

const TEST_TONE_HZ: f32 = 440.0;
const TEST_TONE_AMPLITUDE: f32 = 0.05;
const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;
const AUDIO_OUTPUT_CHANNELS: usize = 2;
const MAX_PENDING_CORE_TCYCLE_FRAMES: usize = 524_288;
const APU_MIX_CHANNELS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixerSource {
    Silence,
    TestTone,
    CoreApu,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdaptiveQueueOptions {
    pub window_ms: u64,
    pub min_target_samples: usize,
    pub max_target_samples: usize,
    pub increase_step_samples: usize,
    pub decrease_step_samples: usize,
    pub decrease_stable_windows: u32,
    pub decrease_queue_headroom_samples: usize,
}

impl Default for AdaptiveQueueOptions {
    fn default() -> Self {
        Self {
            window_ms: 500,
            min_target_samples: 2_048,
            max_target_samples: 16_384,
            increase_step_samples: 1_024,
            decrease_step_samples: 512,
            decrease_stable_windows: 6,
            decrease_queue_headroom_samples: 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdaptiveQueueUpdate {
    pub target_samples: usize,
    pub changed: bool,
    pub window_underrun_samples: u64,
}

#[derive(Clone, Debug)]
pub struct AdaptiveQueueController {
    options: AdaptiveQueueOptions,
    target_samples: usize,
    last_window_ms: u64,
    last_underrun_samples: u64,
    stable_window_count: u32,
}

impl AdaptiveQueueController {
    pub fn new(
        initial_target_samples: usize,
        now_ms: u64,
        total_underrun_samples: u64,
        options: AdaptiveQueueOptions,
    ) -> Self {
        let normalized_options = normalize_adaptive_queue_options(options);
        let initial_target = clamp_target_samples(initial_target_samples, &normalized_options);
        Self {
            options: normalized_options,
            target_samples: initial_target,
            last_window_ms: now_ms,
            last_underrun_samples: total_underrun_samples,
            stable_window_count: 0,
        }
    }

    pub fn options(&self) -> AdaptiveQueueOptions {
        self.options
    }

    pub fn target_samples(&self) -> usize {
        self.target_samples
    }

    pub fn reset(&mut self, now_ms: u64, total_underrun_samples: u64) {
        self.last_window_ms = now_ms;
        self.last_underrun_samples = total_underrun_samples;
        self.stable_window_count = 0;
    }

    pub fn update(
        &mut self,
        now_ms: u64,
        queued_samples: usize,
        total_underrun_samples: u64,
        block_samples: usize,
    ) -> AdaptiveQueueUpdate {
        let current_target = clamp_target_samples(self.target_samples, &self.options);
        let elapsed_ms = now_ms.saturating_sub(self.last_window_ms);
        if elapsed_ms < self.options.window_ms {
            self.target_samples = current_target;
            return AdaptiveQueueUpdate {
                target_samples: current_target,
                changed: false,
                window_underrun_samples: 0,
            };
        }

        let window_underrun_samples =
            total_underrun_samples.saturating_sub(self.last_underrun_samples);
        let mut next_target = current_target;

        if window_underrun_samples > 0 {
            let severe_underrun = window_underrun_samples >= block_samples.max(1) as u64;
            let increase_step = if severe_underrun {
                self.options.increase_step_samples.saturating_mul(2)
            } else {
                self.options.increase_step_samples
            };
            next_target =
                clamp_target_samples(current_target.saturating_add(increase_step), &self.options);
            self.stable_window_count = 0;
        } else {
            let queue_headroom_samples = queued_samples.saturating_sub(current_target);
            if queue_headroom_samples >= self.options.decrease_queue_headroom_samples {
                self.stable_window_count = self.stable_window_count.saturating_add(1);
            } else {
                self.stable_window_count = 0;
            }

            if self.stable_window_count >= self.options.decrease_stable_windows {
                next_target = clamp_target_samples(
                    current_target.saturating_sub(self.options.decrease_step_samples),
                    &self.options,
                );
                self.stable_window_count = 0;
            }
        }

        self.last_window_ms = now_ms;
        self.last_underrun_samples = total_underrun_samples;
        self.target_samples = next_target;
        AdaptiveQueueUpdate {
            target_samples: next_target,
            changed: next_target != current_target,
            window_underrun_samples,
        }
    }
}

pub fn estimate_playback_underrun_samples(
    queued_samples_before_playback: usize,
    elapsed: Duration,
    sample_rate_hz: u32,
) -> u64 {
    if elapsed.is_zero() {
        return 0;
    }

    let sample_rate = sample_rate_hz.max(1) as u128;
    let expected_consumed = elapsed.as_nanos().saturating_mul(sample_rate) / NANOSECONDS_PER_SECOND;
    let queued = queued_samples_before_playback as u128;
    let underrun = expected_consumed.saturating_sub(queued);
    u64::try_from(underrun).unwrap_or(u64::MAX)
}

fn normalize_adaptive_queue_options(options: AdaptiveQueueOptions) -> AdaptiveQueueOptions {
    let min_target_samples = options.min_target_samples.max(1);
    let max_target_samples = options.max_target_samples.max(min_target_samples);

    AdaptiveQueueOptions {
        window_ms: options.window_ms.max(1),
        min_target_samples,
        max_target_samples,
        increase_step_samples: options.increase_step_samples.max(1),
        decrease_step_samples: options.decrease_step_samples.max(1),
        decrease_stable_windows: options.decrease_stable_windows.max(1),
        decrease_queue_headroom_samples: options.decrease_queue_headroom_samples,
    }
}

fn clamp_target_samples(target_samples: usize, options: &AdaptiveQueueOptions) -> usize {
    target_samples
        .max(options.min_target_samples)
        .min(options.max_target_samples)
}

pub struct AudioMixer {
    sample_rate_hz: u32,
    source: MixerSource,
    pending_sample_numerator: u128,
    pending_samples: u64,
    tone_phase: f32,
    core_tcycle_samples: VecDeque<[f32; AUDIO_OUTPUT_CHANNELS]>,
    core_resample_position: f64,
}

impl AudioMixer {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self {
            sample_rate_hz: sample_rate_hz.max(1),
            source: MixerSource::Silence,
            pending_sample_numerator: 0,
            pending_samples: 0,
            tone_phase: 0.0,
            core_tcycle_samples: VecDeque::new(),
            core_resample_position: 0.0,
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn set_sample_rate_hz(&mut self, sample_rate_hz: u32) {
        let next_rate = sample_rate_hz.max(1);
        if self.sample_rate_hz == next_rate {
            return;
        }
        self.sample_rate_hz = next_rate;

        if self.source != MixerSource::CoreApu {
            self.pending_sample_numerator = 0;
            self.pending_samples = 0;
        }
    }

    pub fn set_source(&mut self, source: MixerSource) {
        if self.source == source {
            return;
        }
        if source == MixerSource::CoreApu {
            self.pending_sample_numerator = 0;
            self.pending_samples = 0;
            self.tone_phase = 0.0;
        } else {
            self.core_tcycle_samples.clear();
            self.core_resample_position = 0.0;
        }
        self.source = source;
    }

    pub fn source(&self) -> MixerSource {
        self.source
    }

    pub fn push_tcycles(&mut self, tcycles: u64) {
        if self.source == MixerSource::CoreApu {
            return;
        }
        self.pending_sample_numerator = self
            .pending_sample_numerator
            .saturating_add((tcycles as u128).saturating_mul(self.sample_rate_hz as u128));
        let new_samples = self.pending_sample_numerator / (DMG_T_CYCLES_PER_SECOND as u128);
        self.pending_sample_numerator %= DMG_T_CYCLES_PER_SECOND as u128;
        self.pending_samples = self
            .pending_samples
            .saturating_add(u64::try_from(new_samples).unwrap_or(u64::MAX));
    }

    pub fn push_core_tcycle_samples(&mut self, tcycle_samples: &[f32]) {
        if self.source != MixerSource::CoreApu || tcycle_samples.len() < AUDIO_OUTPUT_CHANNELS {
            return;
        }

        for frame in tcycle_samples.chunks_exact(AUDIO_OUTPUT_CHANNELS) {
            if self.core_tcycle_samples.len() >= MAX_PENDING_CORE_TCYCLE_FRAMES {
                self.core_tcycle_samples.pop_front();
                self.core_resample_position = (self.core_resample_position - 1.0).max(0.0);
            }
            self.core_tcycle_samples.push_back([frame[0], frame[1]]);
        }
    }

    pub fn pending_samples(&self) -> u64 {
        if self.source == MixerSource::CoreApu {
            if self.core_tcycle_samples.len() < 2 {
                return 0;
            }
            let step = (DMG_T_CYCLES_PER_SECOND as f64) / (self.sample_rate_hz.max(1) as f64);
            let available =
                (self.core_tcycle_samples.len() - 1) as f64 - self.core_resample_position;
            if available < 0.0 {
                return 0;
            }
            let pending = (available / step).floor() as u128 + 1;
            return u64::try_from(pending).unwrap_or(u64::MAX);
        }
        self.pending_samples
    }

    pub fn drain_samples(&mut self, max_samples: usize) -> Vec<f32> {
        if max_samples == 0 {
            return Vec::new();
        }
        if self.source == MixerSource::CoreApu {
            return self.drain_core_apu_samples(max_samples);
        }
        if self.pending_samples == 0 {
            return Vec::new();
        }

        let frame_count = (self.pending_samples.min(max_samples as u64)) as usize;
        self.pending_samples = self.pending_samples.saturating_sub(frame_count as u64);
        let mut samples = Vec::with_capacity(frame_count.saturating_mul(AUDIO_OUTPUT_CHANNELS));

        if self.source == MixerSource::TestTone {
            let phase_step = TEST_TONE_HZ / (self.sample_rate_hz as f32);
            for _ in 0..frame_count {
                self.tone_phase += phase_step;
                if self.tone_phase >= 1.0 {
                    self.tone_phase -= 1.0;
                }
                let sample = if self.tone_phase < 0.5 {
                    TEST_TONE_AMPLITUDE
                } else {
                    -TEST_TONE_AMPLITUDE
                };
                samples.push(sample);
                samples.push(sample);
            }
        } else {
            samples.resize(frame_count.saturating_mul(AUDIO_OUTPUT_CHANNELS), 0.0);
        }

        samples
    }

    fn drain_core_apu_samples(&mut self, max_samples: usize) -> Vec<f32> {
        let mut samples = Vec::with_capacity(max_samples.saturating_mul(AUDIO_OUTPUT_CHANNELS));
        if self.core_tcycle_samples.len() < 2 {
            return samples;
        }

        let step = (DMG_T_CYCLES_PER_SECOND as f64) / (self.sample_rate_hz.max(1) as f64);
        while samples.len() < max_samples.saturating_mul(AUDIO_OUTPUT_CHANNELS) {
            let base_index = self.core_resample_position.floor() as usize;
            if base_index + 1 >= self.core_tcycle_samples.len() {
                break;
            }

            let frac = (self.core_resample_position - base_index as f64) as f32;
            let Some(&s0) = self.core_tcycle_samples.get(base_index) else {
                break;
            };
            let Some(&s1) = self.core_tcycle_samples.get(base_index + 1) else {
                break;
            };
            let interpolated_left = s0[0] + (s1[0] - s0[0]) * frac;
            let interpolated_right = s0[1] + (s1[1] - s0[1]) * frac;
            samples.push(interpolated_left.clamp(-1.0, 1.0));
            samples.push(interpolated_right.clamp(-1.0, 1.0));
            self.core_resample_position += step;
        }

        let consumed = self.core_resample_position.floor().max(0.0) as usize;
        if consumed > 0 {
            let remove = consumed.min(self.core_tcycle_samples.len());
            self.core_tcycle_samples.drain(..remove);
            self.core_resample_position -= remove as f64;
        }

        samples
    }

    pub fn drain_synced_samples(&mut self, pending_tcycles: u64, max_samples: usize) -> Vec<f32> {
        self.push_tcycles(pending_tcycles);
        self.drain_samples(max_samples)
    }

    pub fn drain_realtime_block(&mut self, pending_tcycles: u64, block_samples: usize) -> Vec<f32> {
        if block_samples == 0 {
            return Vec::new();
        }

        let mut samples = self.drain_synced_samples(pending_tcycles, block_samples);
        let wanted_scalars = block_samples.saturating_mul(AUDIO_OUTPUT_CHANNELS);
        if samples.len() < wanted_scalars {
            samples.resize(wanted_scalars, 0.0);
        }
        samples
    }

    pub fn drain_all_samples(&mut self) -> Vec<f32> {
        let max_samples = if self.pending_samples > usize::MAX as u64 {
            usize::MAX
        } else {
            self.pending_samples as usize
        };
        self.drain_samples(max_samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::HardwareModel;

    #[test]
    fn analog_profile_defaults_differ_between_models() {
        let dmg = AnalogCalibrationProfile::for_model(HardwareModel::Dmg);
        let mgb = AnalogCalibrationProfile::for_model(HardwareModel::Mgb);
        assert_ne!(dmg.hpf_coeff, mgb.hpf_coeff);
        assert_ne!(dmg.low_pass_alpha, mgb.low_pass_alpha);
        assert_ne!(dmg.channel_gain, mgb.channel_gain);
    }

    #[test]
    fn analog_profile_normalization_clamps_extreme_values() {
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
    fn analog_profile_normalization_replaces_nan_values() {
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

    #[test]
    fn tcycles_are_converted_to_samples_with_fractional_accumulation() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.push_tcycles(DMG_T_CYCLES_PER_SECOND / 2);
        assert_eq!(mixer.pending_samples(), 24_000);

        mixer.push_tcycles(DMG_T_CYCLES_PER_SECOND / 2);
        assert_eq!(mixer.pending_samples(), 48_000);
    }

    #[test]
    fn draining_samples_consumes_pending_budget() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.push_tcycles(DMG_T_CYCLES_PER_SECOND);

        let first = mixer.drain_samples(10_000);
        assert_eq!(first.len(), 20_000);
        assert_eq!(mixer.pending_samples(), 38_000);

        let rest = mixer.drain_all_samples();
        assert_eq!(rest.len(), 76_000);
        assert_eq!(mixer.pending_samples(), 0);
    }

    #[test]
    fn test_tone_source_emits_non_zero_samples() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.set_source(MixerSource::TestTone);
        mixer.push_tcycles(DMG_T_CYCLES_PER_SECOND / 100);
        let samples = mixer.drain_all_samples();
        assert!(!samples.is_empty());
        assert!(samples.iter().any(|sample| *sample != 0.0));
    }

    #[test]
    fn drain_synced_samples_pushes_tcycles_before_draining() {
        let mut mixer = AudioMixer::new(48_000);
        let tcycles = DMG_T_CYCLES_PER_SECOND / 100;
        let expected =
            ((tcycles as u128) * 48_000u128 / (DMG_T_CYCLES_PER_SECOND as u128)) as usize;
        let samples = mixer.drain_synced_samples(tcycles, 10_000);
        assert_eq!(samples.len(), expected * AUDIO_OUTPUT_CHANNELS);
        assert_eq!(mixer.pending_samples(), 0);
    }

    #[test]
    fn drain_realtime_block_pads_with_silence_when_budget_is_short() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.set_source(MixerSource::TestTone);

        let tcycles = DMG_T_CYCLES_PER_SECOND / 100;
        let produced =
            ((tcycles as u128) * 48_000u128 / (DMG_T_CYCLES_PER_SECOND as u128)) as usize;
        let samples = mixer.drain_realtime_block(tcycles, 600);
        let produced_scalars = produced * AUDIO_OUTPUT_CHANNELS;
        assert_eq!(samples.len(), 1_200);
        assert!(
            samples[..produced_scalars]
                .iter()
                .any(|sample| *sample != 0.0)
        );
        assert!(
            samples[produced_scalars..]
                .iter()
                .all(|sample| *sample == 0.0)
        );
        assert_eq!(mixer.pending_samples(), 0);
    }

    #[test]
    fn drain_realtime_block_returns_empty_for_zero_request() {
        let mut mixer = AudioMixer::new(48_000);
        let samples = mixer.drain_realtime_block(DMG_T_CYCLES_PER_SECOND, 0);
        assert!(samples.is_empty());
        assert_eq!(mixer.pending_samples(), 0);
    }

    #[test]
    fn core_apu_source_resamples_constant_tcycle_signal() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.set_source(MixerSource::CoreApu);

        let tcycles = (DMG_T_CYCLES_PER_SECOND / 100) as usize;
        let mut tcycle_samples = Vec::with_capacity(tcycles * AUDIO_OUTPUT_CHANNELS);
        for _ in 0..tcycles {
            tcycle_samples.push(0.5);
            tcycle_samples.push(0.5);
        }
        mixer.push_core_tcycle_samples(&tcycle_samples);
        let expected = mixer.pending_samples() as usize;

        let samples = mixer.drain_samples(10_000);
        assert_eq!(samples.len(), expected * AUDIO_OUTPUT_CHANNELS);
        assert!(samples.iter().all(|sample| (*sample - 0.5).abs() < 0.001));
    }

    #[test]
    fn core_apu_source_preserves_stereo_channels() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.set_source(MixerSource::CoreApu);

        let tcycles = (DMG_T_CYCLES_PER_SECOND / 200) as usize;
        let mut tcycle_samples = Vec::with_capacity(tcycles * AUDIO_OUTPUT_CHANNELS);
        for _ in 0..tcycles {
            tcycle_samples.push(0.75);
            tcycle_samples.push(-0.25);
        }
        mixer.push_core_tcycle_samples(&tcycle_samples);

        let samples = mixer.drain_samples(512);
        assert!(!samples.is_empty());
        assert_eq!(samples.len() % AUDIO_OUTPUT_CHANNELS, 0);
        for frame in samples.chunks_exact(AUDIO_OUTPUT_CHANNELS) {
            assert!((frame[0] - 0.75).abs() < 0.01);
            assert!((frame[1] + 0.25).abs() < 0.01);
        }
    }

    #[test]
    fn core_apu_source_ignores_synthetic_tcycle_budget_pushes() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.set_source(MixerSource::CoreApu);
        mixer.push_tcycles(DMG_T_CYCLES_PER_SECOND / 2);

        let samples = mixer.drain_samples(10_000);
        assert!(samples.is_empty());
        assert_eq!(mixer.pending_samples(), 0);
    }

    #[test]
    fn core_apu_realtime_block_pads_with_silence_when_input_is_short() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.set_source(MixerSource::CoreApu);

        mixer.push_core_tcycle_samples(&vec![1.0; 1_024]);
        let produced = mixer.pending_samples() as usize;
        let samples = mixer.drain_realtime_block(0, 256);
        let produced_scalars = produced * AUDIO_OUTPUT_CHANNELS;

        assert_eq!(samples.len(), 512);
        assert!(produced < 256);
        assert!(
            samples[..produced_scalars]
                .iter()
                .all(|sample| *sample > 0.1)
        );
        assert!(
            samples[produced_scalars..]
                .iter()
                .all(|sample| *sample == 0.0)
        );
    }

    #[test]
    fn core_apu_sample_rate_change_preserves_resampler_queue_continuity() {
        let mut mixer = AudioMixer::new(48_000);
        mixer.set_source(MixerSource::CoreApu);

        let tcycles = (DMG_T_CYCLES_PER_SECOND / 120) as usize;
        let mut tcycle_samples = Vec::with_capacity(tcycles * AUDIO_OUTPUT_CHANNELS);
        for i in 0..tcycles {
            let left = if (i / 8) % 2 == 0 { 0.4 } else { -0.35 };
            let right = if (i / 16) % 2 == 0 { -0.2 } else { 0.25 };
            tcycle_samples.push(left);
            tcycle_samples.push(right);
        }
        mixer.push_core_tcycle_samples(&tcycle_samples);

        let first = mixer.drain_samples(128);
        assert_eq!(first.len(), 256);
        assert!(first.iter().any(|sample| sample.abs() > 0.01));

        let pending_before = mixer.pending_samples();
        assert!(pending_before > 0);

        mixer.set_sample_rate_hz(44_100);

        let pending_after = mixer.pending_samples();
        assert!(
            pending_after > 0,
            "expected queued tcycle audio to remain after rate change"
        );

        let second = mixer.drain_samples(128);
        assert_eq!(second.len(), 256);
        assert!(second.iter().all(|sample| sample.is_finite()));
        assert!(second.iter().any(|sample| sample.abs() > 0.01));
        assert_eq!(second.len() % AUDIO_OUTPUT_CHANNELS, 0);
    }

    #[test]
    fn playback_underrun_estimate_is_zero_when_queue_covers_elapsed_playback() {
        let underrun = estimate_playback_underrun_samples(480, Duration::from_millis(5), 48_000);
        assert_eq!(underrun, 0);
    }

    #[test]
    fn playback_underrun_estimate_is_positive_when_elapsed_consumption_exceeds_queue() {
        let underrun = estimate_playback_underrun_samples(100, Duration::from_millis(5), 48_000);
        assert_eq!(underrun, 140);
    }

    #[test]
    fn adaptive_queue_increases_target_when_underruns_appear() {
        let mut controller =
            AdaptiveQueueController::new(4_096, 0, 0, AdaptiveQueueOptions::default());
        let update = controller.update(500, 2_000, 10, 512);

        assert!(update.changed);
        assert_eq!(update.target_samples, 5_120);
        assert_eq!(update.window_underrun_samples, 10);
    }

    #[test]
    fn adaptive_queue_severe_underrun_uses_larger_increase_step() {
        let mut controller =
            AdaptiveQueueController::new(4_096, 0, 0, AdaptiveQueueOptions::default());
        let update = controller.update(500, 1_000, 800, 512);

        assert!(update.changed);
        assert_eq!(update.target_samples, 6_144);
        assert_eq!(update.window_underrun_samples, 800);
    }

    #[test]
    fn adaptive_queue_decreases_target_after_stable_windows() {
        let options = AdaptiveQueueOptions {
            window_ms: 100,
            min_target_samples: 2_048,
            max_target_samples: 16_384,
            increase_step_samples: 256,
            decrease_step_samples: 128,
            decrease_stable_windows: 2,
            decrease_queue_headroom_samples: 256,
        };
        let mut controller = AdaptiveQueueController::new(4_096, 0, 0, options);

        let first = controller.update(100, 4_600, 0, 512);
        assert!(!first.changed);
        assert_eq!(first.target_samples, 4_096);

        let second = controller.update(200, 4_600, 0, 512);
        assert!(second.changed);
        assert_eq!(second.target_samples, 3_968);

        let third = controller.update(300, 4_400, 0, 512);
        assert!(!third.changed);
        assert_eq!(third.target_samples, 3_968);

        let fourth = controller.update(400, 4_400, 0, 512);
        assert!(fourth.changed);
        assert_eq!(fourth.target_samples, 3_840);
    }

    #[test]
    fn adaptive_queue_respects_min_and_max_limits() {
        let options = AdaptiveQueueOptions {
            window_ms: 100,
            min_target_samples: 1_024,
            max_target_samples: 2_048,
            increase_step_samples: 400,
            decrease_step_samples: 800,
            decrease_stable_windows: 1,
            decrease_queue_headroom_samples: 1,
        };
        let mut controller = AdaptiveQueueController::new(1_900, 0, 0, options);

        let increased = controller.update(100, 100, 2_000, 512);
        assert_eq!(increased.target_samples, 2_048);

        let decreased = controller.update(200, 5_000, 2_000, 512);
        assert_eq!(decreased.target_samples, 1_248);

        let clamped = controller.update(300, 5_000, 2_000, 512);
        assert_eq!(clamped.target_samples, 1_024);
    }
}
