use std::time::{Duration, Instant};

pub(super) struct SaveAutosaveDebouncer {
    debounce: Duration,
    dirty_since: Option<Instant>,
}

impl SaveAutosaveDebouncer {
    pub(super) fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            dirty_since: None,
        }
    }

    pub(super) fn update_and_should_flush(&mut self, save_dirty: bool, now: Instant) -> bool {
        if !save_dirty {
            self.dirty_since = None;
            return false;
        }

        let dirty_since = self.dirty_since.get_or_insert(now);
        now.saturating_duration_since(*dirty_since) >= self.debounce
    }

    pub(super) fn mark_flushed(&mut self) {
        self.dirty_since = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_flush_when_save_is_clean() {
        let mut debouncer = SaveAutosaveDebouncer::new(Duration::from_secs(2));
        let now = Instant::now();

        assert!(!debouncer.update_and_should_flush(false, now));
        assert!(!debouncer.update_and_should_flush(false, now + Duration::from_secs(10)));
    }

    #[test]
    fn flushes_only_after_dirty_debounce_elapsed() {
        let mut debouncer = SaveAutosaveDebouncer::new(Duration::from_secs(2));
        let start = Instant::now();

        assert!(!debouncer.update_and_should_flush(true, start));
        assert!(!debouncer.update_and_should_flush(true, start + Duration::from_millis(1500)));
        assert!(debouncer.update_and_should_flush(true, start + Duration::from_secs(2)));
    }

    #[test]
    fn mark_flushed_restarts_debounce_window() {
        let mut debouncer = SaveAutosaveDebouncer::new(Duration::from_secs(2));
        let start = Instant::now();

        assert!(!debouncer.update_and_should_flush(true, start));
        assert!(debouncer.update_and_should_flush(true, start + Duration::from_secs(2)));

        debouncer.mark_flushed();

        let after_flush = start + Duration::from_secs(3);
        assert!(!debouncer.update_and_should_flush(true, after_flush));
        assert!(debouncer.update_and_should_flush(true, after_flush + Duration::from_secs(2)));
    }

    #[test]
    fn clean_transition_clears_pending_dirty_timer() {
        let mut debouncer = SaveAutosaveDebouncer::new(Duration::from_secs(2));
        let start = Instant::now();

        assert!(!debouncer.update_and_should_flush(true, start));
        assert!(!debouncer.update_and_should_flush(false, start + Duration::from_secs(1)));
        assert!(!debouncer.update_and_should_flush(true, start + Duration::from_secs(1)));
        assert!(
            !debouncer.update_and_should_flush(true, start + Duration::from_secs(2)),
            "dirty timer should restart after a clean period"
        );
        assert!(debouncer.update_and_should_flush(true, start + Duration::from_secs(3)));
    }
}
