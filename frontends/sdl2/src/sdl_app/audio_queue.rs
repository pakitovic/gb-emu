use super::{
    AUDIO_QUEUE_HARD_MAX_SAMPLES, AUDIO_QUEUE_TARGET_INITIAL_SAMPLES,
    AUDIO_QUEUE_TARGET_MAX_SAMPLES, AUDIO_QUEUE_TARGET_MIN_SAMPLES, AUDIO_REFILL_BLOCK_SAMPLES,
    AUDIO_REFILL_MAX_BLOCKS,
};
use gb_runtime::audio::{
    AdaptiveQueueController, AdaptiveQueueOptions, estimate_playback_underrun_samples,
};
use gb_runtime::session::RuntimeSession;
use std::time::Instant;

pub(super) fn refill_audio_queue(
    audio_queue: &sdl2::audio::AudioQueue<f32>,
    session: &mut RuntimeSession,
    queue_state: &mut SdlAudioQueueState,
    now: Instant,
) {
    let channel_count = usize::from(audio_queue.spec().channels.max(1));

    let mut queued_samples = queued_audio_samples(audio_queue);

    if queued_samples > AUDIO_QUEUE_HARD_MAX_SAMPLES {
        audio_queue.clear();
        queued_samples = 0;
    }

    let target_samples = queue_state.observe_and_update_target(now, queued_samples);

    let mut guard = 0;
    while queued_samples < target_samples && guard < AUDIO_REFILL_MAX_BLOCKS {
        let wanted = target_samples
            .saturating_sub(queued_samples)
            .min(AUDIO_REFILL_BLOCK_SAMPLES);
        let samples = session.drain_audio_realtime_block(wanted);
        if samples.is_empty() {
            break;
        }
        if audio_queue.queue_audio(&samples).is_err() {
            break;
        }
        let enqueued_frames = samples.len() / channel_count;
        queued_samples = queued_samples.saturating_add(enqueued_frames);
        guard += 1;
    }

    queue_state.commit_refill(now, queued_samples);
}

fn queued_audio_samples(audio_queue: &sdl2::audio::AudioQueue<f32>) -> usize {
    let sample_size_bytes = std::mem::size_of::<f32>();
    let channel_count = usize::from(audio_queue.spec().channels.max(1));
    (audio_queue.size() as usize) / sample_size_bytes / channel_count
}

pub(super) struct SdlAudioQueueState {
    sample_rate_hz: u32,
    start_instant: Instant,
    last_refill_instant: Instant,
    last_queue_after_refill_samples: usize,
    total_underrun_samples: u64,
    adaptive_target: AdaptiveQueueController,
}

impl SdlAudioQueueState {
    pub(super) fn new(sample_rate_hz: u32, now: Instant) -> Self {
        let options = AdaptiveQueueOptions {
            min_target_samples: AUDIO_QUEUE_TARGET_MIN_SAMPLES,
            max_target_samples: AUDIO_QUEUE_TARGET_MAX_SAMPLES,
            ..AdaptiveQueueOptions::default()
        };
        let adaptive_target =
            AdaptiveQueueController::new(AUDIO_QUEUE_TARGET_INITIAL_SAMPLES, 0, 0, options);
        Self {
            sample_rate_hz: sample_rate_hz.max(1),
            start_instant: now,
            last_refill_instant: now,
            last_queue_after_refill_samples: 0,
            total_underrun_samples: 0,
            adaptive_target,
        }
    }

    fn observe_and_update_target(
        &mut self,
        now: Instant,
        queued_samples_before_refill: usize,
    ) -> usize {
        let elapsed = now.saturating_duration_since(self.last_refill_instant);
        let underrun_delta = estimate_playback_underrun_samples(
            self.last_queue_after_refill_samples,
            elapsed,
            self.sample_rate_hz,
        );
        self.total_underrun_samples = self.total_underrun_samples.saturating_add(underrun_delta);
        let now_ms = u64::try_from(
            now.saturating_duration_since(self.start_instant)
                .as_millis(),
        )
        .unwrap_or(u64::MAX);
        let update = self.adaptive_target.update(
            now_ms,
            queued_samples_before_refill,
            self.total_underrun_samples,
            AUDIO_REFILL_BLOCK_SAMPLES,
        );
        update.target_samples
    }

    fn commit_refill(&mut self, now: Instant, queued_samples_after_refill: usize) {
        self.last_refill_instant = now;
        self.last_queue_after_refill_samples = queued_samples_after_refill;
    }
}
