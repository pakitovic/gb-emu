use super::super::ApuState;
use crate::apu::MAX_PENDING_AUDIO_TCYCLE_FRAMES;
use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

#[derive(Clone, Copy)]
pub(super) struct ApuTestDebugState {
    pub(super) frame_sequencer_step: u8,
    pub(super) frame_sequencer_ticks: u64,
    pub(super) length_tick_count: u64,
    pub(super) sweep_tick_count: u64,
    pub(super) envelope_tick_count: u64,
    pub(super) last_mixed_sample: f32,
    pub(super) last_mixed_sample_left: f32,
    pub(super) last_mixed_sample_right: f32,
    pub(super) square2_envelope_volume: u8,
    pub(super) square2_envelope_timer: u8,
    pub(super) square2_length_counter: u8,
    pub(super) square1_frequency: u16,
    pub(super) square1_enabled: bool,
    pub(super) square2_enabled: bool,
    pub(super) wave_position: u8,
    pub(super) wave_sample_buffer: u8,
    pub(super) noise_lfsr: u16,
    pub(super) analog_hpf_coeff: f32,
    pub(super) analog_low_pass_alpha: f32,
    pub(super) analog_soft_clip_drive: f32,
}

pub(super) struct ApuHarness {
    apu: ApuState,
    io: [u8; 0x80],
    div_counter: u16,
}

impl ApuHarness {
    pub(super) fn with_model(model: HardwareModel) -> Self {
        let io = [0; 0x80];
        let apu = ApuState::from_boot_state(&io, model);
        let harness = Self {
            apu,
            io,
            div_counter: 0,
        };
        harness.assert_apu_invariants();
        harness
    }

    pub(super) fn write_byte(&mut self, addr: u16, value: u8) {
        if addr == 0xFF04 {
            let old_div = self.div_counter;
            self.div_counter = 0;
            self.apu
                .step_frame_sequencer_from_divider(old_div, self.div_counter);
            self.assert_apu_invariants();
            return;
        }
        self.apu.write_io_register(&mut self.io, addr, value);
        self.assert_apu_invariants();
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
            self.assert_apu_invariants();
        }
    }

    pub(super) fn set_audio_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.apu.set_tcycle_stream_enabled(enabled);
        self.assert_apu_invariants();
    }

    pub(super) fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        let samples = self.apu.drain_tcycle_samples();
        self.assert_apu_invariants();
        samples
    }

    pub(super) fn set_apu_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.apu.set_analog_calibration(calibration);
        self.assert_apu_invariants();
    }

    pub(super) fn apu_test_state(&self) -> ApuTestDebugState {
        ApuTestDebugState {
            frame_sequencer_step: self.apu.timing.step,
            frame_sequencer_ticks: self.apu.timing.ticks,
            length_tick_count: self.apu.timing.length_tick_count,
            sweep_tick_count: self.apu.timing.sweep_tick_count,
            envelope_tick_count: self.apu.timing.envelope_tick_count,
            last_mixed_sample: self.apu.analog.last_mixed_sample,
            last_mixed_sample_left: self.apu.analog.last_mixed_sample_left,
            last_mixed_sample_right: self.apu.analog.last_mixed_sample_right,
            square2_envelope_volume: self.apu.square2.envelope.volume,
            square2_envelope_timer: self.apu.square2.envelope.timer,
            square2_length_counter: self.apu.square2.length_counter,
            square1_frequency: self.apu.square1.frequency,
            square1_enabled: self.apu.square1.enabled,
            square2_enabled: self.apu.square2.enabled,
            wave_position: self.apu.wave.wave_position,
            wave_sample_buffer: self.apu.wave.sample_buffer,
            noise_lfsr: self.apu.noise.lfsr,
            analog_hpf_coeff: self.apu.analog_profile.hpf_coeff,
            analog_low_pass_alpha: self.apu.analog_profile.low_pass_alpha,
            analog_soft_clip_drive: self.apu.analog_profile.soft_clip_drive,
        }
    }

    fn assert_apu_invariants(&self) {
        let expected_mask = (self.apu.square1.enabled as u8)
            | ((self.apu.square2.enabled as u8) << 1)
            | ((self.apu.wave.enabled as u8) << 2)
            | ((self.apu.noise.enabled as u8) << 3);
        assert_eq!(self.apu.channel_on_mask & 0x0F, expected_mask);
        assert!(self.apu.timing.step <= 7);

        let pending_scalars = self.apu.stream.pending_tcycle_samples.len();
        assert_eq!(
            pending_scalars % 2,
            0,
            "stereo interleaved queue must be aligned"
        );
        assert!(
            pending_scalars <= MAX_PENDING_AUDIO_TCYCLE_FRAMES.saturating_mul(2),
            "pending tcycle sample queue exceeded cap"
        );
        if !self.apu.stream.capture_tcycle_stream {
            assert!(
                self.apu.stream.pending_tcycle_samples.is_empty(),
                "queue should be empty when tcycle capture is disabled"
            );
        }

        assert!(self.apu.analog.last_mixed_sample_left.is_finite());
        assert!(self.apu.analog.last_mixed_sample_right.is_finite());
        assert!(self.apu.analog.last_mixed_sample.is_finite());
        assert!(self.apu.analog.last_mixed_sample_left.abs() <= 1.0 + f32::EPSILON);
        assert!(self.apu.analog.last_mixed_sample_right.abs() <= 1.0 + f32::EPSILON);
        assert!(self.apu.analog.last_mixed_sample.abs() <= 1.0 + f32::EPSILON);
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
