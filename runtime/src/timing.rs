use gb_emu::timing::{DMG_T_CYCLES_PER_FRAME, DMG_T_CYCLES_PER_SECOND};
use std::time::Duration;

const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

#[derive(Clone, Copy, Debug)]
pub struct FramePacerConfig {
    pub max_catch_up_frames: u32,
}

impl Default for FramePacerConfig {
    fn default() -> Self {
        Self {
            max_catch_up_frames: 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FramePacer {
    pending_host_tcycles: u64,
    pending_host_tcycles_fp: u128,
    max_catch_up_tcycles: u64,
    audio_clock_tcycles: u64,
    pending_audio_tcycles: u64,
}

impl Default for FramePacer {
    fn default() -> Self {
        Self::new(FramePacerConfig::default())
    }
}

impl FramePacer {
    pub fn new(config: FramePacerConfig) -> Self {
        let max_frames = config.max_catch_up_frames.max(1);
        Self {
            pending_host_tcycles: 0,
            pending_host_tcycles_fp: 0,
            max_catch_up_tcycles: (max_frames as u64).saturating_mul(DMG_T_CYCLES_PER_FRAME),
            audio_clock_tcycles: 0,
            pending_audio_tcycles: 0,
        }
    }

    pub fn push_host_time(&mut self, elapsed: Duration) {
        self.pending_host_tcycles_fp = self.pending_host_tcycles_fp.saturating_add(
            elapsed
                .as_nanos()
                .saturating_mul(DMG_T_CYCLES_PER_SECOND as u128),
        );

        let whole_tcycles = self.pending_host_tcycles_fp / NANOSECONDS_PER_SECOND;
        self.pending_host_tcycles_fp %= NANOSECONDS_PER_SECOND;
        let whole_tcycles_u64 = u64::try_from(whole_tcycles).unwrap_or(u64::MAX);
        self.pending_host_tcycles = self.pending_host_tcycles.saturating_add(whole_tcycles_u64);

        if self.pending_host_tcycles > self.max_catch_up_tcycles {
            self.pending_host_tcycles = self.max_catch_up_tcycles;
            self.pending_host_tcycles_fp = 0;
        }
    }

    pub fn has_frame_budget(&self) -> bool {
        self.pending_host_tcycles >= DMG_T_CYCLES_PER_FRAME
    }

    pub fn frame_budget_count(&self) -> u32 {
        (self.pending_host_tcycles / DMG_T_CYCLES_PER_FRAME) as u32
    }

    pub fn consume_emulated_cycles(&mut self, emulated_tcycles: u64) {
        self.pending_host_tcycles = self.pending_host_tcycles.saturating_sub(emulated_tcycles);
        self.audio_clock_tcycles = self.audio_clock_tcycles.wrapping_add(emulated_tcycles);
        self.pending_audio_tcycles = self.pending_audio_tcycles.saturating_add(emulated_tcycles);
    }

    pub fn duration_until_next_frame(&self) -> Duration {
        if self.has_frame_budget() {
            return Duration::ZERO;
        }

        let missing_tcycles = (DMG_T_CYCLES_PER_FRAME - self.pending_host_tcycles) as u128;
        let nanos = (missing_tcycles * NANOSECONDS_PER_SECOND)
            .saturating_add((DMG_T_CYCLES_PER_SECOND as u128).saturating_sub(1))
            / (DMG_T_CYCLES_PER_SECOND as u128);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    pub fn pending_host_tcycles(&self) -> u64 {
        self.pending_host_tcycles
    }

    pub fn audio_clock_tcycles(&self) -> u64 {
        self.audio_clock_tcycles
    }

    pub fn drain_audio_tcycles(&mut self) -> u64 {
        let pending = self.pending_audio_tcycles;
        self.pending_audio_tcycles = 0;
        pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_budget_accumulates_from_host_time() {
        let mut pacer = FramePacer::default();

        let frame_nanos_floor = ((DMG_T_CYCLES_PER_FRAME as u128) * NANOSECONDS_PER_SECOND)
            / (DMG_T_CYCLES_PER_SECOND as u128);
        pacer.push_host_time(Duration::from_nanos(frame_nanos_floor as u64));
        assert!(!pacer.has_frame_budget());

        pacer.push_host_time(Duration::from_nanos(1));
        assert!(pacer.has_frame_budget());
        assert_eq!(pacer.frame_budget_count(), 1);
    }

    #[test]
    fn frame_budget_is_clamped_by_max_catch_up() {
        let mut pacer = FramePacer::new(FramePacerConfig {
            max_catch_up_frames: 3,
        });

        pacer.push_host_time(Duration::from_secs(1));
        assert_eq!(pacer.frame_budget_count(), 3);
        assert_eq!(pacer.pending_host_tcycles(), DMG_T_CYCLES_PER_FRAME * 3);
    }

    #[test]
    fn consume_cycles_updates_audio_clock_and_pending_budget() {
        let mut pacer = FramePacer::default();
        pacer.push_host_time(Duration::from_millis(17));
        assert!(pacer.pending_host_tcycles() > 0);

        pacer.consume_emulated_cycles(10_000);
        assert_eq!(pacer.audio_clock_tcycles(), 10_000);
        assert_eq!(pacer.drain_audio_tcycles(), 10_000);
        assert_eq!(pacer.drain_audio_tcycles(), 0);
    }

    #[test]
    fn duration_until_next_frame_is_zero_when_budget_is_ready() {
        let mut pacer = FramePacer::default();
        pacer.push_host_time(Duration::from_millis(17));
        assert_eq!(pacer.duration_until_next_frame(), Duration::ZERO);
    }
}
