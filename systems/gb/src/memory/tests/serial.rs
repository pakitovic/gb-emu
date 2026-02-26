use super::*;

#[test]
fn serial_transfer_completes_after_eight_div_aligned_falling_edges() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'A');
    bus.write_byte(0xFF02, 0x81);

    for _ in 0..4095 {
        bus.tick(1);
    }

    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x80);

    bus.tick(1);

    assert_ne!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x00);
    assert!(bus.serial_output().contains('A'));
}

#[test]
fn serial_transfer_is_phase_aligned_to_div_and_not_to_start_write() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    // Shift DIV phase so completion is not exactly 4096 cycles after SC write.
    bus.tick(7);
    bus.write_byte(0xFF01, b'B');
    bus.write_byte(0xFF02, 0x81);

    for _ in 0..4088 {
        bus.tick(1);
    }
    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);

    bus.tick(1);
    assert_ne!(bus.interrupt_flags() & (1 << 3), 0);
}

#[test]
fn serial_stop_cancels_transfer_and_does_not_request_interrupt() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'X');
    bus.write_byte(0xFF02, 0x81);

    bus.tick(255); // before first serial clock falling edge
    bus.write_byte(0xFF02, 0x00); // explicit stop

    for _ in 0..5000 {
        bus.tick(1);
    }

    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x00);
    assert!(!bus.serial_output().contains('X'));
}

#[test]
fn serial_restart_uses_latest_tx_byte() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'A');
    bus.write_byte(0xFF02, 0x81); // start transfer

    bus.tick(200); // transfer still in progress
    bus.write_byte(0xFF01, b'B');
    bus.write_byte(0xFF02, 0x81); // restart transfer

    let mut finished = false;
    for _ in 0..5000 {
        bus.tick(1);
        if (bus.interrupt_flags() & (1 << 3)) != 0 {
            finished = true;
            break;
        }
    }

    assert!(finished, "serial transfer did not complete after restart");
    assert_eq!(bus.serial_output(), "B");
}
