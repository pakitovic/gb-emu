use super::*;
use crate::hardware::HardwareModel;
use crate::input::Button;

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

fn wait_for_visible_hblank(bus: &mut Bus) {
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if (1..144).contains(&ly) && mode == 0 {
            return;
        }
        bus.tick(1);
    }
    panic!("Visible HBlank not observed");
}

fn wait_for_next_frame(bus: &mut Bus) {
    let start = bus.frame_counter();
    for _ in 0..(154 * 456 * 2) {
        if bus.frame_counter() > start {
            return;
        }
        bus.tick(1);
    }
    panic!("Frame boundary not observed");
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
fn tac_disable_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // selected input bit becomes high
    bus.write_byte(0xFF07, 0x00); // disable timer => falling edge => TIMA++

    assert_eq!(bus.read_byte(0xFF05), 0x01);
}

#[test]
fn tac_frequency_switch_can_increment_tima_on_falling_edge() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF07, 0x05); // TAC: enable + bit3 source
    bus.write_byte(0xFF05, 0x00); // TIMA

    bus.tick(8); // bit3 high while bit5 is still low
    bus.write_byte(0xFF07, 0x06); // switch to bit5 source => falling edge => TIMA++

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
fn serial_stop_cancels_transfer_and_does_not_request_interrupt() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'X');
    bus.write_byte(0xFF02, 0x81);

    bus.tick(255); // before first serial clock falling edge
    bus.write_byte(0xFF02, 0x00); // explicit stop

    for _ in 0..5000 {
        bus.tick(1);
    }

    assert_eq!(bus.interrupt_flags() & (1 << 3), 0);
    assert_eq!(bus.read_byte(0xFF02) & 0x80, 0x00);
    assert!(!bus.serial_output().contains('X'));
}

#[test]
fn serial_restart_uses_latest_tx_byte() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF01, b'A');
    bus.write_byte(0xFF02, 0x81); // start transfer

    bus.tick(200); // transfer still in progress
    bus.write_byte(0xFF01, b'B');
    bus.write_byte(0xFF02, 0x81); // restart transfer

    let mut finished = false;
    for _ in 0..5000 {
        bus.tick(1);
        if (bus.interrupt_flags() & (1 << 3)) != 0 {
            finished = true;
            break;
        }
    }

    assert!(finished, "serial transfer did not complete after restart");
    assert_eq!(bus.serial_output(), "B");
}

#[test]
fn p1_reads_action_buttons_when_button_group_is_selected() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x10); // P15=0 (buttons), P14=1 (dpad not selected)
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);

    bus.set_button_pressed(Button::A, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);

    bus.set_button_pressed(Button::Start, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x06);
}

#[test]
fn p1_reads_dpad_buttons_when_direction_group_is_selected() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF00, 0x20); // P14=0 (dpad), P15=1 (buttons not selected)
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0F);

    bus.set_button_pressed(Button::Right, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0E);

    bus.set_button_pressed(Button::Up, true);
    assert_eq!(bus.read_byte(0xFF00) & 0x0F, 0x0A);
}

#[test]
fn joypad_interrupt_is_requested_on_new_selected_press() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF00, 0x20); // select dpad

    bus.set_button_pressed(Button::Right, true);
    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_interrupt_flags(0x00);
    bus.set_button_pressed(Button::Right, true); // still pressed, no new edge
    assert_eq!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_button_pressed(Button::Right, false);
    assert_eq!(bus.interrupt_flags() & (1 << 4), 0);

    bus.set_button_pressed(Button::Right, true); // new falling edge
    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);
}

#[test]
fn joypad_interrupt_can_be_requested_when_selection_changes() {
    let mut bus = make_test_bus();
    bus.set_interrupt_flags(0x00);

    bus.set_button_pressed(Button::A, true);
    bus.write_byte(0xFF00, 0x10); // select action keys after A is already pressed

    assert_ne!(bus.interrupt_flags() & (1 << 4), 0);
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
fn stat_mode0_irq_retriggers_when_toggled_during_hblank() {
    let mut bus = make_test_bus();

    wait_for_visible_hblank(&mut bus);
    bus.write_byte(0xFF41, 0x00); // disable all STAT sources

    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF41, 0x08); // enable mode 0 source in active HBlank
    assert_ne!(bus.interrupt_flags() & (1 << 1), 0);

    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF41, 0x00); // drop STAT line
    bus.write_byte(0xFF41, 0x08); // raise again in same HBlank
    assert_ne!(bus.interrupt_flags() & (1 << 1), 0);
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
fn framebuffer_renders_bg_tile_colors_with_identity_palette() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off to allow deterministic VRAM setup
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF47, 0xE4); // BGP identity: 0->0, 1->1, 2->2, 3->3

    // Tile map first entry points to tile 0.
    bus.write_byte(0x9800, 0x00);
    // Tile 0, row 0 encodes color ids: 0,1,2,3,0,1,2,3.
    bus.write_byte(0x8000, 0x55); // low plane
    bus.write_byte(0x8001, 0x33); // high plane

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on + unsigned tile data at 0x8000
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let expected = [0xFF, 0xAA, 0x55, 0x00, 0xFF, 0xAA, 0x55, 0x00];
    assert_eq!(&frame[..8], &expected);
}

#[test]
fn framebuffer_applies_scx_scroll_to_bg_sampling() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off to allow deterministic VRAM setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x08); // SCX: shift view by one tile
    bus.write_byte(0xFF47, 0xE4); // BGP identity

    // First tile is white, second tile is black.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x9801, 0x01);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0x00);
}

#[test]
fn framebuffer_window_overrides_bg_where_visible() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // BG tile map (9C00) uses tile 0 (white).
    bus.write_byte(0x9C00, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window tile map (9800) uses tile 1 (black).
    bus.write_byte(0x9800, 0x01);
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0x07); // WX so window starts at x=0

    // LCD on + window enable + BG map 9C00 + BG on + tile data 8000.
    bus.write_byte(0xFF40, 0xB9);
    // First LCD-on frame contains startup quirks; validate steady-state frame.
    wait_for_next_frame(&mut bus);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0x00);
    assert_eq!(frame[8], 0xFF);
}

#[test]
fn framebuffer_sprite_renders_over_bg() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 2 with color id=2 across full row.
    bus.write_byte(0x8020, 0x00);
    bus.write_byte(0x8021, 0xFF);

    // Sprite at top-left visible corner.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 2); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    // LCD on + OBJ enable + BG enable + tile data 8000.
    bus.write_byte(0xFF40, 0x93);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0x55); // OBJ color id=2
}

#[test]
fn framebuffer_sprite_priority_bit_defers_to_non_zero_bg() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // BG tile 0 row starts with color id=1, then zeros.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x80);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 2 with color id=3 at first pixel.
    bus.write_byte(0x8020, 0x80);
    bus.write_byte(0x8021, 0x80);

    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 2); // tile
    bus.write_byte(0xFE03, 0x80); // priority: behind BG

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0xAA); // BG color id=1 should win
}

#[test]
fn framebuffer_sprite_obeys_palette_and_flip_attributes() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0
    bus.write_byte(0xFF49, 0x1B); // inverted mapping for OBP1

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 3:
    // row0 first pixel color id=1, last pixel color id=2.
    bus.write_byte(0x8030, 0x80);
    bus.write_byte(0x8031, 0x01);

    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 3); // tile
    // bit4=palette1, bit5=xflip.
    bus.write_byte(0xFE03, 0x30);

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // xflip makes leftmost pixel use original rightmost color id=2.
    // OBP1=0x1B maps color id=2 to shade 1 => luma 0xAA.
    assert_eq!(frame[0], 0xAA);
}

#[test]
fn framebuffer_limits_visible_sprites_to_ten_per_line() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 4 fully black (color id=3).
    bus.write_byte(0x8040, 0xFF);
    bus.write_byte(0x8041, 0xFF);

    // Place 11 sprites on the same scanline; first 10 should be considered.
    for i in 0..11u16 {
        let base = 0xFE00 + i * 4;
        bus.write_byte(base, 16); // Y
        bus.write_byte(base + 1, 8 + (i as u8) * 8); // X
        bus.write_byte(base + 2, 4); // tile
        bus.write_byte(base + 3, 0); // attrs
    }

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // Pixel x=80 belongs only to the 11th sprite, which should be dropped.
    assert_eq!(frame[80], 0xFF);
}

#[test]
fn framebuffer_sprite_8x16_uses_sequential_tiles() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // 8x16 sprite uses tiles 6 (top) and 7 (bottom).
    // Tile 6 row0 -> color id 0 (white), tile 7 row0 -> color id 3 (black).
    bus.write_byte(0x8060, 0x00);
    bus.write_byte(0x8061, 0x00);
    bus.write_byte(0x8070, 0xFF);
    bus.write_byte(0x8071, 0xFF);

    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 6); // tile (LSB ignored in 8x16 mode)
    bus.write_byte(0xFE03, 0); // attrs

    // LCD on + BG + OBJ + OBJ size 8x16.
    bus.write_byte(0xFF40, 0x97);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0xFF); // top half from tile 6
    assert_eq!(frame[8 * 160], 0x00); // bottom half from tile 7
}

#[test]
fn framebuffer_sprite_priority_prefers_leftmost_x_then_oam_order() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Tile 8 => color id 3 across full row (black).
    bus.write_byte(0x8080, 0xFF);
    bus.write_byte(0x8081, 0xFF);
    // Tile 9 => color id 1 across full row (light gray).
    bus.write_byte(0x8090, 0xFF);
    bus.write_byte(0x8091, 0x00);

    // OAM index 0: right sprite (higher X), black.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 12); // X -> left=4
    bus.write_byte(0xFE02, 8); // tile
    bus.write_byte(0xFE03, 0);

    // OAM index 1: left sprite (lower X), light gray.
    bus.write_byte(0xFE04, 16); // Y
    bus.write_byte(0xFE05, 10); // X -> left=2
    bus.write_byte(0xFE06, 9); // tile
    bus.write_byte(0xFE07, 0);

    // LCD on + BG + OBJ.
    bus.write_byte(0xFF40, 0x93);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // Pixel x=4 is covered by both; lower X sprite should win.
    assert_eq!(frame[4], 0xAA);
}

#[test]
fn framebuffer_sprite_priority_uses_oam_order_when_x_matches() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Tile 10 => color id 3 across full row (black).
    bus.write_byte(0x80A0, 0xFF);
    bus.write_byte(0x80A1, 0xFF);
    // Tile 11 => color id 1 across full row (light gray).
    bus.write_byte(0x80B0, 0xFF);
    bus.write_byte(0x80B1, 0x00);

    // OAM index 0 should have higher priority for equal X.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 12); // X -> left=4
    bus.write_byte(0xFE02, 10); // tile (black)
    bus.write_byte(0xFE03, 0);

    bus.write_byte(0xFE04, 16); // Y
    bus.write_byte(0xFE05, 12); // X -> left=4
    bus.write_byte(0xFE06, 11); // tile (light gray)
    bus.write_byte(0xFE07, 0);

    // LCD on + BG + OBJ.
    bus.write_byte(0xFF40, 0x93);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[4], 0x00);
}

#[test]
fn framebuffer_window_wx_zero_applies_minus_seven_offset() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // BG tile map uses tile 0 (white).
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map first tile is tile 1, second tile is tile 0.
    // Tile 1 row0 has color id 3 only at pixel 7 (rightmost pixel in tile).
    bus.write_byte(0x9800, 0x01);
    bus.write_byte(0x9801, 0x00);
    bus.write_byte(0x8010, 0x01);
    bus.write_byte(0x8011, 0x01);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0x00); // WX=0 => window starts at x=-7

    // LCD on + window enable + BG enable + tile data 8000.
    bus.write_byte(0xFF40, 0xB1);
    // First LCD-on frame contains startup quirks; validate steady-state frame.
    wait_for_next_frame(&mut bus);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // At x=0 we sample window pixel x=7 from first tile (black),
    // then x=1 samples next tile's pixel x=0 (white).
    assert_eq!(frame[0], 0x00);
    assert_eq!(frame[1], 0xFF);
}

#[test]
fn dmg0_boot_profile_uses_expected_div_phase_and_ly_start() {
    let mut rom = vec![0; 32 * 1024];
    rom[0x0147] = 0x00;
    rom[0x0148] = 0x00;
    let cart = Cartridge::from_bytes(rom).expect("test ROM should be valid");
    let bus = Bus::new_with_model(cart, HardwareModel::Dmg0);

    assert_eq!(bus.timer.div_counter, 0x1830);
    assert_eq!(bus.io[0x44], 0x91);
    assert_eq!(bus.ppu.ly_counter, 96);
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
    assert_eq!(bus_a.timer.div_counter, 0xD860);

    // boot_div2-S.gb checksum bytes at 0x014E/0x014F.
    let bus_b = make_bus(0x96, 0xA7);
    assert_eq!(bus_b.timer.div_counter, 0xD850);
}
