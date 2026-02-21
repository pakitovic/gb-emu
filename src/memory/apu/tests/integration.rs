use super::{HardwareModel, make_test_bus, make_test_bus_with_model};

#[test]
fn apu_boot_nr52_channel_status_bit_is_stable_after_first_tick() {
    let mut bus = make_test_bus_with_model(HardwareModel::Dmg);
    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x01);

    bus.tick(1);

    assert_eq!(bus.read_byte(0xFF26) & 0x0F, 0x01);
}

#[test]
fn apu_io_router_routes_control_registers_with_power_gating() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF26, 0x00);
    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0xF3);
    assert_eq!(bus.read_byte(0xFF24), 0x00);
    assert_eq!(bus.read_byte(0xFF25), 0x00);

    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0xF3);
    assert_eq!(bus.read_byte(0xFF24), 0x77);
    assert_eq!(bus.read_byte(0xFF25), 0xF3);
}

#[test]
fn apu_io_router_routes_channel_registers_and_status_bits() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF26, 0x80);
    bus.write_byte(0xFF24, 0x77);
    bus.write_byte(0xFF25, 0x22); // CH2 to left+right

    bus.write_byte(0xFF16, 0x80);
    bus.write_byte(0xFF17, 0xF0);
    bus.write_byte(0xFF18, 0xFC);
    bus.write_byte(0xFF19, 0x87); // trigger CH2
    bus.tick(8);

    assert_ne!(bus.read_byte(0xFF26) & 0x02, 0x00);
}

#[test]
fn apu_io_router_keeps_wave_ram_accessible_while_powered_off() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF26, 0x00);

    bus.write_byte(0xFF30, 0xAB);
    bus.write_byte(0xFF3F, 0xCD);

    assert_eq!(bus.read_byte(0xFF30), 0xAB);
    assert_eq!(bus.read_byte(0xFF3F), 0xCD);
}
