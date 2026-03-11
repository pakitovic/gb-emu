use super::*;
use crate::sgb::CMD_MLT_REQ;

fn feed_sgb_packet_via_p1(bus: &mut Bus, packet: &[u8; 16]) {
    bus.write_byte(0xFF00, 0x00);
    for byte in packet {
        for bit in 0..8 {
            let bit_value = (byte >> bit) & 0x01;
            let p1_write = if bit_value == 0 { 0x20 } else { 0x10 };
            bus.write_byte(0xFF00, p1_write);
        }
    }
    bus.write_byte(0xFF00, 0x20);
}

fn make_single_packet_command(command_id: u8, payload: &[u8]) -> [u8; 16] {
    let mut packet = [0u8; 16];
    packet[0] = (command_id << 3) | 0x01;
    for (index, value) in payload.iter().copied().enumerate() {
        if index + 1 >= packet.len() {
            break;
        }
        packet[index + 1] = value;
    }
    packet
}

fn select_sgb_player(bus: &mut Bus, target_player: u8) {
    bus.write_byte(0xFF00, 0x30);
    while bus.current_joypad_player_index() != target_player {
        bus.write_byte(0xFF00, 0x10);
        bus.write_byte(0xFF00, 0x30);
    }
}

#[test]
fn p1_reads_action_buttons_when_button_group_is_selected() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x10); // P15=0 (buttons), P14=1 (dpad not selected)
    assert_eq!(bus.read_byte(0xFF00) & 0xF0, 0xD0);
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
    assert_eq!(bus.read_byte(0xFF00) & 0xF0, 0xE0);
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

#[test]
fn recent_key_mmio_writes_tracks_ff00_and_ff40_in_write_order() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x20);
    bus.write_byte(0xFF40, 0x00);
    bus.write_byte(0xFF40, 0x91);

    assert_eq!(
        bus.recent_key_mmio_writes(),
        vec![(0xFF00, 0x20), (0xFF40, 0x00), (0xFF40, 0x91)]
    );
}

#[test]
fn drain_key_mmio_write_events_returns_ordered_events_and_clears_queue() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x20);
    bus.tick(4);
    bus.write_byte(0xFF40, 0x91);

    let events = bus.drain_key_mmio_write_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].addr, 0xFF00);
    assert_eq!(events[0].value, 0x20);
    assert_eq!(events[0].tcycle, 0);
    assert_eq!(events[1].addr, 0xFF40);
    assert_eq!(events[1].value, 0x91);
    assert_eq!(events[1].tcycle, 4);
    assert!(bus.drain_key_mmio_write_events().is_empty());
}

#[test]
fn sgb_mlt_req_enables_multiplayer_and_cycles_player_id_on_p15_rising_edge() {
    let mut bus = make_test_bus_with_model(HardwareModel::Sgb);
    let mlt_req = make_single_packet_command(CMD_MLT_REQ, &[0x01]); // 2 players

    feed_sgb_packet_via_p1(&mut bus, &mlt_req);

    assert_eq!(bus.joypad_player_count(), 2);

    bus.write_byte(0xFF00, 0x30);
    let first_player = bus.current_joypad_player_index();
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F - first_player);

    let previous_player = bus.current_joypad_player_index();
    bus.write_byte(0xFF00, 0x10);
    bus.write_byte(0xFF00, 0x30);
    assert_eq!(bus.current_joypad_player_index(), previous_player ^ 0x01);
    assert_eq!(
        bus.read_byte(0xFF00) & 0x0F,
        0x0F - bus.current_joypad_player_index()
    );
}

#[test]
fn sgb_mlt_req_routes_selected_player_inputs_through_p1() {
    let mut bus = make_test_bus_with_model(HardwareModel::Sgb);
    let mlt_req = make_single_packet_command(CMD_MLT_REQ, &[0x01]); // 2 players
    feed_sgb_packet_via_p1(&mut bus, &mlt_req);

    select_sgb_player(&mut bus, 0);
    assert_eq!(bus.current_joypad_player_index(), 0);

    assert!(bus.set_player_button_pressed(0, Button::Right, true));
    assert!(bus.set_player_button_pressed(1, Button::A, true));

    bus.write_byte(0xFF00, 0x20);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);

    select_sgb_player(&mut bus, 1);
    bus.write_byte(0xFF00, 0x20);
    assert_eq!(bus.current_joypad_player_index(), 1);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);

    bus.write_byte(0xFF00, 0x10);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);
}

#[test]
fn non_sgb_models_ignore_mlt_req_and_keep_single_player_mode() {
    let mut bus = make_test_bus();
    let mlt_req = make_single_packet_command(CMD_MLT_REQ, &[0x01]);

    feed_sgb_packet_via_p1(&mut bus, &mlt_req);

    assert_eq!(bus.joypad_player_count(), 1);
    assert_eq!(bus.current_joypad_player_index(), 0);
    bus.write_byte(0xFF00, 0x30);
    assert_eq!(bus.read_byte(0xFF00), 0xFF);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);
}

#[test]
fn joyp_reads_keep_unused_high_bits_set() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x30);
    assert_eq!(bus.read_byte(0xFF00), 0xFF);

    bus.write_byte(0xFF00, 0x20);
    assert_eq!(bus.read_byte(0xFF00) & 0xC0, 0xC0);

    bus.write_byte(0xFF00, 0x10);
    assert_eq!(bus.read_byte(0xFF00) & 0xC0, 0xC0);
}
