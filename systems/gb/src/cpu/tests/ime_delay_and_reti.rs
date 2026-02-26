use super::super::Cpu;
use super::support::*;

#[test]
fn ei_delays_pending_interrupt_service_until_after_next_instruction() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;

    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x00); // NOP (the instruction after EI)
    bus.write_byte(0xC002, 0x00); // NOP (would be next if no interrupt)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert!(!cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn di_cancels_pending_ei_enable_delay() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;

    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0xF3); // DI
    bus.write_byte(0xC002, 0x00); // NOP
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert!(!cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 1);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(!cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, 4);
    assert_eq!(cpu.registers.pc, 0xC003);
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);
}
#[test]
fn reti_enables_ime_immediately_for_next_step_interrupt_check() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;

    bus.write_byte(0xC000, 0xD9); // RETI
    bus.write_byte(0xD000, 0x34); // return addr low
    bus.write_byte(0xD001, 0x12); // return addr high
    bus.write_byte(0x1234, 0x00); // NOP at return target (must not execute yet)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 16);
    assert_eq!(cpu.registers.pc, 0x1234);
    assert_eq!(cpu.registers.sp, 0xD002);
    assert!(cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn reti_next_step_services_highest_priority_pending_interrupt() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;

    bus.write_byte(0xC000, 0xD9); // RETI
    bus.write_byte(0xD000, 0x34); // return addr low
    bus.write_byte(0xD001, 0x12); // return addr high
    bus.write_byte(0x1234, 0x00); // NOP (must be preempted)
    bus.write_byte(0xFFFF, 0x03); // IE: VBlank + STAT
    bus.set_interrupt_flags(0x03); // IF: VBlank + STAT pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 16);
    assert_eq!(cpu.registers.pc, 0x1234);
    assert!(cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x03);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0040); // VBlank should win over STAT
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x02); // STAT remains pending
}
