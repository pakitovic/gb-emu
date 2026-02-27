use super::WebEmulator;
use gb_runtime::audio::{AudioResamplerQuality, MixerSource};
use wasm_bindgen::prelude::*;

pub(super) fn parse_audio_resampler_quality(quality: &str) -> Option<AudioResamplerQuality> {
    match quality {
        "linear" => Some(AudioResamplerQuality::Linear),
        "cubic" => Some(AudioResamplerQuality::Cubic),
        _ => None,
    }
}

#[wasm_bindgen]
impl WebEmulator {
    pub fn set_audio_sample_rate(&mut self, sample_rate_hz: u32) {
        self.session.set_audio_sample_rate_hz(sample_rate_hz.max(1));
    }

    pub fn audio_resampler_quality(&self) -> String {
        match self.session.audio_resampler_quality() {
            AudioResamplerQuality::Linear => "linear",
            AudioResamplerQuality::Cubic => "cubic",
        }
        .to_string()
    }

    pub fn set_audio_resampler_quality(&mut self, quality: &str) -> Result<(), JsValue> {
        let Some(quality) = parse_audio_resampler_quality(quality) else {
            return Err(JsValue::from_str(
                "Invalid audio resampler quality (expected 'linear' or 'cubic')",
            ));
        };
        self.session.set_audio_resampler_quality(quality);
        Ok(())
    }

    pub fn set_audio_test_tone_enabled(&mut self, enabled: bool) {
        self.session.set_audio_source(if enabled {
            MixerSource::TestTone
        } else {
            MixerSource::CoreApu
        });
    }

    pub fn drain_audio_samples(&mut self, max_samples: u32) -> Vec<f32> {
        self.session.drain_audio_samples(max_samples as usize)
    }

    pub fn drain_audio_samples_realtime(&mut self, block_samples: u32) -> Vec<f32> {
        self.session
            .drain_audio_realtime_block(block_samples as usize)
    }
}
