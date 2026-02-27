use crate::audio::{
    AdaptiveQueueController, AdaptiveQueueOptions, AdaptiveQueueUpdate,
    estimate_playback_underrun_samples,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioQueueRefillConfig {
    pub initial_target_samples: usize,
    pub min_target_samples: usize,
    pub max_target_samples: usize,
    pub hard_max_samples: usize,
    pub refill_block_samples: usize,
    pub max_refill_blocks: usize,
    pub adaptive_options: AdaptiveQueueOptions,
}

impl Default for AudioQueueRefillConfig {
    fn default() -> Self {
        Self {
            initial_target_samples: 4_096,
            min_target_samples: 2_048,
            max_target_samples: 16_384,
            hard_max_samples: 32_768,
            refill_block_samples: 512,
            max_refill_blocks: 32,
            adaptive_options: AdaptiveQueueOptions::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioQueueObservation {
    pub target_samples: usize,
    pub normalized_queued_samples: usize,
    pub queue_cleared: bool,
    pub window_underrun_samples: u64,
}

#[derive(Clone, Debug)]
pub struct AudioQueueController {
    config: AudioQueueRefillConfig,
    sample_rate_hz: u32,
    last_refill_ms: u64,
    last_queue_after_refill_samples: usize,
    total_underrun_samples: u64,
    adaptive_target: AdaptiveQueueController,
    clear_required: bool,
}

impl AudioQueueController {
    pub fn new(sample_rate_hz: u32, now_ms: u64, config: AudioQueueRefillConfig) -> Self {
        let mut normalized_config = config;
        normalized_config.initial_target_samples = normalized_config.initial_target_samples.max(1);
        normalized_config.min_target_samples = normalized_config.min_target_samples.max(1);
        normalized_config.max_target_samples = normalized_config
            .max_target_samples
            .max(normalized_config.min_target_samples);
        normalized_config.initial_target_samples = normalized_config
            .initial_target_samples
            .max(normalized_config.min_target_samples)
            .min(normalized_config.max_target_samples);
        normalized_config.hard_max_samples = normalized_config.hard_max_samples.max(1);
        normalized_config.refill_block_samples = normalized_config.refill_block_samples.max(1);
        normalized_config.max_refill_blocks = normalized_config.max_refill_blocks.max(1);

        let mut options = normalized_config.adaptive_options;
        options.min_target_samples = normalized_config.min_target_samples;
        options.max_target_samples = normalized_config.max_target_samples;
        let adaptive_target = AdaptiveQueueController::new(
            normalized_config.initial_target_samples,
            now_ms,
            0,
            options,
        );
        normalized_config.adaptive_options = adaptive_target.options();

        Self {
            config: normalized_config,
            sample_rate_hz: sample_rate_hz.max(1),
            last_refill_ms: now_ms,
            last_queue_after_refill_samples: 0,
            total_underrun_samples: 0,
            adaptive_target,
            clear_required: false,
        }
    }

    pub fn config(&self) -> AudioQueueRefillConfig {
        self.config
    }

    pub fn refill_block_samples(&self) -> usize {
        self.config.refill_block_samples
    }

    pub fn max_refill_blocks(&self) -> usize {
        self.config.max_refill_blocks
    }

    pub fn clear_required(&self) -> bool {
        self.clear_required
    }

    pub fn total_underrun_samples(&self) -> u64 {
        self.total_underrun_samples
    }

    pub fn observe_and_update_target(
        &mut self,
        now_ms: u64,
        queued_samples_before_refill: usize,
    ) -> AudioQueueObservation {
        let mut normalized_queued_samples = queued_samples_before_refill;
        self.clear_required = false;
        if normalized_queued_samples > self.config.hard_max_samples {
            normalized_queued_samples = 0;
            self.clear_required = true;
        }

        let elapsed_ms = now_ms.saturating_sub(self.last_refill_ms);
        let underrun_delta = estimate_playback_underrun_samples(
            self.last_queue_after_refill_samples,
            Duration::from_millis(elapsed_ms),
            self.sample_rate_hz,
        );
        self.total_underrun_samples = self.total_underrun_samples.saturating_add(underrun_delta);

        let update: AdaptiveQueueUpdate = self.adaptive_target.update(
            now_ms,
            normalized_queued_samples,
            self.total_underrun_samples,
            self.config.refill_block_samples,
        );

        AudioQueueObservation {
            target_samples: update.target_samples,
            normalized_queued_samples,
            queue_cleared: self.clear_required,
            window_underrun_samples: update.window_underrun_samples,
        }
    }

    pub fn commit_refill(&mut self, now_ms: u64, queued_samples_after_refill: usize) {
        self.last_refill_ms = now_ms;
        self.last_queue_after_refill_samples = queued_samples_after_refill;
        self.clear_required = false;
    }

    pub fn set_sample_rate_hz(&mut self, sample_rate_hz: u32, now_ms: u64) {
        self.sample_rate_hz = sample_rate_hz.max(1);
        self.adaptive_target
            .reset(now_ms, self.total_underrun_samples);
        self.last_refill_ms = now_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_max_overflow_requests_queue_clear_and_normalizes_depth() {
        let mut controller =
            AudioQueueController::new(48_000, 0, AudioQueueRefillConfig::default());

        let observation = controller.observe_and_update_target(500, 40_000);

        assert!(observation.queue_cleared);
        assert_eq!(observation.normalized_queued_samples, 0);
        assert!(controller.clear_required());
    }

    #[test]
    fn commit_refill_updates_timing_reference_for_future_underrun_estimation() {
        let mut with_commit =
            AudioQueueController::new(48_000, 0, AudioQueueRefillConfig::default());
        with_commit.commit_refill(0, 240);
        with_commit.observe_and_update_target(10, 240);

        let mut without_commit =
            AudioQueueController::new(48_000, 0, AudioQueueRefillConfig::default());
        without_commit.observe_and_update_target(10, 0);

        assert_eq!(with_commit.total_underrun_samples(), 240);
        assert_eq!(without_commit.total_underrun_samples(), 480);
    }

    #[test]
    fn set_sample_rate_resets_adaptive_window_reference() {
        let mut controller =
            AudioQueueController::new(48_000, 0, AudioQueueRefillConfig::default());

        controller.observe_and_update_target(400, 0);

        controller.set_sample_rate_hz(44_100, 400);

        let before_new_window = controller.observe_and_update_target(500, 0);
        assert_eq!(before_new_window.window_underrun_samples, 0);

        let after_new_window = controller.observe_and_update_target(900, 0);
        assert!(after_new_window.window_underrun_samples > 0);
    }
}
