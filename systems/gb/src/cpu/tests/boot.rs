use super::super::Cpu;
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
