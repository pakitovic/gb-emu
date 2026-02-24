use super::Cpu;
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;
use crate::memory::Bus;

fn make_test_bus() -> Bus {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    Bus::new(cart)
}

#[test]
fn new_with_model_sets_expected_boot_registers() {
    let assert_model = |model: HardwareModel, expected: (u8, u8, u8, u8, u8, u8, u8, u8)| {
        let cpu = Cpu::new_with_model(model);
        assert_eq!(
            (
                cpu.registers.a,
                cpu.registers.f,
                cpu.registers.b,
                cpu.registers.c,
                cpu.registers.d,
                cpu.registers.e,
                cpu.registers.h,
                cpu.registers.l
            ),
            expected
        );
        assert_eq!(cpu.registers.sp, 0xFFFE);
        assert_eq!(cpu.registers.pc, 0x0100);
    };

    assert_model(
        HardwareModel::Dmg0,
        (0x01, 0x00, 0xFF, 0x13, 0x00, 0xC1, 0x84, 0x03),
    );
    assert_model(
        HardwareModel::Dmg,
        (0x01, 0xB0, 0x00, 0x13, 0x00, 0xD8, 0x01, 0x4D),
    );
    assert_model(
        HardwareModel::Mgb,
        (0xFF, 0xB0, 0x00, 0x13, 0x00, 0xD8, 0x01, 0x4D),
    );
    assert_model(
        HardwareModel::Sgb,
        (0x01, 0x00, 0x00, 0x14, 0x00, 0x00, 0xC0, 0x60),
    );
    assert_model(
        HardwareModel::Sgb2,
        (0xFF, 0x00, 0x00, 0x14, 0x00, 0x00, 0xC0, 0x60),
    );
}

#[test]
fn push_and_pop_u16_roundtrip() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();
    cpu.registers.sp = 0xD000;

    cpu.push_u16(&mut bus, 0xBEEF);
    assert_eq!(cpu.registers.sp, 0xCFFE);

    let value = cpu.pop_u16(&mut bus);
    assert_eq!(value, 0xBEEF);
    assert_eq!(cpu.registers.sp, 0xD000);
}

#[test]
fn fetch_d16_reads_little_endian_and_advances_pc() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();
    cpu.registers.pc = 0xC100;

    bus.write_byte(0xC100, 0x34);
    bus.write_byte(0xC101, 0x12);

    let value = cpu.fetch_d16(&mut bus);
    assert_eq!(value, 0x1234);
    assert_eq!(cpu.registers.pc, 0xC102);
}

#[test]
fn pop_hl_pops_once_and_updates_sp_by_two() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0xE1); // POP HL
    bus.write_byte(0xD000, 0x34); // low byte
    bus.write_byte(0xD001, 0x12); // high byte

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(cpu.hl(), 0x1234);
    assert_eq!(cpu.registers.sp, 0xD002);
    assert_eq!(cpu.registers.pc, 0xC001);
}

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

    assert_eq!(cycles, 20);
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

    assert_eq!(cycles, 20);
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

    assert_eq!(cycles, 20);
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

    assert_eq!(cycles, 20);
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

    assert_eq!(cycles, 20);
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

    assert_eq!(cycles, 20);
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0040); // selection already latched before lower-byte push
    assert_eq!(cpu.registers.sp, 0xFF0F);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}

#[test]
fn halt_without_pending_interrupts_stays_halted() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    bus.write_byte(0xC000, 0x76); // HALT

    let cycles_1 = cpu.step(&mut bus);
    assert_eq!(cycles_1, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);
}

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
    assert_eq!(cycles_1, 4); // HALT instruction itself
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF: VBlank pending after HALT
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 20);
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0040);
}

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
    assert_eq!(cycles_3, 20);
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
    assert_eq!(cycles_2, 20);
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
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
    assert_eq!(cycles_3, 20);
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.ime);
    // Interrupt dispatch must not leave a stale HALT bug latch for later instruction fetches.
    assert!(!cpu.halt_bug);
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
    assert_eq!(cycles_1, 20);
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    // Interrupt service preempts fetching/executing HALT when IME is already set.
    assert_eq!(cpu.registers.pc, 0x0040);
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
    assert_eq!(cycles_2, 20);
    assert_eq!(cpu.registers.pc, 0x0040); // VBlank should win over STAT
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x02); // STAT remains pending
}

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

    assert_eq!(cycles, 20);
    assert_eq!(cpu.registers.pc, 0x0040);
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
}
