use super::super::Bus;
use crate::apu::ApuTestDebugState;

impl Bus {
    #[cfg(test)]
    pub(in crate::memory) fn apu_test_state(&self) -> ApuTestDebugState {
        self.apu.test_debug_state()
    }
}
