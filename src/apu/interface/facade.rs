use super::*;
use crate::apu::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

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
}
