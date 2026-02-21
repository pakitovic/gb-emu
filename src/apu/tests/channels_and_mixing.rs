use super::common::{make_test_bus, tick_n};

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
        let sample = bus.apu_test_state().last_mixed_sample;
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
    let state = bus.apu_test_state();
    let (last_left, last_right) = (state.last_mixed_sample_left, state.last_mixed_sample_right);
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
    assert_eq!(bus.apu_test_state().frame_sequencer_step, 1);

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
    assert_eq!(bus.apu_test_state().square2_envelope_volume, 1);

    for _ in 0..8 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }

    assert_eq!(bus.apu_test_state().square2_envelope_volume, 2);
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
    assert_eq!(bus.apu_test_state().square1_frequency, 1000);

    for _ in 0..3 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }

    assert_eq!(bus.apu_test_state().square1_frequency, 1500);
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

    assert!(!bus.apu_test_state().square1_enabled);
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
    assert!(bus.apu_test_state().square1_enabled);

    for _ in 0..3 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }
    assert!(bus.apu_test_state().square1_enabled);

    bus.write_byte(0xFF10, 0x11); // clear negate after subtraction sweep
    assert!(!bus.apu_test_state().square1_enabled);
    assert_eq!(bus.read_byte(0xFF26) & 0x01, 0x00);
}

#[test]
fn apu_trigger_with_zero_length_on_non_length_step_loads_square2_to_63() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);
    tick_n(&mut bus, 4096);
    bus.write_byte(0xFF04, 0x00); // step 0 consumed, next step is 1 (non-length)
    assert_eq!(bus.apu_test_state().frame_sequencer_step, 1);

    bus.write_byte(0xFF17, 0xF0); // DAC on
    bus.write_byte(0xFF19, 0xC0); // trigger + length enable with length counter initially zero

    assert!(bus.apu_test_state().square2_enabled);
    assert_eq!(bus.apu_test_state().square2_length_counter, 63);
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
    assert_eq!(bus.apu_test_state().frame_sequencer_step, 7);

    bus.write_byte(0xFF17, 0x19); // start vol=1, increase, period=1
    bus.write_byte(0xFF19, 0x80); // trigger
    assert_eq!(bus.apu_test_state().square2_envelope_timer, 2);
}

#[test]
fn apu_clearing_dac_disables_square2_immediately() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF26, 0x80);

    bus.write_byte(0xFF16, 0x80);
    bus.write_byte(0xFF17, 0xF0);
    bus.write_byte(0xFF19, 0x80);
    assert!(bus.apu_test_state().square2_enabled);
    assert_ne!(bus.read_byte(0xFF26) & 0x02, 0x00);

    bus.write_byte(0xFF17, 0x00); // DAC off
    assert!(!bus.apu_test_state().square2_enabled);
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
    let initial = bus.apu_test_state().square2_envelope_volume;

    for _ in 0..24 {
        tick_n(&mut bus, 4096);
        bus.write_byte(0xFF04, 0x00);
    }

    assert_eq!(bus.apu_test_state().square2_envelope_volume, initial);
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
        if bus.apu_test_state().wave_position == 2 {
            break;
        }
    }
    assert_eq!(bus.apu_test_state().wave_position, 2);
    let buffer_before_retrigger = bus.apu_test_state().wave_sample_buffer;
    assert_eq!(buffer_before_retrigger, 0xE4);

    bus.write_byte(0xFF1E, 0x87); // retrigger while channel is active
    assert_eq!(bus.apu_test_state().wave_position, 0);
    assert_eq!(
        bus.apu_test_state().wave_sample_buffer,
        buffer_before_retrigger
    );

    for _ in 0..8 {
        bus.tick(1);
        if bus.apu_test_state().wave_position == 1 {
            break;
        }
    }
    assert_eq!(bus.apu_test_state().wave_position, 1);
    assert_eq!(bus.apu_test_state().wave_sample_buffer, 0x12);
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
        let lfsr = bus.apu_test_state().noise_lfsr;
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

        let initial_lfsr = bus.apu_test_state().noise_lfsr;
        tick_n(&mut bus, 80_000);
        assert_eq!(bus.apu_test_state().noise_lfsr, initial_lfsr);
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
        let sample = bus.apu_test_state().last_mixed_sample;
        min_sample = min_sample.min(sample);
        max_sample = max_sample.max(sample);
    }

    assert_ne!(bus.read_byte(0xFF26) & 0x0C, 0x00);
    assert!(max_sample - min_sample > 0.05);
}
