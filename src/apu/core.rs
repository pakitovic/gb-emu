use super::*;

impl ApuState {
    pub(super) fn from_boot_registers(io: &[u8; 0x80], model: HardwareModel) -> Self {
        let nr52 = io[NR52_INDEX];
        let mut state = Self {
            analog_profile: AnalogCalibrationProfile::for_model(model).normalized(),
            enabled: (nr52 & 0x80) != 0,
            channel_on_mask: nr52 & 0x0F,
            ..Self::default()
        };
        if state.enabled {
            let mask = state.channel_on_mask;
            state.square1.enabled = (mask & 0x01) != 0;
            state.square2.enabled = (mask & 0x02) != 0;
            state.wave.enabled = (mask & 0x04) != 0;
            state.noise.enabled = (mask & 0x08) != 0;
        }
        state
    }

    pub(super) fn reset_after_power_toggle(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.channel_on_mask = 0;
        self.timing = FrameSequencerState::default();
        self.square1.reset(true);
        self.square2.reset(false);
        self.wave = WaveChannel::default();
        self.noise = NoiseChannel::default();
        self.analog.last_mixed_sample_left = 0.0;
        self.analog.last_mixed_sample_right = 0.0;
        self.analog.last_mixed_sample = 0.0;
        self.reset_analog_filter_state();
    }
}
