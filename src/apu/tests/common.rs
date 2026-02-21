use super::super::{ApuState, test_debug::ApuTestDebugState};
use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

pub(super) struct ApuHarness {
    apu: ApuState,
    io: [u8; 0x80],
    div_counter: u16,
}

impl ApuHarness {
    pub(super) fn with_model(model: HardwareModel) -> Self {
        let io = [0; 0x80];
        let apu = ApuState::from_boot_state(&io, model);
        Self {
            apu,
            io,
            div_counter: 0,
        }
    }

    pub(super) fn write_byte(&mut self, addr: u16, value: u8) {
        if addr == 0xFF04 {
            let old_div = self.div_counter;
            self.div_counter = 0;
            self.apu
                .step_frame_sequencer_from_divider(old_div, self.div_counter);
            return;
        }
        self.apu.write_io_register(&mut self.io, addr, value);
    }

    pub(super) fn read_byte(&self, addr: u16) -> u8 {
        if let Some(value) = self.apu.read_io_register(addr) {
            return value;
        }
        self.io[(addr - 0xFF00) as usize]
    }

    pub(super) fn tick(&mut self, ticks: usize) {
        for _ in 0..ticks {
            let old_div = self.div_counter;
            self.div_counter = self.div_counter.wrapping_add(1);
            self.apu
                .step_frame_sequencer_from_divider(old_div, self.div_counter);
            self.apu.step_tcycle_with_io(&self.io);
        }
    }

    pub(super) fn set_audio_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.apu.set_tcycle_stream_enabled(enabled);
    }

    pub(super) fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        self.apu.drain_tcycle_samples()
    }

    pub(super) fn set_apu_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.apu.set_analog_calibration(calibration);
    }

    pub(super) fn apu_test_state(&self) -> ApuTestDebugState {
        self.apu.test_debug_state()
    }
}

pub(super) fn make_test_bus() -> ApuHarness {
    make_test_bus_with_model(HardwareModel::default())
}

pub(super) fn make_test_bus_with_model(model: HardwareModel) -> ApuHarness {
    ApuHarness::with_model(model)
}

pub(super) fn tick_n(bus: &mut ApuHarness, ticks: usize) {
    bus.tick(ticks);
}
