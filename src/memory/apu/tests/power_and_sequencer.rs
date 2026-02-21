use super::{HardwareModel, make_test_bus, make_test_bus_with_model, tick_n};

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
fn apu_boot_nr52_channel_status_bit_is_stable_after_first_tick() {
    let mut bus = make_test_bus_with_model(HardwareModel::Dmg);
    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x01);

    bus.tick(1);

    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x01);
}
