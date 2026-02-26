use super::super::Cpu;
use super::support::*;
use crate::hardware::HardwareModel;

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
