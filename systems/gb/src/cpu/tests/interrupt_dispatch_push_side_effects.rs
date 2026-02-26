use super::super::Cpu;
use super::support::*;

#[test]
fn interrupt_ie_push_upper_byte_can_cancel_dispatch() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0x0235;
    cpu.registers.sp = 0x0000;

    bus.write_byte(0xFFFF, 0x04);
    bus.set_interrupt_flags(0x04);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, interrupt_service_tcycles(&bus));
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0000);
    assert_eq!(bus.interrupt_enable(), 0x02);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x04);
}
#[test]
fn interrupt_ie_push_upper_byte_can_change_selected_vector() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0x0235;
    cpu.registers.sp = 0x0000;

    bus.write_byte(0xFFFF, 0x03);
    bus.set_interrupt_flags(0x03);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0048);
    assert_eq!(bus.interrupt_enable(), 0x02);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);
}
#[test]
fn interrupt_ie_push_lower_byte_is_too_late_to_cancel_dispatch() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0x0235;
    cpu.registers.sp = 0x0001;

    bus.write_byte(0xFFFF, 0x08);
    bus.set_interrupt_flags(0x08);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0058);
    assert_eq!(bus.interrupt_enable(), 0x35);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn interrupt_if_push_upper_byte_can_cancel_dispatch() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0x0035; // high byte = 0x00 (will clear IF when written to FF0F)
    cpu.registers.sp = 0xFF10; // upper-byte push lands on FF0F

    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, interrupt_service_tcycles(&bus));
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0000); // dispatch cancelled after IF changes
    assert_eq!(cpu.registers.sp, 0xFF0E);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn interrupt_if_push_upper_byte_can_change_selected_vector() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0x0235; // high byte = 0x02 (will write IF=STAT)
    cpu.registers.sp = 0xFF10; // upper-byte push lands on FF0F

    bus.write_byte(0xFFFF, 0x03); // IE: VBlank + STAT enabled
    bus.set_interrupt_flags(0x03); // IF: VBlank + STAT pending

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, interrupt_service_tcycles(&bus));
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0048); // STAT selected after IF rewrite
    assert_eq!(cpu.registers.sp, 0xFF0E);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn interrupt_if_push_lower_byte_is_too_late_to_cancel_dispatch() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0x0200; // low byte = 0x00 (would clear IF if it mattered)
    cpu.registers.sp = 0xFF11; // lower-byte push lands on FF0F

    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, interrupt_service_tcycles(&bus));
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0040); // selection already latched before lower-byte push
    assert_eq!(cpu.registers.sp, 0xFF0F);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
