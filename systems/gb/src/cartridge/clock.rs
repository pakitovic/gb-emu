use std::time::{SystemTime, UNIX_EPOCH};

pub(super) trait RtcClock {
    fn now_epoch_secs(&self) -> u64;
}

pub(super) struct SystemRtcClock;
pub(super) struct FixedRtcClock {
    now_epoch_secs: u64,
}

impl FixedRtcClock {
    pub(super) fn new(now_epoch_secs: u64) -> Self {
        Self { now_epoch_secs }
    }
}

impl RtcClock for SystemRtcClock {
    fn now_epoch_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl RtcClock for FixedRtcClock {
    fn now_epoch_secs(&self) -> u64 {
        self.now_epoch_secs
    }
}
