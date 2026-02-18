use super::*;
use crate::hardware::HardwareModel;

fn make_test_bus() -> Bus {
    make_test_bus_with_model(HardwareModel::default())
}

fn make_test_bus_with_model(model: HardwareModel) -> Bus {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00; // ROM-only
    rom[0x0148] = 0x00; // 32KB
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    let mut bus = Bus::new_with_model(cart, model);
    // Unit tests use a neutral baseline instead of post-boot runtime defaults.
    bus.write_byte(0xFF04, 0x00); // DIV
    bus.write_byte(0xFF44, 0x00); // LY
    bus
}

fn wait_for_transition(bus: &mut Bus, ly: u8, from_mode: u8, to_mode: u8) {
    let mut prev_mode = bus.read_byte(0xFF41) & 0x03;
    for _ in 0..(154 * 456 * 2) {
        bus.tick(1);
        let cur_mode = bus.read_byte(0xFF41) & 0x03;
        let cur_ly = bus.read_byte(0xFF44);
        if cur_ly == ly && prev_mode == from_mode && cur_mode == to_mode {
            return;
        }
        prev_mode = cur_mode;
    }
    panic!("Transition LY={ly} {from_mode}->{to_mode} not observed");
}

fn measure_hblank_until_ly_increment(bus: &mut Bus, ly: u8) -> u16 {
    let mut ticks = 0u16;
    for _ in 0..512 {
        if bus.read_byte(0xFF44) != ly {
            return ticks;
        }
        bus.tick(1);
        ticks = ticks.wrapping_add(1);
    }
    panic!("LY did not increment within expected HBlank window");
}

fn wait_for_ly(bus: &mut Bus, target_ly: u8) {
    for _ in 0..(154 * 456 * 2) {
        if bus.read_byte(0xFF44) == target_ly {
            return;
        }
        bus.tick(1);
    }
    panic!("LY={target_ly} not observed");
}

#[test]
fn echo_ram_mirrors_work_ram() {
    let mut bus = make_test_bus();
    bus.write_byte(0xC123, 0xAB);
    assert_eq!(bus.read_byte(0xE123), 0xAB);

    bus.write_byte(0xE456, 0xCD);
    assert_eq!(bus.read_byte(0xC456), 0xCD);
}

#[test]
fn div_increments_every_256_tcycles_and_resets_on_write() {
    let mut bus = make_test_bus();
    assert_eq!(bus.read_byte(0xFF04), 0x00);

    bus.tick(255);
    assert_eq!(bus.read_byte(0xFF04), 0x00);

    bus.tick(1);
    assert_eq!(bus.read_byte(0xFF04), 0x01);

    bus.write_byte(0xFF04, 0x99);
    assert_eq!(bus.read_byte(0xFF04), 0x00);
}

#[test]
fn timer_overflow_reloads_tma_and_requests_interrupt() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF07, 0x05); // TAC: enable + 16 t-cycles period
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(16);
    assert_eq!(bus.read_byte(0xFF05), 0x00);
    assert_eq!(bus.interrupt_flags() & (1 << 2), 0);

    bus.tick(4);

    assert_eq!(bus.read_byte(0xFF05), 0x42);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn div_write_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // div bit3 becomes high
    bus.write_byte(0xFF04, 0x00); // reset DIV => falling edge => TIMA++

    assert_eq!(bus.read_byte(0xFF05), 0x01);
}

#[test]
fn tima_write_during_reload_cancels_pending_reload() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(16); // overflow -> pending reload (4 cycles)
    assert_eq!(bus.read_byte(0xFF05), 0x00);

    bus.write_byte(0xFF05, 0x99); // cancel reload
    bus.tick(4);

    assert_eq!(bus.read_byte(0xFF05), 0x99);
    assert_eq!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn tima_write_on_reload_cycle_is_ignored() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(20); // overflow + reload happened; reload block active
    assert_eq!(bus.read_byte(0xFF05), 0x42);

    bus.write_byte(0xFF05, 0x99); // ignored during reload block

    assert_eq!(bus.read_byte(0xFF05), 0x42);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn tma_write_on_reload_cycle_updates_reloaded_tima() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF06, 0x42); // TMA
    bus.write_byte(0xFF05, 0xFF); // TIMA

    bus.tick(19); // overflow happened, 1 t-cycle left for reload
    bus.write_byte(0xFF06, 0x99); // updates TMA and imminent reload value
    bus.tick(1);

    assert_eq!(bus.read_byte(0xFF05), 0x99);
    assert_ne!(bus.interrupt_flags() & (1 << 2), 0);
}

#[test]
fn serial_transfer_completes_after_eight_div_aligned_falling_edges() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'A');
    bus.write_byte(0xFF02, 0x81);

    for _ in 0..4095 {
        bus.tick(1);
    }

    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x80);

    bus.tick(1);

    assert_ne!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x00);
    assert!(bus.serial_output().contains('A'));
}

#[test]
fn serial_transfer_is_phase_aligned_to_div_and_not_to_start_write() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    // Shift DIV phase so completion is not exactly 4096 cycles after SC write.
    bus.tick(7);
    bus.write_byte(0xFF01, b'B');
    bus.write_byte(0xFF02, 0x81);

    for _ in 0..4088 {
        bus.tick(1);
    }
    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);

    bus.tick(1);
    assert_ne!(bus.interrupt_flags() & (1 << 3), 0);
}

#[test]
fn lcdc_enable_starts_with_special_line0_timing() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF40, 0x00); // LCD off
    bus.write_byte(0x8000, 0x12);
    bus.write_byte(0xFE00, 0x34);

    bus.write_byte(0xFF40, 0x80); // LCD on
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x00); // mode 0
    assert_eq!(bus.read_byte(0x8000), 0x12);
    assert_eq!(bus.read_byte(0xFE00), 0x34);

    bus.tick(79);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x00); // startup mode 0 lasts 80 t-cycles
    assert_eq!(bus.read_byte(0x8000), 0x12);
    assert_eq!(bus.read_byte(0xFE00), 0x34);

    bus.tick(1);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x03); // mode 3
    assert_eq!(bus.read_byte(0x8000), 0xFF);
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    bus.tick(172);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x00); // back to mode 0
    assert_eq!(bus.read_byte(0x8000), 0x12);
    assert_eq!(bus.read_byte(0xFE00), 0x34);
}

#[test]
fn startup_mode0_slice_masks_lyc_on_stat_read() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF40, 0x00); // LCD off
    bus.write_byte(0xFF45, 0x01); // LYC=1
    bus.write_byte(0xFF40, 0x80); // LCD on

    wait_for_ly(&mut bus, 0x01);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x00); // startup mode 0 slice
    assert_eq!(bus.read_byte(0xFF41) & 0x04, 0x00); // LYC masked in read value

    bus.tick(4);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x02); // mode 2
    assert_ne!(bus.read_byte(0xFF41) & 0x04, 0x00); // LYC visible again
}

#[test]
fn startup_mode0_slice_blocks_oam_reads_before_normal_hblank() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF40, 0x00); // LCD off
    bus.write_byte(0xFE00, 0x12);
    bus.write_byte(0xFF40, 0x80); // LCD on

    wait_for_ly(&mut bus, 0x01);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x00);
    assert_eq!(bus.read_byte(0xFE00), 0xFF); // blocked in startup mode0 slice

    let mut saw_open = false;
    for _ in 0..456 {
        if bus.read_byte(0xFF44) != 0x01 {
            break;
        }
        if (bus.read_byte(0xFF41) & 0x03) == 0x00 && bus.read_byte(0xFE00) == 0x12 {
            saw_open = true;
            break;
        }
        bus.tick(1);
    }
    assert!(saw_open, "OAM should become readable in normal mode0");
}

#[test]
fn startup_mode2_tail_blocks_vram_reads() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF40, 0x00); // LCD off
    bus.write_byte(0x8000, 0x34);
    bus.write_byte(0xFF40, 0x80); // LCD on

    wait_for_ly(&mut bus, 0x01);
    let mut saw_allowed = false;
    let mut saw_blocked = false;
    for _ in 0..456 {
        if bus.read_byte(0xFF44) != 0x01 {
            break;
        }
        if (bus.read_byte(0xFF41) & 0x03) == 0x02 {
            if bus.read_byte(0x8000) == 0xFF {
                saw_blocked = true;
            } else {
                saw_allowed = true;
            }
        }
        bus.tick(1);
    }

    assert!(saw_allowed, "VRAM should be readable in early mode2");
    assert!(
        saw_blocked,
        "VRAM should be blocked in late mode2 startup tail"
    );
}

#[test]
fn lyc_flag_is_retained_while_lcd_is_disabled() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF41, 0x40); // enable LY=LYC source
    bus.write_byte(0xFF45, 0x00); // LYC=0, LY=0
    assert_ne!(bus.read_byte(0xFF41) & 0x04, 0);

    bus.write_byte(0xFF40, 0x00); // LCD off
    assert_ne!(bus.read_byte(0xFF41) & 0x04, 0);

    bus.write_byte(0xFF45, 0x01); // no effect while LCD is off
    assert_ne!(bus.read_byte(0xFF41) & 0x04, 0);

    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF40, 0x80); // LCD on, LY=0 vs LYC=1 => bit clears
    assert_eq!(bus.read_byte(0xFF41) & 0x04, 0);
    assert_eq!(bus.interrupt_flags() & (1 << 1), 0);
}

#[test]
fn stat_irq_is_edge_triggered_when_enabling_mode1_source() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    for _ in 0..(144 * 456) {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x01); // mode 1 (vblank)

    bus.write_byte(0xFF41, 0x10); // enable mode1 source while already in mode1
    assert_ne!(bus.interrupt_flags() & (1 << 1), 0);

    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF41, 0x10); // line already high => no new edge
    assert_eq!(bus.interrupt_flags() & (1 << 1), 0);
}

#[test]
fn entering_vblank_requests_vblank_interrupt() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    // 144 scanlines * 456 t-cycles per line.
    for _ in 0..(144 * 456) {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte(0xFF44), 144);
    assert_ne!(bus.interrupt_flags() & (1 << 0), 0);
}

#[test]
fn scx_penalty_shortens_hblank_on_visible_lines() {
    let mut bus = make_test_bus();
    // Make sure we are in normal rendering, not the startup line.
    for _ in 0..(456 * 2) {
        bus.tick(1);
    }

    bus.write_byte(0xFF43, 0x00);
    wait_for_transition(&mut bus, 0x42, 0x03, 0x00);
    let delay_scx0 = measure_hblank_until_ly_increment(&mut bus, 0x42);

    bus.write_byte(0xFF43, 0x05);
    wait_for_transition(&mut bus, 0x43, 0x03, 0x00);
    let delay_scx5 = measure_hblank_until_ly_increment(&mut bus, 0x43);

    assert_eq!(delay_scx0, 204);
    assert_eq!(delay_scx5, 199);
}

#[test]
fn mode2_interrupt_source_is_active_on_ly144_entry() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF41, 0x20); // mode 2 STAT source

    for _ in 0..(144 * 456) {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte(0xFF44), 144);
    assert_ne!(bus.interrupt_flags() & (1 << 1), 0);
}

#[test]
fn stat_mode0_irq_to_ly_increment_matches_scx_groups() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF41, 0x08); // mode 0 source only

    let mut delays = [0u16; 8];
    for (scx, delay_out) in delays.iter_mut().enumerate() {
        bus.write_byte(0xFF43, scx as u8);

        while bus.read_byte(0xFF44) != 0x41 {
            bus.tick(1);
        }
        while bus.read_byte(0xFF44) == 0x41 {
            bus.tick(1);
        }

        bus.set_interrupt_flags(0x00);

        for _ in 0..456 {
            bus.tick(1);
            if (bus.interrupt_flags() & (1 << 1)) != 0 {
                break;
            }
        }
        assert_ne!(
            bus.interrupt_flags() & (1 << 1),
            0,
            "mode0 STAT IRQ did not trigger for SCX={scx}"
        );

        let start_ly = bus.read_byte(0xFF44);
        let mut delay = 0u16;
        for _ in 0..456 {
            if bus.read_byte(0xFF44) != start_ly {
                break;
            }
            bus.tick(1);
            delay = delay.wrapping_add(1);
        }
        *delay_out = delay;
    }

    assert_eq!(delays[0], 200);
    assert_eq!(delays[1], 199);
    assert_eq!(delays[2], 198);
    assert_eq!(delays[3], 197);
    assert_eq!(delays[4], 196);
    assert_eq!(delays[5], 195);
    assert_eq!(delays[6], 194);
    assert_eq!(delays[7], 193);
}

#[test]
fn oam_dma_transfers_160_bytes_and_finishes_after_start_delay() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFE00, 0xAA);
    bus.write_byte(0x8000, 0x12);
    bus.write_byte(0x809F, 0x34);

    bus.write_byte(0xFF46, 0x80);
    // Fresh DMA keeps OAM accessible for one M-cycle.
    assert_eq!(bus.read_byte(0xFE00), 0xAA);

    bus.tick(8);
    // DMA starts at M=2; OAM reads are now blocked.
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..640 {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte_raw(0xFE00), 0x12);
    assert_eq!(bus.read_byte_raw(0xFE9F), 0x34);
}

#[test]
fn oam_dma_blocks_cpu_writes_to_oam_during_transfer() {
    let mut bus = make_test_bus();
    bus.write_byte(0x8000, 0x55);
    bus.write_byte(0xFF46, 0x80);
    bus.tick(8);

    bus.write_byte(0xFE00, 0xAA); // ignored while DMA active
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..640 {
        bus.tick(1);
    }

    assert_eq!(bus.read_byte_raw(0xFE00), 0x55);
}

#[test]
fn oam_dma_restart_switches_source_after_two_mcycles() {
    let mut bus = make_test_bus();
    bus.write_byte(0x8000, 0x11);
    bus.write_byte(0x8100, 0x22);

    bus.write_byte(0xFF46, 0x80);
    bus.tick(8); // DMA starts
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    bus.write_byte(0xFF46, 0x81); // request restart
    bus.tick(4); // M=1 after restart request
    assert_eq!(bus.read_byte(0xFE00), 0xFF);

    for _ in 0..644 {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte_raw(0xFE00), 0x22);
}

#[test]
fn oam_dma_remaps_fe_ff_sources_to_de_df_on_dmg() {
    let mut bus = make_test_bus();
    bus.write_byte(0xDE00, 0x66);
    bus.write_byte(0xDF00, 0x77);
    bus.write_byte(0xFE00, 0x11);

    bus.write_byte(0xFF46, 0xFE);
    for _ in 0..648 {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte_raw(0xFE00), 0x66);

    bus.write_byte(0xFF46, 0xFF);
    for _ in 0..648 {
        bus.tick(1);
    }
    assert_eq!(bus.read_byte_raw(0xFE00), 0x77);
}

#[test]
fn unmapped_io_reads_as_ff_and_ignores_writes() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF03, 0x00);
    assert_eq!(bus.read_byte(0xFF03), 0xFF);

    bus.write_byte(0xFF4C, 0x00);
    assert_eq!(bus.read_byte(0xFF4C), 0xFF);

    bus.write_byte(0xFF4C, 0xAA);
    assert_eq!(bus.read_byte(0xFF4C), 0xFF);
}

#[test]
fn io_register_unused_bits_read_back_as_one() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x00);
    assert_eq!(bus.read_byte(0xFF00) & 0xC0, 0xC0);

    bus.write_byte(0xFF02, 0x00);
    assert_eq!(bus.read_byte(0xFF02) & 0x7E, 0x7E);

    bus.write_byte(0xFF07, 0x00);
    assert_eq!(bus.read_byte(0xFF07) & 0xF8, 0xF8);

    bus.write_byte(0xFF41, 0x00);
    assert_eq!(bus.read_byte(0xFF41) & 0x80, 0x80);

    bus.write_byte(0xFF1A, 0x00);
    assert_eq!(bus.read_byte(0xFF1A) & 0x7F, 0x7F);

    bus.write_byte(0xFF26, 0x00);
    assert_eq!(bus.read_byte(0xFF26) & 0x70, 0x70);
}

#[test]
fn dmg0_boot_profile_uses_expected_div_phase_and_ly_start() {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00;
    rom[0x0148] = 0x00;
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    let bus = Bus::new_with_model(cart, HardwareModel::Dmg0);

    assert_eq!(bus.div_counter, 0x1830);
    assert_eq!(bus.io[0x44], 0x91);
    assert_eq!(bus.ly_counter, 96);
}

#[test]
fn sgb_boot_div_phase_depends_on_header_checksum() {
    let make_bus = |checksum_hi: u8, checksum_lo: u8| {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        rom[0x014E] = checksum_hi;
        rom[0x014F] = checksum_lo;
        let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
        Bus::new_with_model(cart, HardwareModel::Sgb)
    };

    // boot_div-S.gb checksum bytes at 0x014E/0x014F.
    let bus_a = make_bus(0x34, 0x12);
    assert_eq!(bus_a.div_counter, 0xD860);

    // boot_div2-S.gb checksum bytes at 0x014E/0x014F.
    let bus_b = make_bus(0x96, 0xA7);
    assert_eq!(bus_b.div_counter, 0xD850);
}
