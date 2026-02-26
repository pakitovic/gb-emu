use super::super::Cpu;
use super::support::*;

#[test]
fn halt_without_pending_interrupts_stays_halted() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x76); // HALT

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, m_tcycles(&bus, 1));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, halt_idle_tcycles(&bus));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);
}
#[test]
fn halt_with_pending_interrupt_and_ime_on_dispatches_interrupt() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank
    bus.set_interrupt_flags(0x00); // clear post-boot IF defaults

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, m_tcycles(&bus, 1)); // HALT instruction itself
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF: VBlank pending after HALT
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, interrupt_service_tcycles(&bus));
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0040);
}
#[test]
fn halted_ime_off_does_not_wake_when_if_changes_without_enabled_ie() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xC001, 0x00); // NOP (must not execute while interrupt stays masked)
    bus.write_byte(0xFFFF, 0x00); // IE: all disabled
    bus.set_interrupt_flags(0x00);

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, m_tcycles(&bus, 1));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF changes while HALTed, but IE still masks it

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, halt_idle_tcycles(&bus));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);
}
#[test]
fn halted_ime_off_wakes_when_if_sets_enabled_interrupt_without_servicing() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xC001, 0x00); // NOP (should execute on wake)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x00);

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, m_tcycles(&bus, 1));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF becomes pending while HALTed

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, halt_idle_tcycles(&bus));
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0xC002); // NOP executed after wake (no dispatch)
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);
}
#[test]
fn halted_ime_off_wakes_when_ie_enables_pending_if_without_servicing() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xC001, 0x00); // NOP (should execute on wake)
    bus.write_byte(0xFFFF, 0x00); // IE initially masks interrupt
    bus.set_interrupt_flags(0x00);

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, m_tcycles(&bus, 1));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF pending while still masked
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, halt_idle_tcycles(&bus));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.write_byte(0xFFFF, 0x01); // IE changes while HALTed and makes pending non-zero
    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, halt_idle_tcycles(&bus));
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0xC002); // NOP executes; interrupt remains pending
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);
}
#[test]
fn halted_ime_on_does_not_wake_when_if_changes_without_enabled_ie() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xC001, 0x00); // NOP (must not execute while interrupt stays masked)
    bus.write_byte(0xFFFF, 0x00); // IE: all disabled
    bus.set_interrupt_flags(0x00);

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, m_tcycles(&bus, 1));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF changes while HALTed, but IE still masks it

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, halt_idle_tcycles(&bus));
    assert!(cpu.halted);
    assert!(cpu.ime);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);
}
#[test]
fn halted_ime_on_services_when_ie_enables_pending_if() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xC001, 0x00); // NOP (must be preempted by interrupt dispatch)
    bus.write_byte(0xFFFF, 0x00); // IE initially masks interrupt
    bus.set_interrupt_flags(0x00);

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, m_tcycles(&bus, 1));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF pending while still masked
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, halt_idle_tcycles(&bus));
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.write_byte(0xFFFF, 0x01); // IE enables pending IF while HALTed
    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0040);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn halt_with_ime_on_and_already_pending_interrupt_is_preempted_before_halt_executes() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank
    bus.set_interrupt_flags(0x01); // IF: VBlank already pending before HALT executes

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, interrupt_service_tcycles(&bus));
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    // Interrupt service preempts fetching/executing HALT when IME is already set.
    assert_eq!(cpu.registers.pc, 0x0040);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
