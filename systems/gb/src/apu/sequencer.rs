use super::*;

#[derive(Clone, Copy)]
pub(in crate::apu) struct FrameSequencerClocks {
    pub(in crate::apu) clock_length: bool,
    pub(in crate::apu) clock_sweep: bool,
    pub(in crate::apu) clock_envelope: bool,
}

impl FrameSequencerState {
    pub(in crate::apu) fn advance(&mut self) -> FrameSequencerClocks {
        self.ticks = self.ticks.saturating_add(1);
        let clocks = FrameSequencerClocks {
            clock_length: self.length_clocks_on_next_step(),
            clock_sweep: self.step == 2 || self.step == 6,
            clock_envelope: self.envelope_clocks_on_next_step(),
        };
        if clocks.clock_length {
            self.length_tick_count = self.length_tick_count.saturating_add(1);
        }
        if clocks.clock_sweep {
            self.sweep_tick_count = self.sweep_tick_count.saturating_add(1);
        }
        if clocks.clock_envelope {
            self.envelope_tick_count = self.envelope_tick_count.saturating_add(1);
        }
        self.step = (self.step + 1) & 0x07;
        clocks
    }

    pub(in crate::apu) fn length_clocks_on_next_step(&self) -> bool {
        (self.step & 0x01) == 0
    }

    pub(in crate::apu) fn envelope_clocks_on_next_step(&self) -> bool {
        self.step == 7
    }
}

impl ApuState {
    pub(super) fn clock_frame_sequencer(&mut self) {
        if !self.enabled {
            return;
        }

        let clocks = self.timing.advance();
        if clocks.clock_length {
            self.square1.clock_length();
            self.square2.clock_length();
            self.wave.clock_length();
            self.noise.clock_length();
        }
        if clocks.clock_sweep {
            self.square1.clock_sweep();
        }
        if clocks.clock_envelope {
            self.square1.clock_envelope();
            self.square2.clock_envelope();
            self.noise.clock_envelope();
        }
        self.refresh_channel_on_mask();
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
