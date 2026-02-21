use super::*;

impl ApuState {
    pub(crate) fn drain_tcycle_samples(&mut self) -> Vec<f32> {
        if self.pending_tcycle_samples.is_empty() {
            return Vec::new();
        }
        self.pending_tcycle_samples.drain(..).collect()
    }

    pub(crate) fn set_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.capture_tcycle_stream = enabled;
        if !enabled {
            self.pending_tcycle_samples.clear();
        }
    }
}
