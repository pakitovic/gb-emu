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
fn push_bc_returns_policy_derived_four_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    cpu.registers.b = 0x12;
    cpu.registers.c = 0x34;
    bus.write_byte(0xC000, 0xC5); // PUSH BC

    let expected = bus.cpu_tcycles_for_mcycles(4);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.registers.sp, 0xCFFE);
    assert_eq!(bus.read_byte(0xCFFF), 0x12);
    assert_eq!(bus.read_byte(0xCFFE), 0x34);
    assert_eq!(cpu.registers.pc, 0xC001);
}

#[test]
fn jr_nz_not_taken_returns_policy_derived_two_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.f = 0x80; // Z set => NZ condition fails
    bus.write_byte(0xC000, 0x20); // JR NZ,r8
    bus.write_byte(0xC001, 0x05);

    let expected = bus.cpu_tcycles_for_mcycles(2);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.registers.pc, 0xC002);
}

#[test]
fn call_a16_returns_policy_derived_six_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.sp = 0xD000;
    bus.write_byte(0xC000, 0xCD); // CALL a16
    bus.write_byte(0xC001, 0x34);
    bus.write_byte(0xC002, 0x12);

    let expected = bus.cpu_tcycles_for_mcycles(6);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.registers.pc, 0x1234);
    assert_eq!(cpu.registers.sp, 0xCFFE);
    assert_eq!(bus.read_byte(0xCFFF), 0xC0);
    assert_eq!(bus.read_byte(0xCFFE), 0x03);
}

#[test]
fn add_a_b_returns_policy_derived_one_mcycle() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.a = 0x10;
    cpu.registers.b = 0x22;
    bus.write_byte(0xC000, 0x80); // ADD A,B

    let expected = bus.cpu_tcycles_for_mcycles(1);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.registers.a, 0x32);
    assert_eq!(cpu.registers.pc, 0xC001);
}

#[test]
fn cp_hl_returns_policy_derived_two_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.a = 0x33;
    cpu.set_hl(0xC100);
    bus.write_byte(0xC000, 0xBE); // CP A,(HL)
    bus.write_byte(0xC100, 0x33);

    let expected = bus.cpu_tcycles_for_mcycles(2);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.registers.a, 0x33); // CP does not modify A
    assert_eq!(cpu.registers.pc, 0xC001);
}

#[test]
fn ld_b_c_returns_policy_derived_one_mcycle() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.b = 0x00;
    cpu.registers.c = 0x77;
    bus.write_byte(0xC000, 0x41); // LD B,C

    let expected = bus.cpu_tcycles_for_mcycles(1);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.registers.b, 0x77);
    assert_eq!(cpu.registers.pc, 0xC001);
}

#[test]
fn ld_hl_d8_returns_policy_derived_three_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.set_hl(0xC100);
    bus.write_byte(0xC000, 0x36); // LD (HL),d8
    bus.write_byte(0xC001, 0x5A);

    let expected = bus.cpu_tcycles_for_mcycles(3);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(bus.read_byte(0xC100), 0x5A);
    assert_eq!(cpu.registers.pc, 0xC002);
}

#[test]
fn add_hl_bc_returns_policy_derived_two_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.set_hl(0x1234);
    cpu.set_bc(0x0102);
    bus.write_byte(0xC000, 0x09); // ADD HL,BC

    let expected = bus.cpu_tcycles_for_mcycles(2);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.hl(), 0x1336);
    assert_eq!(cpu.registers.pc, 0xC001);
}

#[test]
fn cb_bit_7_b_returns_policy_derived_two_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.registers.b = 0x80;
    bus.write_byte(0xC000, 0xCB);
    bus.write_byte(0xC001, 0x78); // BIT 7,B

    let expected = bus.cpu_tcycles_for_mcycles(2);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(cpu.registers.b, 0x80);
    assert_eq!(cpu.registers.pc, 0xC002);
}

#[test]
fn cb_rlc_hl_returns_policy_derived_four_mcycles() {
    let mut cpu = Cpu::new();
    let mut bus = make_test_bus();

    cpu.registers.pc = 0xC000;
    cpu.set_hl(0xC120);
    bus.write_byte(0xC000, 0xCB);
    bus.write_byte(0xC001, 0x06); // RLC (HL)
    bus.write_byte(0xC120, 0x81);

    let expected = bus.cpu_tcycles_for_mcycles(4);
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, expected);
    assert_eq!(bus.read_byte(0xC120), 0x03);
    assert_eq!(cpu.registers.pc, 0xC002);
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
    assert_eq!(cycles_1, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF changes while HALTed, but IE still masks it

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
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
    assert_eq!(cycles_1, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF becomes pending while HALTed

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
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
    assert_eq!(cycles_1, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF pending while still masked
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.write_byte(0xFFFF, 0x01); // IE changes while HALTed and makes pending non-zero
    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, 4);
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
    assert_eq!(cycles_1, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF changes while HALTed, but IE still masks it

    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
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
    assert_eq!(cycles_1, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.set_interrupt_flags(0x01); // IF pending while still masked
    let cycles_2 = cpu.step(&mut bus);
    assert_eq!(cycles_2, 4);
    assert!(cpu.halted);
    assert_eq!(cpu.registers.pc, 0xC001);

    bus.write_byte(0xFFFF, 0x01); // IE enables pending IF while HALTed
    let cycles_3 = cpu.step(&mut bus);
    assert_eq!(cycles_3, 20);
    assert!(!cpu.halted);
    assert!(!cpu.ime);
    assert_eq!(cpu.registers.pc, 0x0040);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x00);
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
    assert_eq!(cycles_3, 20);
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
    assert_eq!(cycles_3, 20);
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
    assert_eq!(cycles_3, 20);
    assert_eq!(cpu.registers.pc, 0x0050); // TIMER vector wins over JOYPAD
    assert_eq!(cpu.registers.b, 0x00); // HALT-bug duplicate fetch was preempted by dispatch
    assert!(!cpu.ime);
    assert!(!cpu.halt_bug);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x18); // SERIAL + JOYPAD remain pending
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
    assert_eq!(cycles_3, 20);
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
    assert_eq!(cycles_3, 20);
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
    assert_eq!(cycles_3, 20);
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
    assert_eq!(cycles_3, 20);
    assert_eq!(cpu.registers.pc, 0x0048); // STAT vector wins over TIMER
    assert!(!cpu.ime);
    assert_eq!(bus.interrupt_flags() & 0x1F, 0x0C); // TIMER + SERIAL remain pending
}
