use std::time::{SystemTime, UNIX_EPOCH};

pub(super) trait RtcClock {
    fn now_epoch_secs(&self) -> u64;
}

pub(super) struct SystemRtcClock;

impl RtcClock for SystemRtcClock {
    fn now_epoch_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
