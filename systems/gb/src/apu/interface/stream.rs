use super::*;

impl ApuState {
    pub(crate) fn drain_tcycle_samples(&mut self) -> Vec<f32> {
        if self.stream.pending_tcycle_samples.is_empty() {
            return Vec::new();
        }
        self.stream.pending_tcycle_samples.drain(..).collect()
    }

    pub(crate) fn set_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.stream.capture_tcycle_stream = enabled;
        if !enabled {
            self.stream.pending_tcycle_samples.clear();
        }
    }
}
