use super::super::Bus;
use crate::apu::ApuState;
use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

impl Bus {
    pub(in crate::memory) fn sync_apu_boot_state(&mut self, model: HardwareModel) {
        self.apu = ApuState::from_boot_state(&self.io, model);
    }

    pub fn set_apu_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.apu.set_analog_calibration(calibration);
    }

    pub(in crate::memory) fn read_apu_io_register(&self, addr: u16) -> Option<u8> {
        self.apu.read_io_register(addr)
    }

    pub(in crate::memory) fn write_apu_io_register(&mut self, addr: u16, value: u8) {
        self.apu.write_io_register(&mut self.io, addr, value);
    }

    pub(in crate::memory) fn step_apu_frame_sequencer_from_divider(
        &mut self,
        old_div: u16,
        new_div: u16,
    ) {
        self.apu.step_frame_sequencer_from_divider(old_div, new_div);
    }

    pub(in crate::memory) fn step_apu_tcycle(&mut self) {
        self.apu.step_tcycle_with_io(&self.io);
    }

    pub fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        self.apu.drain_tcycle_samples()
    }

    pub fn set_audio_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.apu.set_tcycle_stream_enabled(enabled);
    }
}
