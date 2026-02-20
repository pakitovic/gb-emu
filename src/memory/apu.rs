use super::Bus;

const NR10_INDEX: usize = 0x10;
const NR50_INDEX: usize = 0x24;
const NR51_INDEX: usize = 0x25;
const NR52_INDEX: usize = 0x26;
const DIV_APU_BIT: u16 = 1 << 12;

#[derive(Default)]
pub(super) struct ApuState {
    enabled: bool,
    channel_on_mask: u8,
    frame_sequencer_step: u8,
    frame_sequencer_ticks: u64,
    length_tick_count: u64,
    sweep_tick_count: u64,
    envelope_tick_count: u64,
}

impl ApuState {
    fn from_boot_nr52(nr52: u8) -> Self {
        Self {
            enabled: (nr52 & 0x80) != 0,
            channel_on_mask: nr52 & 0x0F,
            frame_sequencer_step: 0,
            frame_sequencer_ticks: 0,
            length_tick_count: 0,
            sweep_tick_count: 0,
            envelope_tick_count: 0,
        }
    }

    fn clock_frame_sequencer(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_sequencer_ticks = self.frame_sequencer_ticks.saturating_add(1);
        let step = self.frame_sequencer_step;
        if (step & 0x01) == 0 {
            self.length_tick_count = self.length_tick_count.saturating_add(1);
        }
        if step == 2 || step == 6 {
            self.sweep_tick_count = self.sweep_tick_count.saturating_add(1);
        }
        if step == 7 {
            self.envelope_tick_count = self.envelope_tick_count.saturating_add(1);
        }
        self.frame_sequencer_step = (self.frame_sequencer_step + 1) & 0x07;
    }

    fn reset_after_power_toggle(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.channel_on_mask = 0;
        self.frame_sequencer_step = 0;
        self.frame_sequencer_ticks = 0;
        self.length_tick_count = 0;
        self.sweep_tick_count = 0;
        self.envelope_tick_count = 0;
    }
}

impl Bus {
    pub(super) fn sync_apu_boot_state(&mut self) {
        self.apu = ApuState::from_boot_nr52(self.io[NR52_INDEX]);
    }

    pub(super) fn read_nr52(&self) -> u8 {
        ((self.apu.enabled as u8) << 7) | (self.apu.channel_on_mask & 0x0F)
    }

    pub(super) fn write_nr50(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR50_INDEX] = value;
    }

    pub(super) fn write_nr51(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR51_INDEX] = value;
    }

    pub(super) fn write_nr52(&mut self, value: u8) {
        let request_enabled = (value & 0x80) != 0;

        if self.apu.enabled && !request_enabled {
            self.clear_apu_registers();
            self.apu.reset_after_power_toggle(false);
            self.io[NR52_INDEX] = 0x00;
            return;
        }

        if !self.apu.enabled && request_enabled {
            self.clear_apu_registers();
            self.apu.reset_after_power_toggle(true);
            self.io[NR52_INDEX] = 0x80;
            return;
        }

        if self.apu.enabled {
            self.io[NR52_INDEX] = 0x80 | (self.apu.channel_on_mask & 0x0F);
        } else {
            self.io[NR52_INDEX] = 0x00;
        }
    }

    pub(super) fn step_apu_frame_sequencer_from_divider(&mut self, old_div: u16, new_div: u16) {
        if !self.apu.enabled {
            return;
        }

        let old_high = (old_div & DIV_APU_BIT) != 0;
        let new_high = (new_div & DIV_APU_BIT) != 0;
        if old_high && !new_high {
            self.apu.clock_frame_sequencer();
        }
    }

    fn clear_apu_registers(&mut self) {
        for index in NR10_INDEX..=NR51_INDEX {
            self.io[index] = 0x00;
        }
    }

    #[cfg(test)]
    pub(super) fn apu_frame_sequencer_step(&self) -> u8 {
        self.apu.frame_sequencer_step
    }

    #[cfg(test)]
    pub(super) fn apu_frame_sequencer_ticks(&self) -> u64 {
        self.apu.frame_sequencer_ticks
    }

    #[cfg(test)]
    pub(super) fn apu_length_tick_count(&self) -> u64 {
        self.apu.length_tick_count
    }

    #[cfg(test)]
    pub(super) fn apu_sweep_tick_count(&self) -> u64 {
        self.apu.sweep_tick_count
    }

    #[cfg(test)]
    pub(super) fn apu_envelope_tick_count(&self) -> u64 {
        self.apu.envelope_tick_count
    }
}
