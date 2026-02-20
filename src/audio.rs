use crate::timing::DMG_T_CYCLES_PER_SECOND;

const TEST_TONE_HZ: f32 = 440.0;
const TEST_TONE_AMPLITUDE: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixerSource {
    Silence,
    TestTone,
}

pub struct AudioMixer {
    sample_rate_hz: u32,
    source: MixerSource,
    pending_sample_numerator: u128,
    pending_samples: u64,
    tone_phase: f32,
}

impl AudioMixer {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self {
            sample_rate_hz: sample_rate_hz.max(1),
            source: MixerSource::Silence,
            pending_sample_numerator: 0,
            pending_samples: 0,
            tone_phase: 0.0,
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn set_source(&mut self, source: MixerSource) {
        self.source = source;
    }

    pub fn source(&self) -> MixerSource {
        self.source
    }

    pub fn push_tcycles(&mut self, tcycles: u64) {
        self.pending_sample_numerator = self
            .pending_sample_numerator
            .saturating_add((tcycles as u128).saturating_mul(self.sample_rate_hz as u128));
        let new_samples = self.pending_sample_numerator / (DMG_T_CYCLES_PER_SECOND as u128);
        self.pending_sample_numerator %= DMG_T_CYCLES_PER_SECOND as u128;
        self.pending_samples = self
            .pending_samples
            .saturating_add(u64::try_from(new_samples).unwrap_or(u64::MAX));
    }

    pub fn pending_samples(&self) -> u64 {
        self.pending_samples
    }

    pub fn drain_samples(&mut self, max_samples: usize) -> Vec<f32> {
        if max_samples == 0 || self.pending_samples == 0 {
            return Vec::new();
        }

        let count = (self.pending_samples.min(max_samples as u64)) as usize;
        self.pending_samples = self.pending_samples.saturating_sub(count as u64);
        let mut samples = vec![0.0f32; count];

        if self.source == MixerSource::TestTone {
            let phase_step = TEST_TONE_HZ / (self.sample_rate_hz as f32);
            for sample in &mut samples {
                self.tone_phase += phase_step;
                if self.tone_phase >= 1.0 {
                    self.tone_phase -= 1.0;
                }
                *sample = if self.tone_phase < 0.5 {
                    TEST_TONE_AMPLITUDE
                } else {
                    -TEST_TONE_AMPLITUDE
                };
            }
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
        assert_eq!(first.len(), 10_000);
        assert_eq!(mixer.pending_samples(), 38_000);

        let rest = mixer.drain_all_samples();
        assert_eq!(rest.len(), 38_000);
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
}
