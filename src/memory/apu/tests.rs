use super::super::test_utils::{make_test_bus, make_test_bus_with_model, tick_n};
use crate::audio::AnalogCalibrationProfile;
use crate::hardware::HardwareModel;

#[test]
fn nr52_power_toggle_clears_nr50_nr51_and_blocks_writes_while_off() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF26, 0x00); // power off APU
    assert_eq!(bus.read_byte(0xFF26) & 0x80, 0x00);
    assert_eq!(bus.read_byte(0xFF24), 0x00);
    assert_eq!(bus.read_byte(0xFF25), 0x00);

    bus.write_byte(0xFF24, 0x77); // ignored while powered off
    bus.write_byte(0xFF25, 0xF3); // ignored while powered off
    assert_eq!(bus.read_byte(0xFF24), 0x00);
    assert_eq!(bus.read_byte(0xFF25), 0x00);

    bus.write_byte(0xFF26, 0x80); // power on APU
    assert_eq!(bus.read_byte(0xFF26) & 0x80, 0x80);

    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0xF3);
    assert_eq!(bus.read_byte(0xFF24), 0x77);
    assert_eq!(bus.read_byte(0xFF25), 0xF3);

    bus.write_byte(0xFF26, 0x00); // power off clears control regs again
    assert_eq!(bus.read_byte(0xFF24), 0x00);
    assert_eq!(bus.read_byte(0xFF25), 0x00);
}

#[test]
fn nr52_ignores_writes_to_channel_status_bits() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00); // reset to known state
    bus.write_byte(0xFF26, 0x80); // power on
    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x00);

    bus.write_byte(0xFF26, 0x8F); // low nibble should be ignored
    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x00);
}

#[test]
fn apu_frame_sequencer_advances_on_divider_bit12_falling_edges() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00); // reset frame sequencer state
    bus.write_byte(0xFF26, 0x80);

    assert_eq!(bus.apu_frame_sequencer_ticks(), 0);
    assert_eq!(bus.apu_frame_sequencer_step(), 0);

    for _ in 0..8 {
        tick_n(&mut bus, 4096); // raise DIV bit12
        bus.write_byte(0xFF04, 0x00); // clear DIV => falling edge clocks frame sequencer
    }

    assert_eq!(bus.apu_frame_sequencer_ticks(), 8);
    assert_eq!(bus.apu_frame_sequencer_step(), 0);
    assert_eq!(bus.apu_length_tick_count(), 4);
    assert_eq!(bus.apu_sweep_tick_count(), 2);
    assert_eq!(bus.apu_envelope_tick_count(), 1);
}

#[test]
fn apu_frame_sequencer_stops_when_apu_is_powered_off() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00); // power off and reset counters
    assert_eq!(bus.apu_frame_sequencer_ticks(), 0);

    tick_n(&mut bus, 4096);
    bus.write_byte(0xFF04, 0x00); // divider falling edge while APU disabled

    assert_eq!(bus.apu_frame_sequencer_ticks(), 0);
    assert_eq!(bus.apu_frame_sequencer_step(), 0);
}

#[test]
fn apu_square_channels_generate_dynamic_mixed_output() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77); // max left/right output volume
    bus.write_byte(0xFF25, 0x33); // CH1+CH2 routed to both sides

    // CH1
    bus.write_byte(0xFF11, 0x80); // duty 10, length 0
    bus.write_byte(0xFF12, 0xF0); // DAC on, volume 15
    bus.write_byte(0xFF13, 0xFC); // high frequency => short waveform period
    bus.write_byte(0xFF14, 0x87); // trigger

    // CH2
    bus.write_byte(0xFF16, 0xC0); // duty 11
    bus.write_byte(0xFF17, 0xE0); // DAC on, volume 14
    bus.write_byte(0xFF18, 0xF0);
    bus.write_byte(0xFF19, 0x87); // trigger

    let mut min_sample = f32::INFINITY;
    let mut max_sample = f32::NEG_INFINITY;
    for _ in 0..128 {
        bus.tick(1);
        let sample = bus.apu_last_mixed_sample();
        min_sample = min_sample.min(sample);
        max_sample = max_sample.max(sample);
    }

    assert_ne!(bus.read_byte(0xFF26) & 0x03, 0x00);
    assert!(max_sample - min_sample > 0.05);
}

#[test]
fn apu_tcycle_stream_respects_stereo_routing_masks() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77); // max left/right output volume
    bus.write_byte(0xFF25, 0x01); // CH1 routed to right only
    bus.set_audio_tcycle_stream_enabled(true);

    bus.write_byte(0xFF11, 0x80);
    bus.write_byte(0xFF12, 0xF0);
    bus.write_byte(0xFF13, 0xFC);
    bus.write_byte(0xFF14, 0x87); // trigger CH1

    tick_n(&mut bus, 512);
    let (last_left, last_right) = bus.apu_last_mixed_sample_stereo();
    let samples = bus.drain_audio_tcycle_samples();
    assert!(!samples.is_empty());
    assert_eq!(samples.len() % 2, 0);

    let mut left_peak = 0.0f32;
    let mut right_peak = 0.0f32;
    for frame in samples.chunks_exact(2) {
        left_peak = left_peak.max(frame[0].abs());
        right_peak = right_peak.max(frame[1].abs());
    }

    assert!(right_peak > 0.05);
    assert!(left_peak < 0.001);
    assert!(last_right.abs() >= last_left.abs());
}

#[test]
fn apu_length_clock_disables_square_channel_when_counter_expires() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF16, 0x3F); // length=1
    bus.write_byte(0xFF17, 0xF0); // DAC on
    bus.write_byte(0xFF19, 0xC0); // length enable + trigger
    assert_ne!(bus.read_byte(0xFF26) & 0x02, 0x00);

    tick_n(&mut bus, 4096);
    bus.write_byte(0xFF04, 0x00); // force first frame-sequencer length clock (step 0)

    assert_eq!(bus.read_byte(0xFF26) & 0x02, 0x00);
}

#[test]
fn apu_enabling_length_on_non_length_step_clocks_length_immediately() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF16, 0x3F); // CH2 length=1
    bus.write_byte(0xFF17, 0xF0); // DAC on
    bus.write_byte(0xFF19, 0x80); // trigger with length disabled
    assert_ne!(bus.read_byte(0xFF26) & 0x02, 0x00);

    tick_n(&mut bus, 4096);
    bus.write_byte(0xFF04, 0x00); // frame sequencer step advances to 1 (non-length step next)
    assert_eq!(bus.apu_frame_sequencer_step(), 1);

    bus.write_byte(0xFF19, 0x40); // enable length without trigger
    assert_eq!(bus.read_byte(0xFF26) & 0x02, 0x00);
}

#[test]
fn apu_envelope_clock_updates_square_volume() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF16, 0x80);
    bus.write_byte(0xFF17, 0x19); // start vol=1, increase, period=1
    bus.write_byte(0xFF19, 0x80); // trigger
    assert_eq!(bus.apu_square2_envelope_volume(), 1);

    for _ in 0..8 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }

    assert_eq!(bus.apu_square2_envelope_volume(), 2);
}

#[test]
fn apu_sweep_clock_updates_square1_frequency() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF10, 0x11); // period=1, increase, shift=1
    bus.write_byte(0xFF11, 0x80);
    bus.write_byte(0xFF12, 0xF0);
    bus.write_byte(0xFF13, 0xE8); // freq = 1000
    bus.write_byte(0xFF14, 0x83); // trigger
    assert_eq!(bus.apu_square1_frequency(), 1000);

    for _ in 0..3 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }

    assert_eq!(bus.apu_square1_frequency(), 1500);
}

#[test]
fn apu_sweep_trigger_overflow_disables_square1() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF10, 0x11); // period=1, increase, shift=1
    bus.write_byte(0xFF11, 0x80);
    bus.write_byte(0xFF12, 0xF0); // DAC on
    bus.write_byte(0xFF13, 0xF8); // freq low (2040)
    bus.write_byte(0xFF14, 0x87); // trigger with high bits=0b111

    assert!(!bus.apu_square1_enabled());
    assert_eq!(bus.read_byte(0xFF26) & 0x01, 0x00);
}

#[test]
fn apu_sweep_negate_clear_after_subtraction_disables_square1() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF10, 0x19); // period=1, negate, shift=1
    bus.write_byte(0xFF11, 0x80);
    bus.write_byte(0xFF12, 0xF0);
    bus.write_byte(0xFF13, 0xE8); // freq=1000
    bus.write_byte(0xFF14, 0x83); // trigger
    assert!(bus.apu_square1_enabled());

    for _ in 0..3 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }
    assert!(bus.apu_square1_enabled());

    bus.write_byte(0xFF10, 0x11); // clear negate after subtraction sweep
    assert!(!bus.apu_square1_enabled());
    assert_eq!(bus.read_byte(0xFF26) & 0x01, 0x00);
}

#[test]
fn apu_trigger_with_zero_length_on_non_length_step_loads_square2_to_63() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    tick_n(&mut bus, 4096);
    bus.write_byte(0xFF04, 0x00); // step 0 consumed, next step is 1 (non-length)
    assert_eq!(bus.apu_frame_sequencer_step(), 1);

    bus.write_byte(0xFF17, 0xF0); // DAC on
    bus.write_byte(0xFF19, 0xC0); // trigger + length enable with length counter initially zero

    assert!(bus.apu_square2_enabled());
    assert_eq!(bus.apu_square2_length_counter(), 63);
}

#[test]
fn apu_trigger_on_envelope_step_reloads_envelope_timer_plus_one() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    for _ in 0..7 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }
    assert_eq!(bus.apu_frame_sequencer_step(), 7);

    bus.write_byte(0xFF17, 0x19); // start vol=1, increase, period=1
    bus.write_byte(0xFF19, 0x80); // trigger
    assert_eq!(bus.apu_square2_envelope_timer(), 2);
}

#[test]
fn apu_clearing_dac_disables_square2_immediately() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF16, 0x80);
    bus.write_byte(0xFF17, 0xF0);
    bus.write_byte(0xFF19, 0x80);
    assert!(bus.apu_square2_enabled());
    assert_ne!(bus.read_byte(0xFF26) & 0x02, 0x00);

    bus.write_byte(0xFF17, 0x00); // DAC off
    assert!(!bus.apu_square2_enabled());
    assert_eq!(bus.read_byte(0xFF26) & 0x02, 0x00);
}

#[test]
fn apu_envelope_period_zero_keeps_volume_constant() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF16, 0x80);
    bus.write_byte(0xFF17, 0xF0); // start vol=15, period=0
    bus.write_byte(0xFF19, 0x80);
    let initial = bus.apu_square2_envelope_volume();

    for _ in 0..24 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }

    assert_eq!(bus.apu_square2_envelope_volume(), initial);
}

#[test]
fn apu_wave_retrigger_keeps_previous_sample_buffer_until_next_fetch() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF30, 0x12);
    bus.write_byte(0xFF31, 0xE4);
    bus.write_byte(0xFF1A, 0x80); // DAC on
    bus.write_byte(0xFF1C, 0x20); // output level 100%
    bus.write_byte(0xFF1D, 0xFF); // high frequency
    bus.write_byte(0xFF1E, 0x87); // trigger

    for _ in 0..32 {
        bus.tick(1);
        if bus.apu_wave_position() == 2 {
            break;
        }
    }
    assert_eq!(bus.apu_wave_position(), 2);
    let buffer_before_retrigger = bus.apu_wave_sample_buffer();
    assert_eq!(buffer_before_retrigger, 0xE4);

    bus.write_byte(0xFF1E, 0x87); // retrigger while channel is active
    assert_eq!(bus.apu_wave_position(), 0);
    assert_eq!(bus.apu_wave_sample_buffer(), buffer_before_retrigger);

    for _ in 0..8 {
        bus.tick(1);
        if bus.apu_wave_position() == 1 {
            break;
        }
    }
    assert_eq!(bus.apu_wave_position(), 1);
    assert_eq!(bus.apu_wave_sample_buffer(), 0x12);
}

#[test]
fn apu_noise_width_mode_mirrors_lfsr_bit6_to_bit14() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF21, 0xF0); // DAC on
    bus.write_byte(0xFF22, 0x08); // width mode 7-bit, shift=0, divisor=0
    bus.write_byte(0xFF23, 0x80); // trigger

    for _ in 0..256 {
        bus.tick(1);
        let lfsr = bus.apu_noise_lfsr();
        assert_eq!((lfsr >> 6) & 0x1, (lfsr >> 14) & 0x1);
    }
}

#[test]
fn apu_noise_shift14_and_shift15_stop_lfsr_clocking() {
    for polynomial in [0xE0, 0xF0] {
        let mut bus = make_test_bus();
        bus.write_byte(0xFF26, 0x00);
        bus.write_byte(0xFF26, 0x80);
        bus.write_byte(0xFF21, 0xF0); // DAC on
        bus.write_byte(0xFF22, polynomial);
        bus.write_byte(0xFF23, 0x80); // trigger

        let initial_lfsr = bus.apu_noise_lfsr();
        tick_n(&mut bus, 80_000);
        assert_eq!(bus.apu_noise_lfsr(), initial_lfsr);
    }
}

#[test]
fn apu_wave_and_noise_channels_set_status_bits_on_trigger() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0xCC); // CH3+CH4 routed to both sides

    // CH3 (wave)
    bus.write_byte(0xFF30, 0xF0);
    bus.write_byte(0xFF31, 0x00);
    bus.write_byte(0xFF1A, 0x80); // DAC on
    bus.write_byte(0xFF1C, 0x20); // output level 100%
    bus.write_byte(0xFF1D, 0x40);
    bus.write_byte(0xFF1E, 0x80); // trigger

    // CH4 (noise)
    bus.write_byte(0xFF20, 0x3F); // length=1
    bus.write_byte(0xFF21, 0xF0); // DAC on, volume 15
    bus.write_byte(0xFF22, 0x00); // shortest divisor
    bus.write_byte(0xFF23, 0x80); // trigger

    let mut min_sample = f32::INFINITY;
    let mut max_sample = f32::NEG_INFINITY;
    for _ in 0..256 {
        bus.tick(1);
        let sample = bus.apu_last_mixed_sample();
        min_sample = min_sample.min(sample);
        max_sample = max_sample.max(sample);
    }

    assert_ne!(bus.read_byte(0xFF26) & 0x0C, 0x00);
    assert!(max_sample - min_sample > 0.05);
}

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

#[test]
fn apu_boot_nr52_channel_status_bit_is_stable_after_first_tick() {
    let mut bus = make_test_bus_with_model(HardwareModel::Dmg);
    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x01);

    bus.tick(1);

    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x01);
}
