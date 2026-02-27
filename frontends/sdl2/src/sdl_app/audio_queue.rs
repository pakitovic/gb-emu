use gb_runtime::audio_queue::{
    AudioQueueController, AudioQueueObservation, AudioQueueRefillConfig,
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
    let now_ms = queue_state.now_ms(now);
    let queued_samples_before_refill = queued_audio_samples(audio_queue);
    let AudioQueueObservation {
        target_samples,
        normalized_queued_samples,
        queue_cleared,
        ..
    } = queue_state.observe_and_update_target(now_ms, queued_samples_before_refill);
    let mut queued_samples = normalized_queued_samples;
    if queue_cleared {
        audio_queue.clear();
    }

    let mut guard = 0;
    while queued_samples < target_samples && guard < queue_state.max_refill_blocks() {
        let wanted = target_samples
            .saturating_sub(queued_samples)
            .min(queue_state.refill_block_samples());
        let samples = drain_refill_samples(session, wanted);
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

    queue_state.commit_refill(now_ms, queued_samples);
}

fn drain_refill_samples(session: &mut RuntimeSession, wanted_samples: usize) -> Vec<f32> {
    // Queue-based backends should enqueue only currently available emulated audio.
    // Padding with synthetic silence here causes audible discontinuities under load.
    session.drain_audio_samples(wanted_samples)
}

fn queued_audio_samples(audio_queue: &sdl2::audio::AudioQueue<f32>) -> usize {
    let sample_size_bytes = std::mem::size_of::<f32>();
    let channel_count = usize::from(audio_queue.spec().channels.max(1));
    (audio_queue.size() as usize) / sample_size_bytes / channel_count
}

pub(super) struct SdlAudioQueueState {
    start_instant: Instant,
    controller: AudioQueueController,
}

impl SdlAudioQueueState {
    pub(super) fn new(sample_rate_hz: u32, now: Instant) -> Self {
        let config = AudioQueueRefillConfig::default();
        let controller = AudioQueueController::new(sample_rate_hz, 0, config);
        Self {
            start_instant: now,
            controller,
        }
    }

    fn now_ms(&self, now: Instant) -> u64 {
        u64::try_from(
            now.saturating_duration_since(self.start_instant)
                .as_millis(),
        )
        .unwrap_or(u64::MAX)
    }

    fn observe_and_update_target(
        &mut self,
        now_ms: u64,
        queued_samples_before_refill: usize,
    ) -> AudioQueueObservation {
        self.controller
            .observe_and_update_target(now_ms, queued_samples_before_refill)
    }

    fn commit_refill(&mut self, now_ms: u64, queued_samples_after_refill: usize) {
        self.controller
            .commit_refill(now_ms, queued_samples_after_refill);
    }

    fn refill_block_samples(&self) -> usize {
        self.controller.refill_block_samples()
    }

    fn max_refill_blocks(&self) -> usize {
        self.controller.max_refill_blocks()
    }
}

#[cfg(test)]
mod tests {
    use super::drain_refill_samples;
    use gb_emu::cartridge::Cartridge;
    use gb_emu::gameboy::GameBoy;
    use gb_emu::timing::DMG_T_CYCLES_PER_SECOND;
    use gb_runtime::audio::MixerSource;
    use gb_runtime::session::RuntimeSession;

    fn make_rom_32kb() -> Vec<u8> {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        rom
    }

    #[test]
    fn drain_refill_samples_does_not_pad_with_synthetic_silence() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("valid ROM should load");
        let gb = GameBoy::new(cartridge);
        let mut session = RuntimeSession::new(gb, 48_000);
        session.set_audio_source(MixerSource::TestTone);
        session.consume_emulated_cycles(DMG_T_CYCLES_PER_SECOND / 100);

        let first = drain_refill_samples(&mut session, 600);
        assert!(!first.is_empty());
        assert_eq!(first.len() % 2, 0);
        assert!(first.len() < 600 * 2);
        assert!(first.iter().any(|sample| *sample != 0.0));

        let second = drain_refill_samples(&mut session, 600);
        assert!(second.is_empty());
    }
}
