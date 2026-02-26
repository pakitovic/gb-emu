use super::super::Cpu;
use super::support::*;

#[test]
fn halt_with_pending_interrupt_and_ime_off_triggers_halt_bug() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.registers.pc = 0xC000;
    cpu.registers.b = 0x00;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xC001, 0x04); // INC B
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert!(!cpu.halted);
    assert!(cpu.halt_bug);
    assert_eq!(cpu.registers.pc, 0xC001);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.b, 0x01);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert!(!cpu.halt_bug);

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, 4);
    assert_eq!(cpu.registers.b, 0x02);
    assert_eq!(cpu.registers.pc, 0xC002);
}
#[test]
fn halt_bug_latch_persists_if_pending_if_is_cleared_before_next_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.registers.pc = 0xC000;
    cpu.registers.b = 0x00;
    bus.write_byte(0xC000, 0x76); // HALT
    bus.write_byte(0xC001, 0x04); // INC B (must still be duplicated by HALT bug)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert!(cpu.halt_bug);
    assert!(!cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    // Clear the pending source before the next step; the HALT bug latch should still apply once.
    bus.set_interrupt_flags(0x00);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.b, 0x01);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert!(!cpu.halt_bug);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, 4);
    assert_eq!(cpu.registers.b, 0x02);
    assert_eq!(cpu.registers.pc, 0xC002);
}
#[test]
fn ei_then_halt_with_pending_interrupt_services_without_leaking_halt_bug() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;

    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x76); // HALT
    bus.write_byte(0xC002, 0x00); // NOP (must not execute before interrupt service)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending from the start

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert!(!cpu.ime);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    // HALT executes while IME is still off and interrupt is pending => HALT bug path.
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(cpu.halt_bug);
    assert!(!cpu.halted);
    // EI delay expires at the end of the HALT step.
    assert!(cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.ime);
    // Interrupt dispatch must not leave a stale HALT bug latch for later instruction fetches.
    assert!(!cpu.halt_bug);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn ei_then_halt_halt_bug_executes_if_ie_masks_pending_before_next_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    cpu.registers.b = 0x00;

    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x76); // HALT
    bus.write_byte(0xC002, 0x04); // INC B (must be duplicated via HALT bug if dispatch is masked)
    bus.write_byte(0xC003, 0x00); // NOP
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending from the start

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert!(!cpu.ime);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(cpu.halt_bug);
    assert!(!cpu.halted);
    assert!(cpu.ime); // EI delay expires at end of HALT step
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    // Mask the pending interrupt before the next step. IME remains set, but dispatch must not occur.
    bus.write_byte(0xFFFF, 0x00);

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, 4);
    assert_eq!(cpu.registers.b, 0x01);
    assert_eq!(cpu.registers.pc, 0xC002); // HALT bug duplicate fetch path
    assert!(!cpu.halt_bug);
    assert!(cpu.ime); // no interrupt dispatch happened
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01); // IF stays pending but masked

    let cycles_4 = cpu.step(&mut bus);
    assert_eq!(cycles_4, 4);
    assert_eq!(cpu.registers.b, 0x02);
    assert_eq!(cpu.registers.pc, 0xC003);
}
#[test]
fn ei_then_halt_halt_bug_dispatch_uses_updated_if_source_before_next_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;

    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x76); // HALT
    bus.write_byte(0xC002, 0x00); // NOP (must not execute if interrupt is serviced)
    bus.write_byte(0xFFFF, 0x03); // IE: VBlank + STAT enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending (initial HALT-bug source)

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert!(!cpu.ime);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(cpu.halt_bug);
    assert!(!cpu.halted);
    assert!(cpu.ime); // EI delay expires at end of HALT step
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    // Change pending source before the next step while keeping a pending interrupt.
    bus.set_interrupt_flags(0x02); // IF: STAT only

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0048); // STAT vector
    assert!(!cpu.ime);
    assert!(!cpu.halt_bug); // dispatch clears halt-bug latch
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn ei_then_halt_halt_bug_dispatch_uses_updated_ie_source_before_next_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;

    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x76); // HALT
    bus.write_byte(0xC002, 0x00); // NOP (must not execute if interrupt is serviced)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank enabled (initial HALT-bug source)
    bus.set_interrupt_flags(0x03); // IF: VBlank + STAT pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert!(!cpu.ime);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(cpu.halt_bug);
    assert!(!cpu.halted);
    assert!(cpu.ime); // EI delay expires at end of HALT step
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x03);

    // Switch the enabled source before the next step while keeping pending interrupts.
    bus.write_byte(0xFFFF, 0x02); // IE: STAT only

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0048); // STAT vector
    assert!(!cpu.ime);
    assert!(!cpu.halt_bug); // dispatch clears halt-bug latch
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01); // VBlank remains pending but now masked
}
#[test]
fn ei_then_halt_halt_bug_dispatch_reevaluates_combined_if_ie_priority_before_next_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    cpu.registers.b = 0x00;

    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x76); // HALT
    bus.write_byte(0xC002, 0x04); // INC B (must be preempted if dispatch happens)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank only (initially)
    bus.set_interrupt_flags(0x01); // IF: VBlank pending (initial source)

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert!(cpu.halt_bug);
    assert!(cpu.ime); // EI delay expired at end of HALT step
    assert_eq!(cpu.registers.pc, 0xC002);

    // Replace both IF and IE with a different combined pending/enabled set.
    // Intersection becomes TIMER + JOYPAD; TIMER (bit2) must win by priority.
    bus.set_interrupt_flags(0x1C); // TIMER + SERIAL + JOYPAD pending
    bus.write_byte(0xFFFF, 0x14); // TIMER + JOYPAD enabled

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0050); // TIMER vector wins over JOYPAD
    assert_eq!(cpu.registers.b, 0x00); // HALT-bug duplicate fetch was preempted by dispatch
    assert!(!cpu.ime);
    assert!(!cpu.halt_bug);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x18); // SERIAL + JOYPAD remain pending
}
