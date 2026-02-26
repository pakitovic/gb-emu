use super::*;

#[test]
fn p1_reads_action_buttons_when_button_group_is_selected() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x10); // P15=0 (buttons), P14=1 (dpad not selected)
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);

    bus.set_button_pressed(Button::A, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);

    bus.set_button_pressed(Button::Start, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x06);
}

#[test]
fn p1_reads_dpad_buttons_when_direction_group_is_selected() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x20); // P14=0 (dpad), P15=1 (buttons not selected)
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);

    bus.set_button_pressed(Button::Right, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);

    bus.set_button_pressed(Button::Up, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0A);
}

#[test]
fn joypad_interrupt_is_requested_on_new_selected_press() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF00, 0x20); // select dpad

    bus.set_button_pressed(Button::Right, true);
    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_interrupt_flags(0x00);
    bus.set_button_pressed(Button::Right, true); // still pressed, no new edge
    assert_eq!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_button_pressed(Button::Right, false);
    assert_eq!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_button_pressed(Button::Right, true); // new falling edge
    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);
}

#[test]
fn joypad_interrupt_can_be_requested_when_selection_changes() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    bus.set_button_pressed(Button::A, true);
    bus.write_byte(0xFF00, 0x10); // select action keys after A is already pressed

    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);
}
