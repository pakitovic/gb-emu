use super::super::Cpu;
use super::support::*;

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
