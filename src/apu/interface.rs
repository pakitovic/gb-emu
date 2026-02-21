use super::*;

impl ApuState {
    pub(crate) fn from_boot_state(io: &[u8; 0x80], model: HardwareModel) -> Self {
        Self::from_boot_registers(io, model)
    }

    pub(crate) fn set_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.analog_profile = calibration.normalized();
        self.reset_analog_filter_state();
    }

    pub(crate) fn read_io_register(&self, addr: u16) -> Option<u8> {
        self.read_io_register_mmio(addr)
    }

    pub(crate) fn write_io_register(&mut self, io: &mut [u8; 0x80], addr: u16, value: u8) {
        self.write_io_register_mmio(io, addr, value);
    }

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
