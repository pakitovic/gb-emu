use super::common::{make_test_bus, tick_n};
use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

#[test]
fn apu_interface_natural_divider_falling_edge_clocks_frame_sequencer_once() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    assert_eq!(bus.apu_test_state().frame_sequencer_ticks, 0);

    tick_n(&mut bus, 4096);
    assert_eq!(bus.apu_test_state().frame_sequencer_ticks, 0);

    tick_n(&mut bus, 4096);
    assert_eq!(bus.apu_test_state().frame_sequencer_ticks, 1);
}

#[test]
fn apu_interface_stream_disable_clears_pending_samples_and_stops_capture() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0x01); // CH1 right
    bus.set_audio_tcycle_stream_enabled(true);

    bus.write_byte(0xFF11, 0x80);
    bus.write_byte(0xFF12, 0xF0);
    bus.write_byte(0xFF13, 0xFC);
    bus.write_byte(0xFF14, 0x87);
    tick_n(&mut bus, 128);
    assert!(!bus.drain_audio_tcycle_samples().is_empty());

    bus.set_audio_tcycle_stream_enabled(false);
    assert!(bus.drain_audio_tcycle_samples().is_empty());

    tick_n(&mut bus, 128);
    assert!(bus.drain_audio_tcycle_samples().is_empty());
}

#[test]
fn apu_interface_mixed_power_stream_and_calibration_activity_preserves_invariants() {
    let mut bus = make_test_bus();
    let mut calibration = AnalogCalibrationProfile::for_model(HardwareModel::Dmg);
    calibration.soft_clip_drive = 2.5;
    calibration.crossfeed = 0.1;
    calibration.channel_gain = [1.2, 0.9, 1.1, 1.0];
    bus.set_apu_analog_calibration(calibration);

    for round in 0..8 {
        bus.write_byte(0xFF26, 0x00);
        bus.write_byte(0xFF26, 0x80);
        bus.write_byte(0xFF24, 0x77);
        bus.write_byte(0xFF25, 0x11);
        bus.set_audio_tcycle_stream_enabled((round & 1) == 0);

        bus.write_byte(0xFF11, 0x80);
        bus.write_byte(0xFF12, 0xF0);
        bus.write_byte(0xFF13, 0xFC);
        bus.write_byte(0xFF14, 0x87);

        tick_n(&mut bus, 1_024);
        let samples = bus.drain_audio_tcycle_samples();
        assert_eq!(samples.len() % 2, 0);
        assert!(samples.iter().all(|sample| sample.is_finite()));

        if (round & 1) == 0 {
            assert!(!samples.is_empty());
        } else {
            assert!(samples.is_empty());
        }
    }
}
