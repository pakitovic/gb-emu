use super::{
    AnalogCalibrationProfile, HardwareModel, make_test_bus, make_test_bus_with_model, tick_n,
};

#[test]
fn apu_hpf_reduces_dc_offset_for_constant_wave_output() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77); // max volume
    bus.write_byte(0xFF25, 0x44); // CH3 to both sides

    // Constant max wave sample -> DC-like input before HPF.
    for addr in 0xFF30..=0xFF3F {
        bus.write_byte(addr, 0xFF);
    }

    bus.write_byte(0xFF1A, 0x80); // CH3 DAC on
    bus.write_byte(0xFF1C, 0x20); // 100% output level
    bus.write_byte(0xFF1D, 0x00); // frequency low
    bus.write_byte(0xFF1E, 0x80); // trigger

    let mut early_peak = 0.0f32;
    for _ in 0..128 {
        bus.tick(1);
        early_peak = early_peak.max(bus.apu_last_mixed_sample().abs());
    }

    tick_n(&mut bus, 80_000);
    let late_abs = bus.apu_last_mixed_sample().abs();

    assert!(early_peak > 0.1);
    assert!(
        late_abs < early_peak * 0.25,
        "expected HPF to reduce DC offset over time (early={early_peak}, late={late_abs})"
    );
}

#[test]
fn apu_hpf_state_keeps_decaying_while_tcycle_stream_capture_is_disabled() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77); // max volume
    bus.write_byte(0xFF25, 0x44); // CH3 to both sides

    for addr in 0xFF30..=0xFF3F {
        bus.write_byte(addr, 0xFF);
    }

    bus.write_byte(0xFF1A, 0x80); // CH3 DAC on
    bus.write_byte(0xFF1C, 0x20); // 100% output level
    bus.write_byte(0xFF1D, 0x00);
    bus.write_byte(0xFF1E, 0x80); // trigger

    bus.set_audio_tcycle_stream_enabled(true);
    tick_n(&mut bus, 128);
    let enabled_peak = bus
        .drain_audio_tcycle_samples()
        .into_iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));

    bus.set_audio_tcycle_stream_enabled(false);
    tick_n(&mut bus, 80_000);
    assert!(bus.drain_audio_tcycle_samples().is_empty());

    bus.set_audio_tcycle_stream_enabled(true);
    tick_n(&mut bus, 1);
    let resumed_sample_abs = bus
        .drain_audio_tcycle_samples()
        .into_iter()
        .next()
        .unwrap_or_default()
        .abs();

    assert!(enabled_peak > 0.1);
    assert!(
        resumed_sample_abs < enabled_peak * 0.25,
        "expected HPF to decay while capture is disabled (enabled_peak={enabled_peak}, resumed={resumed_sample_abs})"
    );
}

#[test]
fn apu_analog_profile_is_model_specific() {
    let dmg = make_test_bus_with_model(HardwareModel::Dmg);
    let mgb = make_test_bus_with_model(HardwareModel::Mgb);
    let sgb = make_test_bus_with_model(HardwareModel::Sgb);

    assert!((dmg.apu_analog_hpf_coeff() - mgb.apu_analog_hpf_coeff()).abs() > f32::EPSILON);
    assert!(
        (dmg.apu_analog_low_pass_alpha() - mgb.apu_analog_low_pass_alpha()).abs() > f32::EPSILON
    );
    assert!(
        (mgb.apu_analog_soft_clip_drive() - sgb.apu_analog_soft_clip_drive()).abs() > f32::EPSILON
    );
}

#[test]
fn apu_custom_calibration_profile_can_mute_channel_output() {
    let mut bus = make_test_bus_with_model(HardwareModel::Dmg);
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    let mut calibration = AnalogCalibrationProfile::for_model(HardwareModel::Dmg);
    calibration.channel_gain = [0.0; 4];
    calibration.routing_left = [0.0; 4];
    calibration.routing_right = [0.0; 4];
    bus.set_apu_analog_calibration(calibration);

    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0x11); // CH1 to both sides
    bus.write_byte(0xFF11, 0x80);
    bus.write_byte(0xFF12, 0xF0);
    bus.write_byte(0xFF13, 0xFC);
    bus.write_byte(0xFF14, 0x87); // trigger
    tick_n(&mut bus, 256);

    assert!(
        bus.apu_last_mixed_sample().abs() < 0.000_01,
        "expected near-silence with zeroed calibration gain"
    );
}

#[test]
fn apu_custom_calibration_crossfeed_can_inject_right_into_left() {
    let mut bus = make_test_bus_with_model(HardwareModel::Dmg);
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0x01); // CH1 to right only

    let mut calibration = AnalogCalibrationProfile::for_model(HardwareModel::Dmg);
    calibration.crossfeed = 0.2;
    bus.set_apu_analog_calibration(calibration);

    bus.write_byte(0xFF11, 0x80);
    bus.write_byte(0xFF12, 0xF0);
    bus.write_byte(0xFF13, 0xFC);
    bus.write_byte(0xFF14, 0x87); // trigger
    tick_n(&mut bus, 512);

    let (left, right) = bus.apu_last_mixed_sample_stereo();
    assert!(right.abs() > 0.01);
    assert!(
        left.abs() > 0.001,
        "expected crossfeed to produce non-zero left output"
    );
}

#[test]
fn apu_low_pass_stage_softens_initial_attack() {
    let mut bus = make_test_bus_with_model(HardwareModel::Dmg);
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0x44); // CH3 to both sides

    for addr in 0xFF30..=0xFF3F {
        bus.write_byte(addr, 0xFF);
    }

    bus.write_byte(0xFF1A, 0x80); // CH3 DAC on
    bus.write_byte(0xFF1C, 0x20); // 100% output level
    bus.write_byte(0xFF1D, 0x00);
    bus.write_byte(0xFF1E, 0x80); // trigger

    bus.tick(1);
    let first_abs = bus.apu_last_mixed_sample().abs();

    let mut later_peak = 0.0f32;
    for _ in 0..512 {
        bus.tick(1);
        later_peak = later_peak.max(bus.apu_last_mixed_sample().abs());
    }

    assert!(first_abs > 0.0);
    assert!(
        later_peak > first_abs * 1.2,
        "expected LPF attack ramp (first={first_abs}, later_peak={later_peak})"
    );
}
