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
