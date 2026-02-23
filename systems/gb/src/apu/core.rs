use super::channels::{NoiseChannel, WaveChannel};
use super::registers::ApuRegisters;
use super::*;
use crate::apu::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

impl ApuState {
    pub(super) fn from_boot_registers(io: &[u8; 0x80], model: HardwareModel) -> Self {
        let nr52 = io[NR52_INDEX];
        let power_enabled = (nr52 & 0x80) != 0;
        let boot_channel_mask = nr52 & 0x0F;
        let mut state = Self {
            analog_profile: AnalogCalibrationProfile::for_model(model).normalized(),
            registers: ApuRegisters::from_io(io),
            ..Self::default()
        };
        state.reinitialize_power_state(power_enabled, boot_channel_mask);
        state
    }

    pub(super) fn reset_after_power_toggle(&mut self, enabled: bool) {
        self.reinitialize_power_state(enabled, 0);
    }

    fn reinitialize_power_state(&mut self, enabled: bool, channel_mask: u8) {
        self.enabled = enabled;
        self.apply_channel_status_mask(0);
        self.timing = FrameSequencerState::default();
        self.square1.reset(true);
        self.square2.reset(false);
        self.wave = WaveChannel::default();
        self.noise = NoiseChannel::default();
        self.analog.last_mixed_sample_left = 0.0;
        self.analog.last_mixed_sample_right = 0.0;
        self.analog.last_mixed_sample = 0.0;
        self.reset_analog_filter_state();

        if self.enabled {
            self.apply_channel_status_mask(channel_mask);
        }
    }

    fn apply_channel_status_mask(&mut self, mask: u8) {
        let masked = mask & 0x0F;
        self.channel_on_mask = masked;
        self.square1.enabled = (masked & 0x01) != 0;
        self.square2.enabled = (masked & 0x02) != 0;
        self.wave.enabled = (masked & 0x04) != 0;
        self.noise.enabled = (masked & 0x08) != 0;
    }
}
