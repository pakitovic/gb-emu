use super::super::ApuState;
use super::super::constants::*;
use super::super::mmio::{ApuRegister, decode_register};
use crate::hardware::HardwareModel;

fn make_apu_with_power(enabled: bool) -> (ApuState, [u8; 0x80]) {
    let mut io = [0u8; 0x80];
    io[NR52_INDEX] = if enabled { 0x80 } else { 0x00 };
    let apu = ApuState::from_boot_state(&io, HardwareModel::Dmg);
    (apu, io)
}

fn channel_on_mask(apu: &ApuState) -> u8 {
    apu.read_io_register(0xFF26).unwrap_or_default() & 0x0F
}

#[test]
fn apu_mmio_decode_maps_channel_control_and_wave_registers() {
    assert_eq!(decode_register(0xFF10), Some(ApuRegister::Nr10));
    assert_eq!(decode_register(0xFF24), Some(ApuRegister::Nr50));
    assert_eq!(decode_register(0xFF26), Some(ApuRegister::Nr52));
    assert_eq!(
        decode_register(0xFF30),
        Some(ApuRegister::WaveRam(WAVE_RAM_START_INDEX))
    );
    assert_eq!(
        decode_register(0xFF3F),
        Some(ApuRegister::WaveRam(WAVE_RAM_END_INDEX))
    );
}

#[test]
fn apu_mmio_decode_marks_unknown_io_slots_as_other_and_rejects_non_io_addresses() {
    assert_eq!(decode_register(0xFF15), Some(ApuRegister::Other(0x15)));
    assert_eq!(decode_register(0xFF27), Some(ApuRegister::Other(0x27)));
    assert_eq!(decode_register(0xFF80), None);
    assert_eq!(decode_register(0xFEFF), None);
}

#[test]
fn apu_mmio_power_gating_blocks_channel_writes_but_keeps_wave_ram_writable() {
    let (mut apu, mut io) = make_apu_with_power(false);

    apu.write_io_register(&mut io, 0xFF12, 0xF0);
    assert_eq!(io[NR12_INDEX], 0x00);

    apu.write_io_register(&mut io, 0xFF30, 0xAB);
    assert_eq!(io[WAVE_RAM_START_INDEX], 0xAB);
}

#[test]
fn apu_mmio_nr50_nr51_writes_require_apu_power() {
    let (mut apu, mut io) = make_apu_with_power(false);

    apu.write_io_register(&mut io, 0xFF24, 0x77);
    apu.write_io_register(&mut io, 0xFF25, 0xF3);
    assert_eq!(io[NR50_INDEX], 0x00);
    assert_eq!(io[NR51_INDEX], 0x00);

    apu.write_io_register(&mut io, 0xFF26, 0x80);
    apu.write_io_register(&mut io, 0xFF24, 0x77);
    apu.write_io_register(&mut io, 0xFF25, 0xF3);
    assert_eq!(io[NR50_INDEX], 0x77);
    assert_eq!(io[NR51_INDEX], 0xF3);
}

#[test]
fn apu_mmio_nr52_power_toggle_clears_apu_register_window() {
    let (mut apu, mut io) = make_apu_with_power(true);
    io[NR10_INDEX] = 0x11;
    io[NR12_INDEX] = 0xF0;
    io[NR50_INDEX] = 0x77;
    io[NR51_INDEX] = 0xF3;

    apu.write_io_register(&mut io, 0xFF26, 0x00);

    assert_eq!(io[NR52_INDEX], 0x00);
    for register in io.iter().take(NR51_INDEX + 1).skip(NR10_INDEX) {
        assert_eq!(*register, 0x00);
    }

    apu.write_io_register(&mut io, 0xFF26, 0x80);
    assert_eq!(io[NR52_INDEX], 0x80);
}

#[test]
fn apu_mmio_dispatch_square1_registers_updates_io_and_channel_status() {
    let (mut apu, mut io) = make_apu_with_power(true);

    apu.write_io_register(&mut io, 0xFF10, 0x00);
    apu.write_io_register(&mut io, 0xFF11, 0x80);
    apu.write_io_register(&mut io, 0xFF12, 0xF0);
    apu.write_io_register(&mut io, 0xFF13, 0xFC);
    apu.write_io_register(&mut io, 0xFF14, 0x87);

    assert_eq!(io[NR10_INDEX], 0x00);
    assert_eq!(io[NR11_INDEX], 0x80);
    assert_eq!(io[NR12_INDEX], 0xF0);
    assert_eq!(io[NR13_INDEX], 0xFC);
    assert_eq!(io[NR14_INDEX], 0x87);
    assert_ne!(channel_on_mask(&apu) & 0x01, 0x00);
}

#[test]
fn apu_mmio_dispatch_square2_registers_updates_io_and_channel_status() {
    let (mut apu, mut io) = make_apu_with_power(true);

    apu.write_io_register(&mut io, 0xFF16, 0x80);
    apu.write_io_register(&mut io, 0xFF17, 0xF0);
    apu.write_io_register(&mut io, 0xFF18, 0xFC);
    apu.write_io_register(&mut io, 0xFF19, 0x87);

    assert_eq!(io[NR21_INDEX], 0x80);
    assert_eq!(io[NR22_INDEX], 0xF0);
    assert_eq!(io[NR23_INDEX], 0xFC);
    assert_eq!(io[NR24_INDEX], 0x87);
    assert_ne!(channel_on_mask(&apu) & 0x02, 0x00);
}

#[test]
fn apu_mmio_dispatch_wave_registers_updates_io_and_channel_status() {
    let (mut apu, mut io) = make_apu_with_power(true);

    apu.write_io_register(&mut io, 0xFF30, 0xAB);
    apu.write_io_register(&mut io, 0xFF1A, 0x80);
    apu.write_io_register(&mut io, 0xFF1B, 0x20);
    apu.write_io_register(&mut io, 0xFF1C, 0x20);
    apu.write_io_register(&mut io, 0xFF1D, 0xFC);
    apu.write_io_register(&mut io, 0xFF1E, 0x87);

    assert_eq!(io[WAVE_RAM_START_INDEX], 0xAB);
    assert_eq!(io[NR30_INDEX], 0x80);
    assert_eq!(io[NR31_INDEX], 0x20);
    assert_eq!(io[NR32_INDEX], 0x20);
    assert_eq!(io[NR33_INDEX], 0xFC);
    assert_eq!(io[NR34_INDEX], 0x87);
    assert_ne!(channel_on_mask(&apu) & 0x04, 0x00);
}

#[test]
fn apu_mmio_dispatch_noise_registers_updates_io_and_channel_status() {
    let (mut apu, mut io) = make_apu_with_power(true);

    apu.write_io_register(&mut io, 0xFF20, 0x3F);
    apu.write_io_register(&mut io, 0xFF21, 0xF0);
    apu.write_io_register(&mut io, 0xFF22, 0x00);
    apu.write_io_register(&mut io, 0xFF23, 0x80);

    assert_eq!(io[NR41_INDEX], 0x3F);
    assert_eq!(io[NR42_INDEX], 0xF0);
    assert_eq!(io[NR43_INDEX], 0x00);
    assert_eq!(io[NR44_INDEX], 0x80);
    assert_ne!(channel_on_mask(&apu) & 0x08, 0x00);
}
