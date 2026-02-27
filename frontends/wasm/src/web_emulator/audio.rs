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
        let next_rate = sample_rate_hz.max(1);
        self.session.set_audio_sample_rate_hz(next_rate);
        self.audio_queue_controller
            .set_sample_rate_hz(next_rate, self.audio_queue_clock_ms);
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

    pub fn audio_queue_refill_block_samples(&self) -> u32 {
        u32::try_from(self.audio_queue_controller.refill_block_samples()).unwrap_or(u32::MAX)
    }

    pub fn audio_queue_max_refill_blocks(&self) -> u32 {
        u32::try_from(self.audio_queue_controller.max_refill_blocks()).unwrap_or(u32::MAX)
    }

    pub fn audio_queue_clear_required(&self) -> bool {
        self.audio_queue_controller.clear_required()
    }

    pub fn observe_audio_queue_target(&mut self, now_ms: f64, queued_samples: u32) -> u32 {
        let now_ms = normalize_host_now_ms(now_ms);
        self.audio_queue_clock_ms = now_ms;
        let observation = self
            .audio_queue_controller
            .observe_and_update_target(now_ms, queued_samples as usize);
        u32::try_from(observation.target_samples).unwrap_or(u32::MAX)
    }

    pub fn commit_audio_queue_refill(&mut self, now_ms: f64, queued_samples_after_refill: u32) {
        let now_ms = normalize_host_now_ms(now_ms);
        self.audio_queue_clock_ms = now_ms;
        self.audio_queue_controller
            .commit_refill(now_ms, queued_samples_after_refill as usize);
    }
}

fn normalize_host_now_ms(now_ms: f64) -> u64 {
    if !now_ms.is_finite() || now_ms.is_sign_negative() {
        return 0;
    }
    now_ms.floor() as u64
}
