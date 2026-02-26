use super::*;

#[test]
fn dmg_family_models_use_fixed_dmg_clock_ratio_policy() {
    for model in [
        HardwareModel::Dmg0,
        HardwareModel::Dmg,
        HardwareModel::Mgb,
        HardwareModel::Sgb,
        HardwareModel::Sgb2,
    ] {
        let bus = make_test_bus_with_model(model);
        assert_eq!(
            bus.cpu_tcycles_for_mcycles(1),
            DMG_CPU_T_CYCLES_PER_M_CYCLE,
            "unexpected CPU m-cycle ratio for model {model:?}"
        );
        assert_eq!(
            bus.cpu_tcycles_for_mcycles(2),
            DMG_CPU_T_CYCLES_PER_M_CYCLE * 2,
            "unexpected 2-mcycle conversion for model {model:?}"
        );
    }
}

#[test]
fn div_increments_every_256_tcycles_and_resets_on_write() {
    let mut bus = make_test_bus();
    assert_eq!(bus.read_byte(0xFF04), 0x00);

    bus.tick(255);
    assert_eq!(bus.read_byte(0xFF04), 0x00);

    bus.tick(1);
    assert_eq!(bus.read_byte(0xFF04), 0x01);

    bus.write_byte(0xFF04, 0x99);
    assert_eq!(bus.read_byte(0xFF04), 0x00);
}

#[test]
fn timer_overflow_reloads_tma_and_requests_interrupt() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF07, 0x05); // TAC: enable + 16 t-cycles period
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(16);
    assert_eq!(bus.read_byte(0xFF05), 0x00);
    assert_eq!(bus.interrupt_flags() & (1 << 2), 0);

    bus.tick(4);

    assert_eq!(bus.read_byte(0xFF05), 0x42);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn tick_chunking_preserves_div_tima_state_across_timer_edges_and_control_writes() {
    let mut chunked = make_test_bus();
    let mut single = make_test_bus();

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF07, 0x05); // TAC: enable + 16 t-cycles period
        bus.write_byte(0xFF06, 0x9C); // TMA
        bus.write_byte(0xFF05, 0xFA); // TIMA near overflow
    }
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[1, 3, 4, 8, 15, 2, 17, 9, 11] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF04, 0x00); // DIV reset edge-sensitive behavior
    }
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[5, 7, 13, 1, 16, 4, 3, 19] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF07, 0x07); // switch to another enabled timer source
    }
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[2, 6, 10, 14, 18, 22, 31] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }
}

#[test]
fn tick_chunking_preserves_ly_stat_and_tima_through_visible_mode_transitions() {
    let mut chunked = make_test_bus();
    let mut single = make_test_bus();

    for bus in [&mut chunked, &mut single] {
        bus.write_byte(0xFF43, 0x07); // SCX low bits affect visible mode3 timing
        bus.write_byte(0xFF41, 0x28); // enable mode2 + mode0 STAT sources
        bus.write_byte(0xFF07, 0x05); // timer enabled for concurrent DIV/TIMA activity
        bus.write_byte(0xFF06, 0x77);
        bus.write_byte(0xFF05, 0xF8);
    }

    wait_for_ly_mode(&mut chunked, 2, 2);
    wait_for_ly_mode(&mut single, 2, 2);
    assert_eq!(
        timing_contract_snapshot(&chunked),
        timing_contract_snapshot(&single)
    );

    for &chunk in &[1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 34, 21] {
        tick_chunk_and_compare_timing_state(&mut chunked, &mut single, chunk);
    }
}

#[test]
fn div_write_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // div bit3 becomes high
    bus.write_byte(0xFF04, 0x00); // reset DIV => falling edge => TIMA++

    assert_eq!(bus.read_byte(0xFF05), 0x01);
}

#[test]
fn tac_disable_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // selected input bit becomes high
    bus.write_byte(0xFF07, 0x00); // disable timer => falling edge => TIMA++

    assert_eq!(bus.read_byte(0xFF05), 0x01);
}

#[test]
fn tac_frequency_switch_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // bit3 high while bit5 is still low
    bus.write_byte(0xFF07, 0x06); // switch to bit5 source => falling edge => TIMA++

    assert_eq!(bus.read_byte(0xFF05), 0x01);
}

#[test]
fn tima_write_during_reload_cancels_pending_reload() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(16); // overflow -> pending reload (4 cycles)
    assert_eq!(bus.read_byte(0xFF05), 0x00);

    bus.write_byte(0xFF05, 0x99); // cancel reload
    bus.tick(4);

    assert_eq!(bus.read_byte(0xFF05), 0x99);
    assert_eq!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn tima_write_on_reload_cycle_is_ignored() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(20); // overflow + reload happened; reload block active
    assert_eq!(bus.read_byte(0xFF05), 0x42);

    bus.write_byte(0xFF05, 0x99); // ignored during reload block

    assert_eq!(bus.read_byte(0xFF05), 0x42);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn tma_write_on_reload_cycle_updates_reloaded_tima() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(19); // overflow happened, 1 t-cycle left for reload
    bus.write_byte(0xFF06, 0x99); // updates TMA and imminent reload value
    bus.tick(1);

    assert_eq!(bus.read_byte(0xFF05), 0x99);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}
