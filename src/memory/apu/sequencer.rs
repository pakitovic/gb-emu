use super::*;

impl ApuState {
    pub(super) fn clock_frame_sequencer(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_sequencer_ticks = self.frame_sequencer_ticks.saturating_add(1);
        let step = self.frame_sequencer_step;
        if (step & 0x01) == 0 {
            self.length_tick_count = self.length_tick_count.saturating_add(1);
            self.square1.clock_length();
            self.square2.clock_length();
            self.wave.clock_length();
            self.noise.clock_length();
        }
        if step == 2 || step == 6 {
            self.sweep_tick_count = self.sweep_tick_count.saturating_add(1);
            self.square1.clock_sweep();
        }
        if step == 7 {
            self.envelope_tick_count = self.envelope_tick_count.saturating_add(1);
            self.square1.clock_envelope();
            self.square2.clock_envelope();
            self.noise.clock_envelope();
        }
        self.frame_sequencer_step = (self.frame_sequencer_step + 1) & 0x07;
        self.refresh_channel_on_mask();
    }

    pub(super) fn length_clocks_on_next_frame_step(&self) -> bool {
        (self.frame_sequencer_step & 0x01) == 0
    }

    pub(super) fn envelope_clocks_on_next_frame_step(&self) -> bool {
        self.frame_sequencer_step == 7
    }

    pub(super) fn refresh_channel_on_mask(&mut self) {
        let mut mask = 0u8;
        if self.square1.enabled {
            mask |= 1 << 0;
        }
        if self.square2.enabled {
            mask |= 1 << 1;
        }
        if self.wave.enabled {
            mask |= 1 << 2;
        }
        if self.noise.enabled {
            mask |= 1 << 3;
        }
        self.channel_on_mask = mask;
    }
}
