use super::*;

impl ApuState {
    pub(crate) fn step_frame_sequencer_from_divider(&mut self, old_div: u16, new_div: u16) {
        if !self.enabled {
            return;
        }

        let old_high = (old_div & DIV_APU_BIT) != 0;
        let new_high = (new_div & DIV_APU_BIT) != 0;
        if old_high && !new_high {
            self.clock_frame_sequencer();
        }
    }

    pub(crate) fn step_tcycle_with_io(&mut self, io: &[u8; 0x80]) {
        self.step_tcycle(io);
    }
}
