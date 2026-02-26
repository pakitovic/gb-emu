use super::super::Cpu;
use super::support::*;

#[test]
fn stop_is_characterized_as_two_byte_control_noop_in_current_dmg_scope() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x10); // STOP
    bus.write_byte(0xC001, 0x99); // padding byte consumed by current implementation

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(!cpu.halted);
    assert!(cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);
}
#[test]
fn stop_with_ime_on_and_already_pending_interrupt_is_preempted_before_stop_executes() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0x10); // STOP (must be preempted)
    bus.write_byte(0xC001, 0x99); // padding byte (must not be consumed)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn stop_with_ime_on_and_masked_pending_interrupt_executes_current_noop_characterization() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = true;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x10); // STOP
    bus.write_byte(0xC001, 0x99); // padding byte consumed by current implementation
    bus.write_byte(0xFFFF, 0x00); // IE: all masked
    bus.set_interrupt_flags(0x01); // IF: VBlank pending but masked

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(!cpu.halted);
    assert!(cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01); // pending remains latched
}
#[test]
fn stop_with_ime_off_and_pending_interrupt_executes_current_noop_characterization() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x10); // STOP
    bus.write_byte(0xC001, 0x99); // padding byte consumed by current implementation
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.pc, 0xC002);
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01); // pending remains latched
}
#[test]
fn ei_then_stop_consumes_padding_before_delayed_interrupt_service() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x10); // STOP (the instruction after EI)
    bus.write_byte(0xC002, 0x99); // STOP padding byte
    bus.write_byte(0xC003, 0x00); // NOP (must be preempted after STOP completes)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank
    bus.set_interrupt_flags(0x01); // IF: VBlank pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert!(!cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC003); // STOP opcode + padding consumed
    assert!(!cpu.halted);
    assert!(cpu.ime); // EI delay expires after STOP step
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn ei_then_stop_delayed_dispatch_uses_updated_if_source_before_next_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x10); // STOP
    bus.write_byte(0xC002, 0x99); // STOP padding byte
    bus.write_byte(0xC003, 0x00); // NOP (must be preempted)
    bus.write_byte(0xFFFF, 0x03); // IE: VBlank + STAT enabled
    bus.set_interrupt_flags(0x01); // IF: VBlank pending (initial source)

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert!(!cpu.ime);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC003);
    assert!(cpu.ime); // EI delay expires after STOP step
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01);

    // Change pending source before the delayed service step.
    bus.set_interrupt_flags(0x02); // IF: STAT only

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0048); // STAT vector
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
#[test]
fn ei_then_stop_delayed_dispatch_uses_updated_ie_source_before_next_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x10); // STOP
    bus.write_byte(0xC002, 0x99); // STOP padding byte
    bus.write_byte(0xC003, 0x00); // NOP (must be preempted)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank only (initially)
    bus.set_interrupt_flags(0x03); // IF: VBlank + STAT pending

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert_eq!(cpu.registers.pc, 0xC001);
    assert_eq!(cpu.ime_enable_delay, 1);
    assert!(!cpu.ime);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC003);
    assert!(cpu.ime);
    assert_eq!(cpu.ime_enable_delay, 0);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x03);

    // Switch enabled source before the delayed service step.
    bus.write_byte(0xFFFF, 0x02); // IE: STAT only

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0048); // STAT vector
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x01); // VBlank remains pending but masked
}
#[test]
fn ei_then_stop_delayed_dispatch_reevaluates_combined_if_ie_priority_before_service_step() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.ime = false;
    cpu.ime_enable_delay = 0;
    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0xFB); // EI
    bus.write_byte(0xC001, 0x10); // STOP
    bus.write_byte(0xC002, 0x99); // STOP padding
    bus.write_byte(0xC003, 0x00); // NOP (must be preempted by delayed dispatch)
    bus.write_byte(0xFFFF, 0x01); // IE: VBlank only (initially)
    bus.set_interrupt_flags(0x01); // IF: VBlank pending (initial source)

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert_eq!(cpu.registers.pc, 0xC003); // STOP consumed opcode + padding
    assert!(cpu.ime); // EI delay expired at end of STOP step

    // Replace both IF and IE before the delayed service step.
    // Intersection becomes STAT + TIMER; STAT (bit1) must win by priority.
    bus.set_interrupt_flags(0x0E); // STAT + TIMER + SERIAL pending
    bus.write_byte(0xFFFF, 0x06); // STAT + TIMER enabled

    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, interrupt_service_tcycles(&bus));
    assert_eq!(cpu.registers.pc, 0x0048); // STAT vector wins over TIMER
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x0C); // TIMER + SERIAL remain pending
}
