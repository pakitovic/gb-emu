use super::super::Bus;
use super::*;

impl Bus {
    pub(in crate::memory) fn sync_apu_boot_state(&mut self, model: HardwareModel) {
        self.apu = ApuState::from_boot_registers(&self.io, model);
    }

    pub fn set_apu_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.apu.analog_profile = calibration.normalized();
        self.apu.reset_analog_filter_state();
    }

    pub(in crate::memory) fn read_nr52(&self) -> u8 {
        ((self.apu.enabled as u8) << 7) | (self.apu.channel_on_mask & 0x0F)
    }

    pub(in crate::memory) fn write_nr50(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR50_INDEX] = value;
    }

    pub(in crate::memory) fn write_nr51(&mut self, value: u8) {
        if !self.apu.enabled {
            return;
        }
        self.io[NR51_INDEX] = value;
    }

    pub(in crate::memory) fn write_nr52(&mut self, value: u8) {
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

    pub(in crate::memory) fn step_apu_frame_sequencer_from_divider(
        &mut self,
        old_div: u16,
        new_div: u16,
    ) {
        if !self.apu.enabled {
            return;
        }

        let old_high = (old_div & DIV_APU_BIT) != 0;
        let new_high = (new_div & DIV_APU_BIT) != 0;
        if old_high && !new_high {
            self.apu.clock_frame_sequencer();
        }
    }

    pub(in crate::memory) fn step_apu_tcycle(&mut self) {
        self.apu.step_tcycle(&self.io);
    }

    pub fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        if self.apu.pending_tcycle_samples.is_empty() {
            return Vec::new();
        }
        self.apu.pending_tcycle_samples.drain(..).collect()
    }

    pub fn set_audio_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.apu.capture_tcycle_stream = enabled;
        if !enabled {
            self.apu.pending_tcycle_samples.clear();
        }
    }
}
