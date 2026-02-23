use super::test_utils::{make_test_bus, tick_n};
use super::*;
use crate::hardware::HardwareModel;
use crate::input::Button;

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

fn wait_for_ly_mode(bus: &mut Bus, target_ly: u8, target_mode: u8) {
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == target_ly && mode == target_mode {
            return;
        }
        bus.tick(1);
    }
    panic!("LY={target_ly} mode={target_mode} not observed");
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
fn stat_mode0_enable_during_mode3_triggers_on_hblank_entry() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF41, 0x00); // disable all STAT sources

    // Reach a stable visible Mode 3 period.
    wait_for_transition(&mut bus, 0x42, 0x02, 0x03);
    assert_eq!(bus.read_byte(0xFF44), 0x42);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 0x03);

    bus.set_interrupt_flags(0x00);
    bus.write_byte(0xFF41, 0x08); // enable mode0 source while still in mode3

    wait_for_transition(&mut bus, 0x42, 0x03, 0x00);
    // Source is armed during mode3, so interrupt line raises at HBlank entry.
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
fn oam_dma_restart_keeps_previous_transfer_running_during_full_restart_delay() {
    let mut bus = make_test_bus();

    // Source A pattern.
    bus.write_byte(0x8000, 0xA0);
    bus.write_byte(0x8001, 0xA1);
    bus.write_byte(0x8002, 0xA2);
    // Source B distinct first bytes.
    bus.write_byte(0x8100, 0xB0);
    bus.write_byte(0x8101, 0xB1);

    bus.write_byte(0xFF46, 0x80);
    bus.tick(8); // DMA A starts.
    bus.tick(4); // Copy first byte from A -> OAM[0].
    assert_eq!(bus.read_byte_raw(0xFE00), 0xA0);

    bus.write_byte(0xFF46, 0x81); // request restart to source B
    bus.tick(8); // full restart delay window

    // Previous DMA should keep running during all 8 t-cycles of restart delay.
    assert_eq!(bus.read_byte_raw(0xFE01), 0xA1);
    assert_eq!(bus.read_byte_raw(0xFE02), 0xA2);

    // New DMA should take over after the delay and restart from OAM index 0.
    bus.tick(4);
    assert_eq!(bus.read_byte_raw(0xFE00), 0xB0);
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
fn framebuffer_scx_write_mid_frame_affects_following_lines_only() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX (initial)
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // Tile 0 white, tile 1 black.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x9801, 0x01);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);
    bus.write_byte(0x8014, 0xFF);
    bus.write_byte(0x8015, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on

    let mut reached_ly2_mode2 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 2 {
            reached_ly2_mode2 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode2);

    // Change SCX during frame after lines 0 and 1 are already rendered.
    bus.write_byte(0xFF43, 0x08);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // Line 1 keeps old SCX=0 (white at x=0).
    assert_eq!(frame[160], 0xFF);
    // Line 2 uses new SCX=8 (black at x=0).
    assert_eq!(frame[320], 0x00);
}

#[test]
fn framebuffer_scx_write_during_mode3_affects_remaining_pixels_same_line() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // Fill first BG map row with alternating white/black tiles.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, if (i & 1) == 0 { 0x00 } else { 0x01 });
    }

    // Tile 0 white.
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8004, 0x00);
    bus.write_byte(0x8005, 0x00);

    // Tile 1 black.
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);
    bus.write_byte(0x8014, 0xFF);
    bus.write_byte(0x8015, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on

    // Reach LY=2 mode 3.
    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // Let some pixels of LY=2 render with SCX=0, then shift by one tile.
    bus.tick(20);
    bus.write_byte(0xFF43, 0x08);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    // Early pixel rendered before SCX write: tile 0 (white).
    assert_eq!(frame[line2], 0xFF);
    // Later pixel rendered after SCX write: shifted one tile (white instead of black).
    assert_eq!(frame[line2 + 40], 0xFF);
}

#[test]
fn framebuffer_scx_low_bits_write_during_mode3_does_not_truncate_line_output() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x07); // SCX low-bits penalty latched at mode3 start
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // Fill first BG map row with black tile so rendered pixels are visibly non-white.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);
    bus.write_byte(0x8014, 0xFF);
    bus.write_byte(0x8015, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on

    // Reach LY=2 mode 3 and change SCX low bits to reduce live penalty.
    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    bus.tick(20);
    bus.write_byte(0xFF43, 0x00);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    assert_eq!(frame[line2 + 159], 0x00);
}

#[test]
fn framebuffer_obp_write_during_mode3_affects_later_obj_pixels_only() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8004, 0x00);
    bus.write_byte(0x8005, 0x00);

    // Sprite tile 2 with color id=2 across the row used by LY=2.
    bus.write_byte(0x8024, 0x00);
    bus.write_byte(0x8025, 0xFF);

    // Two sprites on LY=2: one at x=0..7 and one at x=16..23.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 2); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    bus.write_byte(0xFE04, 16); // Y
    bus.write_byte(0xFE05, 24); // X
    bus.write_byte(0xFE06, 2); // tile
    bus.write_byte(0xFE07, 0x00); // attrs

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ

    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // Render first sprite with original OBP0, then switch palette before second sprite.
    bus.tick(34);
    bus.write_byte(0xFF48, 0x00); // color 2 -> shade 0 (white)
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    assert_eq!(frame[line2], 0x55); // first sprite kept old OBP0 mapping
    assert_eq!(frame[line2 + 16], 0xFF); // second sprite used updated OBP0 mapping
}

#[test]
fn mode3_obj_fetch_stall_delays_scx_write_effect_on_first_visible_pixel() {
    fn render_line2_prefix_with_optional_hidden_obj_stall(
        add_hidden_obj: bool,
        ticks_before_scx_write: usize,
    ) -> [u8; 24] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, 0x00); // SCX
        bus.write_byte(0xFF47, 0xE4); // identity palette

        // Tile 0 white, tile 1 black.
        bus.write_byte(0x9800, 0x00);
        bus.write_byte(0x9801, 0x01);
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);
        bus.write_byte(0x8004, 0x00);
        bus.write_byte(0x8005, 0x00);
        bus.write_byte(0x8010, 0xFF);
        bus.write_byte(0x8011, 0xFF);
        bus.write_byte(0x8014, 0xFF);
        bus.write_byte(0x8015, 0xFF);

        if add_hidden_obj {
            // Hidden X=0 object on LY=2. It should still consume fetch dots in mode 3.
            bus.write_byte(0xFE00, 18); // Y => top at LY=2
            bus.write_byte(0xFE01, 0); // X hidden/off-screen on DMG
            bus.write_byte(0xFE02, 0x00); // tile
            bus.write_byte(0xFE03, 0x00); // attrs
        }

        bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ

        let mut reached_ly2_mode3 = false;
        for _ in 0..(154 * 456 * 2) {
            let ly = bus.read_byte(0xFF44);
            let mode = bus.read_byte(0xFF41) & 0x03;
            if ly == 2 && mode == 3 {
                reached_ly2_mode3 = true;
                break;
            }
            bus.tick(1);
        }
        assert!(reached_ly2_mode3);

        // SCX write is early enough to race with first visible output.
        bus.tick(ticks_before_scx_write as u8);
        bus.write_byte(0xFF43, 0x08);
        wait_for_next_frame(&mut bus);

        let frame = bus.framebuffer();
        let mut prefix = [0u8; 24];
        prefix.copy_from_slice(&frame[2 * 160..(2 * 160 + 24)]);
        prefix
    }

    let mut observed_window = None;
    for ticks in 0..96usize {
        let no_obj_stall = render_line2_prefix_with_optional_hidden_obj_stall(false, ticks);
        let hidden_obj_stall = render_line2_prefix_with_optional_hidden_obj_stall(true, ticks);
        if no_obj_stall != hidden_obj_stall {
            let diff_index = no_obj_stall
                .iter()
                .zip(hidden_obj_stall.iter())
                .position(|(a, b)| a != b)
                .expect("prefix differs");
            observed_window = Some((
                ticks,
                diff_index,
                no_obj_stall[diff_index],
                hidden_obj_stall[diff_index],
            ));
            break;
        }
    }

    let (ticks, diff_index, no_obj_stall, hidden_obj_stall) = observed_window
        .expect("expected a timing window where OBJ fetch stall changes SCX write effect");

    assert!(
        (no_obj_stall == 0xFF && hidden_obj_stall == 0x00)
            || (no_obj_stall == 0x00 && hidden_obj_stall == 0xFF),
        "unexpected SCX/stall edge colors at ticks={ticks}, x={diff_index}: no_obj={no_obj_stall:#04X}, hidden_obj={hidden_obj_stall:#04X}"
    );
    assert!(diff_index < 24);
}

fn line2_hblank_delay_with_obj_toggle(
    obj_enabled_at_mode3_start: bool,
    toggle_after_mode3_ticks: Option<(usize, bool)>,
) -> u16 {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // Keep BG output deterministic.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Hidden X=0 sprite on LY=2 still consumes OBJ fetch dots.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 0); // X hidden/off-screen
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    let mut lcdc = 0x91u8; // LCD on + BG on
    if obj_enabled_at_mode3_start {
        lcdc |= 0x02;
    }
    bus.write_byte(0xFF40, lcdc);

    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    if let Some((ticks, enable_obj)) = toggle_after_mode3_ticks {
        if ticks > 0 {
            bus.tick(ticks as u8);
        }
        let mut next_lcdc = bus.read_byte(0xFF40);
        if enable_obj {
            next_lcdc |= 0x02;
        } else {
            next_lcdc &= !0x02;
        }
        bus.write_byte(0xFF40, next_lcdc);
    }

    wait_for_transition(&mut bus, 2, 3, 0);
    measure_hblank_until_ly_increment(&mut bus, 2)
}

fn setup_line2_mode3_bus_with_hidden_obj(obj_enabled: bool) -> Bus {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0x8000, 0x12); // VRAM probe byte

    // Hidden X=0 sprite on LY=2 still consumes OBJ fetch dots when OBJ is enabled.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 0); // X hidden/off-screen
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    let mut lcdc = 0x91; // LCD on + BG on
    if obj_enabled {
        lcdc |= 0x02;
    }
    bus.write_byte(0xFF40, lcdc);
    wait_for_ly_mode(&mut bus, 2, 3);
    bus
}

fn ticks_until_mode3_exit_on_current_line(bus: &mut Bus) -> u16 {
    let target_ly = bus.read_byte(0xFF44);
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 3);

    let mut ticks = 0u16;
    for _ in 0..512 {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == target_ly && mode != 3 {
            return ticks;
        }
        bus.tick(1);
        ticks = ticks.saturating_add(1);
    }
    panic!("mode3 exit not observed on LY={target_ly}");
}

fn ticks_until_stat_irq(bus: &mut Bus) -> u16 {
    let mut ticks = 0u16;
    for _ in 0..512 {
        if (bus.interrupt_flags() & (1 << 1)) != 0 {
            return ticks;
        }
        bus.tick(1);
        ticks = ticks.saturating_add(1);
    }
    panic!("STAT IRQ not observed within expected window");
}

#[test]
fn mode3_enabling_obj_mid_line_shortens_hblank_via_runtime_contention() {
    let delay_no_obj = line2_hblank_delay_with_obj_toggle(false, None);
    let delay_enable_midline = line2_hblank_delay_with_obj_toggle(false, Some((4, true)));

    assert!(
        delay_enable_midline < delay_no_obj,
        "expected OBJ enable mid-line to consume mode3 dots (no_obj={delay_no_obj}, enable_midline={delay_enable_midline})"
    );
}

#[test]
fn mode3_disabling_obj_mid_line_restores_hblank_budget() {
    let delay_obj_full_line = line2_hblank_delay_with_obj_toggle(true, None);
    let delay_disable_midline = line2_hblank_delay_with_obj_toggle(true, Some((0, false)));

    assert!(
        delay_disable_midline > delay_obj_full_line,
        "expected OBJ disable mid-line to recover mode3 dots (full_obj={delay_obj_full_line}, disable_midline={delay_disable_midline})"
    );
}

#[test]
fn mode3_bg_fetch_pipeline_pushes_first_tile_after_six_dots() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on

    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // First Mode 3 render dot starts tile-index phase (2 dots).
    bus.tick(1);
    assert_eq!(bus.mode3_bg_fifo_len(), 0);
    assert_eq!(bus.mode3_bg_fetch_phase(), 0);
    assert_eq!(bus.mode3_bg_fetch_dots_remaining(), 1);

    // Dot 2 finishes tile-index and enters low-byte phase.
    bus.tick(1);
    assert_eq!(bus.mode3_bg_fifo_len(), 0);
    assert_eq!(bus.mode3_bg_fetch_phase(), 1);
    assert_eq!(bus.mode3_bg_fetch_dots_remaining(), 2);

    // Dot 4 finishes low-byte and enters high-byte phase.
    bus.tick(1);
    bus.tick(1);
    assert_eq!(bus.mode3_bg_fifo_len(), 0);
    assert_eq!(bus.mode3_bg_fetch_phase(), 2);
    assert_eq!(bus.mode3_bg_fetch_dots_remaining(), 2);

    // Dot 6 finishes high-byte and pushes 8 pixels in push phase.
    bus.tick(1);
    bus.tick(1);
    assert_eq!(bus.mode3_bg_fifo_len(), 8);
    assert_eq!(bus.mode3_bg_fetch_phase(), 0);
    assert_eq!(bus.mode3_bg_fetch_dots_remaining(), 0);
}

#[test]
fn mode3_normal_bg_push_path_does_not_expose_push_ready_takeover_boundary() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on
    wait_for_ly_mode(&mut bus, 2, 3);

    // First 6 Mode3 dots are a non-stalled BG fetch/push block. The current model
    // collapses the normal Push path into the same dot as TileDataHigh completion,
    // so the post-recovery Push-ready takeover boundary must not appear here.
    for dot in 1..=6 {
        assert!(
            !bus.mode3_bg_push_ready_takeover_boundary(),
            "normal first-tile path must not classify a push-ready takeover boundary before dot {dot}"
        );
        assert!(
            !bus.mode3_bg_push_recovery_sleep_pending(),
            "normal first-tile path must not enter recovery-sleep before dot {dot}"
        );
        bus.tick(1);
    }

    assert_eq!(bus.mode3_bg_fifo_len(), 8);
    assert_eq!(bus.mode3_bg_fetch_phase(), 0);
    assert!(
        !bus.mode3_bg_push_ready_takeover_boundary(),
        "normal first-tile push should return to TileIndex without an observable push-ready boundary"
    );
}

#[test]
fn mode3_obj_fetch_waits_for_bg_fetch_boundary_before_starting() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX

    // Hidden X=0 sprite on LY=2 should request OBJ fetch immediately after mode3 start.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 0); // X hidden/off-screen
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ

    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // While BG fetch block is in progress, OBJ fetch should not start yet.
    for _ in 0..6 {
        bus.tick(1);
        assert_eq!(bus.mode3_obj_fetch_dots_remaining(), 0);
        assert_eq!(bus.mode3_obj_next_sprite_index(), 0);
    }

    // After BG block boundary, OBJ fetch begins.
    bus.tick(1);
    assert!(bus.mode3_obj_fetch_dots_remaining() > 0);
    assert_eq!(bus.mode3_obj_next_sprite_index(), 1);
}

#[test]
fn mode3_obj_fetch_can_start_from_fifo_stall_boundary() {
    let mut bus = setup_line2_mode3_bus_with_hidden_obj(false);

    let mut reached_fifo_stall_boundary = false;
    for _ in 0..128 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3
            && bus.mode3_obj_next_sprite_index() == 0
            && bus.mode3_bg_push_stalled_for_fifo()
        {
            reached_fifo_stall_boundary = true;
            break;
        }
    }
    assert!(
        reached_fifo_stall_boundary,
        "expected a FIFO stall boundary before enabling OBJ"
    );

    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x02); // enable OBJ mid-line
    bus.tick(1);
    assert_eq!(
        bus.mode3_obj_next_sprite_index(),
        1,
        "expected OBJ fetch handover to start from stalled Push boundary on the next dot"
    );
}

#[test]
fn mode3_window_trigger_can_restart_from_fifo_stall_boundary() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_fifo_stall_boundary = false;
    for _ in 0..128 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3
            && bus.mode3_output_x() > 0
            && bus.mode3_bg_push_stalled_for_fifo()
        {
            reached_fifo_stall_boundary = true;
            break;
        }
    }
    assert!(
        reached_fifo_stall_boundary,
        "expected FIFO stall boundary before arming window trigger"
    );

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    let mut window_triggered = false;
    for _ in 0..2 {
        bus.tick(1);
        if bus.mode3_window_triggered_this_line() {
            window_triggered = true;
            break;
        }
    }
    assert!(
        window_triggered,
        "expected window to restart promptly from stalled Push boundary arbitration (output_x={output_x}, wx={wx})"
    );
    assert!(!bus.mode3_window_trigger_pending());
}

#[test]
fn mode3_fifo_stall_window_recovery_sleep_delays_stat_mode0_and_bus_release() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_fifo_stall_boundary = false;
    for _ in 0..128 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3
            && bus.mode3_output_x() > 0
            && bus.mode3_bg_push_stalled_for_fifo()
        {
            reached_fifo_stall_boundary = true;
            break;
        }
    }
    assert!(reached_fifo_stall_boundary);

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    assert_eq!(bus.interrupt_flags() & (1 << 1), 0);
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    bus.tick(1);
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "expected mode3 to remain active during stalled-push recovery"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "expected STAT mode0 IRQ to remain pending until the delayed HBlank boundary"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );
}

#[test]
fn mode3_bg_push_stall_recovery_consumes_sleep_dot_before_push() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_push_stall = false;
    for _ in 0..128 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3
            && bus.mode3_output_x() > 0
            && bus.mode3_bg_push_stalled_for_fifo()
        {
            reached_push_stall = true;
            break;
        }
    }
    assert!(
        reached_push_stall,
        "expected to observe a BG push stall in mode3"
    );

    // The fetcher remains stalled in Push while FIFO drains, then latches an
    // explicit RecoverySleep substate for the next dot.
    let mut observed_recovery_latch = false;
    for _ in 0..8 {
        bus.tick(1);
        assert_eq!(bus.mode3_bg_fetch_phase(), 3);
        if bus.mode3_bg_push_recovery_sleep_pending() {
            observed_recovery_latch = true;
            break;
        }
    }
    assert!(
        observed_recovery_latch,
        "expected to observe explicit Push recovery-sleep substate after FIFO drain"
    );

    bus.tick(1);
    assert_eq!(
        bus.mode3_bg_fetch_phase(),
        3,
        "expected push-ready dot after RecoverySleep before Push completes"
    );
    assert!(bus.mode3_bg_push_ready_takeover_boundary());

    bus.tick(1);
    assert_eq!(bus.mode3_bg_fetch_phase(), 0);
}

#[test]
fn mode3_bg_push_substate_sequence_latches_recovery_sleep_before_push_ready() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut saw_stalled_push = false;
    for _ in 0..128 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3
            && bus.mode3_output_x() > 0
            && bus.mode3_bg_push_stalled_for_fifo()
        {
            saw_stalled_push = true;
            break;
        }
    }
    assert!(saw_stalled_push, "expected a stalled Push substate");
    assert!(!bus.mode3_bg_push_recovery_sleep_pending());
    assert!(!bus.mode3_bg_push_ready_takeover_boundary());

    let mut saw_recovery_sleep = false;
    for _ in 0..8 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3 && bus.mode3_bg_push_recovery_sleep_pending() {
            saw_recovery_sleep = true;
            break;
        }
    }
    assert!(
        saw_recovery_sleep,
        "expected explicit Push recovery-sleep substate"
    );
    assert!(!bus.mode3_bg_push_stalled_for_fifo());
    assert!(
        !bus.mode3_bg_push_ready_takeover_boundary(),
        "recovery-sleep dot must not be classified as push-ready boundary"
    );

    bus.tick(1);
    assert_eq!(
        bus.mode3_bg_fetch_phase(),
        3,
        "expected to stay in Push for push-ready dot"
    );
    assert!(!bus.mode3_bg_push_stalled_for_fifo());
    assert!(!bus.mode3_bg_push_recovery_sleep_pending());
    assert!(
        bus.mode3_bg_push_ready_takeover_boundary(),
        "expected push-ready boundary exactly one dot after recovery sleep"
    );

    bus.tick(1);
    assert_eq!(
        bus.mode3_bg_fetch_phase(),
        0,
        "expected Push to complete into TileIndex"
    );
    assert!(!bus.mode3_bg_push_recovery_sleep_pending());
}

#[test]
fn mode3_window_trigger_does_not_restart_on_fifo_recovery_sleep_dot() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_sleep_dot = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_push_recovery_sleep_pending() && bus.mode3_output_x() > 0 {
            reached_sleep_dot = true;
            break;
        }
    }
    assert!(
        reached_sleep_dot,
        "expected to observe Push recovery sleep dot"
    );
    assert!(!bus.mode3_window_takeover_boundary());

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line on sleep dot

    bus.tick(1);
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "window restart must stay queued on FIFO recovery sleep dot"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 3);
    assert_eq!(bus.interrupt_flags() & (1 << 1), 0);
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );

    let mut triggered = false;
    for _ in 0..4 {
        bus.tick(1);
        if bus.mode3_window_triggered_this_line() {
            triggered = true;
            break;
        }
    }
    assert!(
        triggered,
        "queued window restart should fire on a later valid Push boundary after sleep"
    );
}

#[test]
fn mode3_obj_fetch_does_not_start_on_fifo_recovery_sleep_dot() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // Sprite positioned so it becomes eligible mid-line.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, OBJ disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_sleep_with_obj_eligible = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_push_recovery_sleep_pending()
            && bus.mode3_obj_next_sprite_index() == 0
            && bus.mode3_output_x() >= 18
        {
            reached_sleep_with_obj_eligible = true;
            break;
        }
    }
    assert!(
        reached_sleep_with_obj_eligible,
        "expected FIFO recovery sleep dot with OBJ eligible soon after"
    );

    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x02); // enable OBJ on sleep dot
    bus.tick(1);
    assert_eq!(
        bus.mode3_obj_next_sprite_index(),
        0,
        "OBJ fetch must not start on FIFO recovery sleep dot"
    );
    assert_eq!(bus.mode3_obj_fetch_dots_remaining(), 0);

    let mut started = false;
    for _ in 0..4 {
        bus.tick(1);
        if bus.mode3_obj_next_sprite_index() == 1 || bus.mode3_obj_fetch_dots_remaining() > 0 {
            started = true;
            break;
        }
    }
    assert!(
        started,
        "OBJ fetch should start on a later valid boundary after recovery sleep"
    );
}

#[test]
fn mode3_window_trigger_restarts_on_push_ready_boundary_after_recovery_sleep() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_sleep_dot = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_push_recovery_sleep_pending() && bus.mode3_output_x() > 0 {
            reached_sleep_dot = true;
            break;
        }
    }
    assert!(reached_sleep_dot);
    assert!(!bus.mode3_bg_push_ready_takeover_boundary());

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window on sleep dot

    // Next dot is the recovery sleep dot: no takeover yet.
    bus.tick(1);
    assert!(!bus.mode3_window_triggered_this_line());
    assert!(bus.mode3_window_trigger_pending());
    assert!(bus.mode3_bg_push_ready_takeover_boundary());

    // Following dot is the Push-ready takeover boundary after recovery sleep.
    bus.tick(1);
    assert!(
        bus.mode3_window_triggered_this_line(),
        "expected window restart exactly on push-ready boundary after recovery sleep"
    );
    assert!(!bus.mode3_window_trigger_pending());
}

#[test]
fn mode3_push_ready_boundary_then_hblank_entry_with_dma_splits_vram_oam_release_and_stat_irq() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte + DMA source byte
    bus.write_byte(0x8001, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    // Keep DMA active across the later Mode3->HBlank release edge.
    bus.write_byte(0xFF46, 0x80);
    bus.tick(8);
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "DMA should block OAM after start delay"
    );

    let mut reached_sleep_dot = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_push_recovery_sleep_pending() && bus.mode3_output_x() > 0 {
            reached_sleep_dot = true;
            break;
        }
    }
    assert!(reached_sleep_dot);
    assert!(!bus.mode3_bg_push_ready_takeover_boundary());

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window on sleep dot

    // Recovery sleep dot: no takeover yet, mode3 + bus blocking remain.
    bus.tick(1);
    assert!(!bus.mode3_window_triggered_this_line());
    assert!(bus.mode3_window_trigger_pending());
    assert!(bus.mode3_bg_push_ready_takeover_boundary());
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active on recovery sleep dot"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should stay clear before push-ready boundary"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3 on recovery sleep dot"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3 on recovery sleep dot"
    );

    // Push-ready takeover boundary dot: window restart happens but mode3 is still active.
    bus.tick(1);
    assert!(
        bus.mode3_window_triggered_this_line(),
        "expected window restart exactly on push-ready boundary after recovery sleep"
    );
    assert!(!bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active on push-ready takeover dot"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should stay clear before HBlank entry"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked on the mode3 push-ready takeover dot"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked on the mode3 push-ready takeover dot"
    );

    // On HBlank entry, STAT mode changes immediately and VRAM releases, but OAM stays blocked
    // while DMA is still active.
    wait_for_transition(&mut bus, 2, 3, 0);
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        0,
        "expected HBlank at mode3 release"
    );
    assert_ne!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should raise exactly on HBlank entry"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0x12,
        "VRAM should be readable again in HBlank even while DMA remains active"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should stay blocked in HBlank while DMA remains active"
    );
}

#[test]
fn mode3_push_ready_boundary_window_and_obj_same_dot_prefers_obj_then_queues_window() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // Sprite later on the line so it becomes eligible during visible output.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte
    bus.write_byte(0x8001, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, OBJ/window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_sleep_with_obj_soon = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_push_recovery_sleep_pending() && bus.mode3_output_x() >= 18 {
            reached_sleep_with_obj_soon = true;
            break;
        }
    }
    assert!(reached_sleep_with_obj_soon);
    assert!(!bus.mode3_window_takeover_boundary());

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x22); // enable OBJ + window on sleep dot

    // Next dot is the recovery sleep dot: shared takeover must still not happen.
    bus.tick(1);
    assert!(!bus.mode3_window_triggered_this_line());
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(bus.mode3_obj_next_sprite_index(), 0);
    assert_eq!(bus.mode3_obj_fetch_dots_remaining(), 0);
    assert!(bus.mode3_bg_push_ready_takeover_boundary());
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 3);
    assert_eq!(bus.interrupt_flags() & (1 << 1), 0);
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );

    // Following dot is the Push-ready boundary: OBJ should win, window should queue.
    bus.tick(1);
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "window restart should remain queued while OBJ wins shared push-ready boundary"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.mode3_obj_next_sprite_index(),
        1,
        "expected OBJ fetch to start on shared push-ready boundary"
    );
    assert_eq!(bus.read_byte(0xFF41) & 0x03, 3);
    assert_eq!(bus.interrupt_flags() & (1 << 1), 0);
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );
}

#[test]
fn mode3_window_trigger_queues_until_obj_fetch_finishes() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // Sprite later on the line to keep OBJ fetch active while output_x > 0.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24 (session 1)
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs
    bus.write_byte(0xFE04, 18); // Y => top at LY=2
    bus.write_byte(0xFE05, 80); // X => separated session 2 (forces shutdown penalty after session 1)
    bus.write_byte(0xFE06, 0x00); // tile
    bus.write_byte(0xFE07, 0x00); // attrs

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut obj_fetch_active = false;
    for _ in 0..192 {
        bus.tick(1);
        if bus.mode3_obj_fetch_dots_remaining() >= 2 {
            obj_fetch_active = true;
            break;
        }
    }
    assert!(
        obj_fetch_active,
        "expected active OBJ fetch window on LY=2 before arming window trigger"
    );

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line
    bus.tick(1);

    assert!(!bus.mode3_window_triggered_this_line());
    assert!(bus.mode3_window_trigger_pending());
    assert!(!bus.mode3_window_takeover_boundary());

    let mut triggered = false;
    for _ in 0..64 {
        bus.tick(1);
        if bus.mode3_window_triggered_this_line() {
            triggered = true;
            break;
        }
    }
    assert!(
        triggered,
        "expected queued window trigger to fire after OBJ fetch releases takeover boundary"
    );
}

#[test]
fn mode3_window_trigger_defers_to_obj_on_shared_takeover_boundary() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // Sprite positioned so its fetch becomes eligible mid-line.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs
    bus.write_byte(0xFE04, 18); // Y => top at LY=2
    bus.write_byte(0xFE05, 80); // X => second separated session (enables shutdown penalty after session 1)
    bus.write_byte(0xFE06, 0x00); // tile
    bus.write_byte(0xFE07, 0x00); // attrs
    bus.write_byte(0xFE04, 18); // Y => top at LY=2
    bus.write_byte(0xFE05, 80); // X => separated second session, forces shutdown penalty after session 1
    bus.write_byte(0xFE06, 0x00); // tile
    bus.write_byte(0xFE07, 0x00); // attrs

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_shared_boundary = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_window_takeover_boundary()
            && bus.mode3_obj_next_sprite_index() == 0
            && bus.mode3_obj_fetch_dots_remaining() == 0
            && bus.mode3_output_x() >= 16
        {
            reached_shared_boundary = true;
            break;
        }
    }
    assert!(
        reached_shared_boundary,
        "expected shared takeover boundary where window trigger and OBJ fetch can compete"
    );

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    bus.tick(1);
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "expected OBJ to win shared takeover boundary before queued window restart"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.mode3_obj_next_sprite_index(),
        1,
        "expected OBJ fetch to start on shared takeover boundary"
    );

    let mut triggered = false;
    for _ in 0..64 {
        bus.tick(1);
        if bus.mode3_window_triggered_this_line() {
            triggered = true;
            break;
        }
    }
    assert!(
        triggered,
        "expected queued window trigger to fire after OBJ fetch/shutdown releases boundary"
    );
}

#[test]
fn mode3_shared_window_obj_boundary_keeps_stat_and_bus_blocked_when_obj_wins() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // Sprite positioned so its fetch becomes eligible mid-line.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_shared_boundary = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_window_takeover_boundary()
            && bus.mode3_obj_next_sprite_index() == 0
            && bus.mode3_obj_fetch_dots_remaining() == 0
            && bus.mode3_output_x() >= 16
        {
            reached_shared_boundary = true;
            break;
        }
    }
    assert!(reached_shared_boundary);

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    bus.tick(1);
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "expected window restart to remain queued while OBJ wins shared boundary"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should stay clear while mode3 remains active"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );
}

#[test]
fn mode3_window_trigger_restarts_on_tileindex_takeover_boundary() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_tileindex_boundary = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 0
            && bus.mode3_bg_fetch_dots_remaining() == 0
            && bus.mode3_bg_takeover_boundary()
            && !bus.mode3_bg_push_stalled_for_fifo()
            && !bus.mode3_bg_push_recovery_sleep_pending()
            && !bus.mode3_bg_push_ready_takeover_boundary()
            && bus.mode3_output_x() > 0
        {
            reached_tileindex_boundary = true;
            break;
        }
    }
    assert!(
        reached_tileindex_boundary,
        "expected a normal TileIndex takeover boundary on a visible mode3 dot"
    );

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    bus.tick(1);
    assert!(
        bus.mode3_window_triggered_this_line(),
        "window restart should trigger on TileIndex takeover boundary"
    );
    assert!(!bus.mode3_window_trigger_pending());
}

#[test]
fn mode3_tileindex_boundary_window_restart_keeps_stat_and_bus_blocked() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte
    bus.write_byte(0x8001, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_tileindex_boundary = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 0
            && bus.mode3_bg_fetch_dots_remaining() == 0
            && bus.mode3_bg_takeover_boundary()
            && !bus.mode3_bg_push_stalled_for_fifo()
            && !bus.mode3_bg_push_recovery_sleep_pending()
            && !bus.mode3_bg_push_ready_takeover_boundary()
            && bus.mode3_output_x() > 0
        {
            reached_tileindex_boundary = true;
            break;
        }
    }
    assert!(reached_tileindex_boundary);

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    bus.tick(1);
    assert!(
        bus.mode3_window_triggered_this_line(),
        "window restart should trigger on TileIndex takeover boundary"
    );
    assert!(!bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should stay clear while mode3 remains active"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3 on the window-restart dot"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3 on the window-restart dot"
    );
}

#[test]
fn mode3_tileindex_boundary_then_hblank_entry_with_dma_splits_vram_oam_release_and_stat_irq() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte + DMA source byte
    bus.write_byte(0x8001, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    // Keep DMA active across the later Mode3->HBlank release edge.
    bus.write_byte(0xFF46, 0x80);
    bus.tick(8);
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "DMA should block OAM after start delay"
    );

    let mut reached_tileindex_boundary = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 0
            && bus.mode3_bg_fetch_dots_remaining() == 0
            && bus.mode3_bg_takeover_boundary()
            && !bus.mode3_bg_push_stalled_for_fifo()
            && !bus.mode3_bg_push_recovery_sleep_pending()
            && !bus.mode3_bg_push_ready_takeover_boundary()
            && bus.mode3_output_x() > 0
        {
            reached_tileindex_boundary = true;
            break;
        }
    }
    assert!(reached_tileindex_boundary);

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    // TileIndex takeover boundary dot: window restart happens but mode3 is still active.
    bus.tick(1);
    assert!(
        bus.mode3_window_triggered_this_line(),
        "window restart should trigger on TileIndex takeover boundary"
    );
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should stay clear before HBlank entry"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked on the mode3 takeover dot"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked on the mode3 takeover dot"
    );

    // On HBlank entry, STAT mode changes immediately and VRAM releases, but OAM stays blocked
    // while DMA is still active.
    wait_for_transition(&mut bus, 2, 3, 0);
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        0,
        "expected HBlank at mode3 release"
    );
    assert_ne!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should raise exactly on HBlank entry"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0x12,
        "VRAM should be readable again in HBlank even while DMA remains active"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should stay blocked in HBlank while DMA remains active"
    );
}

#[test]
fn mode3_tileindex_boundary_window_beats_non_eligible_obj() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // Sprite is visible on the line but far enough to be ineligible at the
    // early TileIndex takeover boundary used by this test.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 160); // X => left edge at x=152
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte
    bus.write_byte(0x8001, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, OBJ/window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_tileindex_boundary = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 0
            && bus.mode3_bg_fetch_dots_remaining() == 0
            && bus.mode3_bg_takeover_boundary()
            && !bus.mode3_bg_push_stalled_for_fifo()
            && !bus.mode3_bg_push_recovery_sleep_pending()
            && !bus.mode3_bg_push_ready_takeover_boundary()
            && bus.mode3_output_x() > 0
        {
            reached_tileindex_boundary = true;
            break;
        }
    }
    assert!(reached_tileindex_boundary);

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x22); // enable OBJ + window mid-line

    bus.tick(1);
    assert!(
        bus.mode3_window_triggered_this_line(),
        "window should restart when OBJ is enabled but not yet eligible on TileIndex boundary"
    );
    assert!(!bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.mode3_obj_next_sprite_index(),
        0,
        "OBJ fetch should not start when next sprite is not eligible yet"
    );
    assert_eq!(bus.mode3_obj_fetch_dots_remaining(), 0);
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should stay clear while mode3 remains active"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );
}

#[test]
fn mode3_stalled_push_boundary_window_and_obj_same_dot_prefers_obj_then_queues_window() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // Sprite later on the line so it becomes eligible after output has started.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, OBJ/window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_stalled_push_with_obj_eligible = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3
            && bus.mode3_bg_push_stalled_for_fifo()
            && !bus.mode3_bg_push_recovery_sleep_pending()
            && bus.mode3_output_x() >= 24
        {
            reached_stalled_push_with_obj_eligible = true;
            break;
        }
    }
    assert!(
        reached_stalled_push_with_obj_eligible,
        "expected stalled Push takeover boundary with OBJ becoming eligible"
    );

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x22); // enable OBJ + window mid-line

    bus.tick(1);
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "expected window restart to defer when OBJ is armed on the same stalled Push boundary"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.mode3_obj_next_sprite_index(),
        1,
        "expected OBJ fetch to win the shared stalled Push boundary"
    );

    let mut window_triggered = false;
    for _ in 0..64 {
        bus.tick(1);
        if bus.mode3_window_triggered_this_line() {
            window_triggered = true;
            break;
        }
    }
    assert!(
        window_triggered,
        "expected queued window restart after OBJ fetch/shutdown releases shared stalled boundary"
    );
}

#[test]
fn mode3_stalled_push_boundary_window_and_obj_same_dot_keeps_stat_and_bus_blocked() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // Sprite later on the line so it becomes eligible after output has started.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on, OBJ/window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_stalled_push_with_obj_eligible = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3
            && bus.mode3_bg_push_stalled_for_fifo()
            && !bus.mode3_bg_push_recovery_sleep_pending()
            && bus.mode3_output_x() >= 24
        {
            reached_stalled_push_with_obj_eligible = true;
            break;
        }
    }
    assert!(reached_stalled_push_with_obj_eligible);

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x22); // enable OBJ + window mid-line

    bus.tick(1);
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "expected queued window restart while OBJ wins shared stalled Push boundary"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ should remain clear while mode3 is still active"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );
}

#[test]
fn mode3_window_trigger_stays_queued_through_obj_shutdown_boundary() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => left edge at x=24
    bus.write_byte(0xFE02, 0x00); // tile
    bus.write_byte(0xFE03, 0x00); // attrs
    bus.write_byte(0xFE04, 18); // Y => top at LY=2
    bus.write_byte(0xFE05, 80); // X => second separated session (forces shutdown penalty after session 1)
    bus.write_byte(0xFE06, 0x00); // tile
    bus.write_byte(0xFE07, 0x00); // attrs

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);
    let mut reached_obj_shutdown = false;
    for _ in 0..512 {
        bus.tick(1);
        if bus.mode3_obj_shutdown_dots_remaining() > 0 && bus.mode3_output_x() > 0 {
            reached_obj_shutdown = true;
            break;
        }
    }
    assert!(reached_obj_shutdown);
    assert!(!bus.mode3_window_takeover_boundary());

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    let mut saw_shutdown_release = false;
    for _ in 0..8 {
        bus.tick(1);
        if bus.mode3_obj_shutdown_dots_remaining() > 0 {
            assert!(
                !bus.mode3_window_triggered_this_line(),
                "window restart must remain queued while OBJ shutdown still owns the boundary"
            );
            assert!(bus.mode3_window_trigger_pending());
            continue;
        }
        saw_shutdown_release = true;
        break;
    }
    assert!(
        saw_shutdown_release,
        "expected OBJ shutdown to finish shortly"
    );
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "window should still wait for a valid takeover boundary after OBJ shutdown release"
    );
    assert!(bus.mode3_window_trigger_pending());

    let mut triggered = false;
    for _ in 0..64 {
        bus.tick(1);
        if bus.mode3_window_triggered_this_line() {
            triggered = true;
            break;
        }
    }
    assert!(triggered);
}

#[test]
fn mode3_push_sleep_edge_does_not_raise_stat_mode0_before_hblank() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x00); // mode0 source disabled initially
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);
    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x91); // LCD on + BG on
    wait_for_ly_mode(&mut bus, 2, 3);
    let target_ly = bus.read_byte(0xFF44);

    let mut reached_push_sleep = false;
    for _ in 0..256 {
        bus.tick(1);
        if bus.mode3_bg_fetch_phase() == 3 && bus.mode3_bg_push_stalled_for_fifo() {
            for _ in 0..8 {
                bus.tick(1);
                if bus.mode3_bg_fetch_phase() == 3 && !bus.mode3_bg_push_stalled_for_fifo() {
                    reached_push_sleep = true;
                    break;
                }
            }
            break;
        }
    }
    assert!(reached_push_sleep);
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "expected mode3 on Push sleep dot"
    );

    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF41, 0x08); // arm mode0 source while still in mode3 Push sleep
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ must not raise before actual HBlank entry"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );

    let mut saw_hblank_and_irq = false;
    for _ in 0..512 {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == target_ly && mode == 0 {
            saw_hblank_and_irq = (bus.interrupt_flags() & (1 << 1)) != 0;
            break;
        }
        bus.tick(1);
    }
    assert!(
        saw_hblank_and_irq,
        "expected STAT mode0 IRQ only when HBlank starts after Push sleep edge"
    );
}

#[test]
fn mode3_window_trigger_stays_queued_when_obj_fetch_transitions_into_shutdown() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // Two separated sprite sessions so session 1 ends with shutdown dots.
    bus.write_byte(0xFE00, 18); // Y => top at LY=2
    bus.write_byte(0xFE01, 32); // X => session 1
    bus.write_byte(0xFE02, 0x00);
    bus.write_byte(0xFE03, 0x00);
    bus.write_byte(0xFE04, 18); // Y => top at LY=2
    bus.write_byte(0xFE05, 80); // X => session 2, separated
    bus.write_byte(0xFE06, 0x00);
    bus.write_byte(0xFE07, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x12); // VRAM probe byte
    bus.write_byte(0x8001, 0x00);
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut reached_last_fetch_dot = false;
    for _ in 0..512 {
        bus.tick(1);
        if bus.mode3_obj_fetch_dots_remaining() == 1 && bus.mode3_output_x() > 0 {
            reached_last_fetch_dot = true;
            break;
        }
    }
    assert!(
        reached_last_fetch_dot,
        "expected to observe last OBJ fetch dot before shutdown transition"
    );

    let output_x = bus.mode3_output_x();
    let wx = output_x.saturating_add(7).clamp(8, 166);
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));
    bus.write_byte(0xFF4B, wx);
    bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line

    // This dot transitions OBJ fetch -> OBJ shutdown. Window must stay queued.
    bus.tick(1);
    assert_eq!(bus.mode3_obj_fetch_dots_remaining(), 0);
    assert!(
        bus.mode3_obj_shutdown_dots_remaining() > 0,
        "expected OBJ fetch to hand over into shutdown ownership"
    );
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "window restart must not trigger on OBJ fetch->shutdown transition dot"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ must remain clear while still in mode3"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3"
    );
}

#[test]
fn mode3_window_trigger_does_not_fire_on_obj_shutdown_release_dot() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // Two separated sprite sessions so session 1 ends with shutdown dots.
    bus.write_byte(0xFE00, 18);
    bus.write_byte(0xFE01, 32);
    bus.write_byte(0xFE02, 0x00);
    bus.write_byte(0xFE03, 0x00);
    bus.write_byte(0xFE04, 18);
    bus.write_byte(0xFE05, 80);
    bus.write_byte(0xFE06, 0x00);
    bus.write_byte(0xFE07, 0x00);

    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX off-screen by default
    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ, window disabled
    wait_for_ly_mode(&mut bus, 2, 3);

    let mut armed = false;
    for _ in 0..512 {
        bus.tick(1);
        if bus.mode3_obj_shutdown_dots_remaining() == 1 && bus.mode3_output_x() > 0 {
            let output_x = bus.mode3_output_x();
            let wx = output_x.saturating_add(7).clamp(8, 166);
            bus.write_byte(0xFF4B, wx);
            bus.write_byte(0xFF40, bus.read_byte(0xFF40) | 0x20); // enable window mid-line
            armed = true;
            break;
        }
    }
    assert!(
        armed,
        "expected to arm window trigger while OBJ shutdown has one dot remaining"
    );
    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 1));

    // This dot decrements shutdown from 1 to 0. Window should still remain queued
    // because takeover was checked before shutdown ownership was released.
    bus.tick(1);
    assert_eq!(bus.mode3_obj_shutdown_dots_remaining(), 0);
    let release_is_push_ready_boundary = bus.mode3_bg_push_ready_takeover_boundary();
    let release_is_tileindex_boundary = bus.mode3_bg_fetch_phase() == 0
        && bus.mode3_bg_fetch_dots_remaining() == 0
        && bus.mode3_bg_takeover_boundary()
        && !bus.mode3_bg_push_stalled_for_fifo()
        && !bus.mode3_bg_push_recovery_sleep_pending()
        && !release_is_push_ready_boundary;
    assert!(
        !bus.mode3_window_triggered_this_line(),
        "window restart must not fire on the same dot OBJ shutdown releases"
    );
    assert!(bus.mode3_window_trigger_pending());
    assert_eq!(
        bus.read_byte(0xFF41) & 0x03,
        3,
        "mode3 should remain active on the OBJ shutdown release dot"
    );
    assert_eq!(
        bus.interrupt_flags() & (1 << 1),
        0,
        "STAT mode0 IRQ must remain clear while mode3 is still active"
    );
    assert_eq!(
        bus.read_byte(0x8000),
        0xFF,
        "VRAM should remain blocked in mode3 on the OBJ shutdown release dot"
    );
    assert_eq!(
        bus.read_byte(0xFE00),
        0xFF,
        "OAM should remain blocked in mode3 on the OBJ shutdown release dot"
    );
    assert!(
        release_is_tileindex_boundary || release_is_push_ready_boundary,
        "expected OBJ shutdown release dot to align with a classified BG boundary (tileindex or push-ready)"
    );
    assert_ne!(
        release_is_tileindex_boundary, release_is_push_ready_boundary,
        "OBJ shutdown release dot classification should be exactly one boundary class"
    );

    let mut triggered = false;
    for _ in 0..64 {
        bus.tick(1);
        if bus.mode3_window_triggered_this_line() {
            triggered = true;
            break;
        }
    }
    assert!(
        triggered,
        "expected queued window restart after a later valid boundary following shutdown release"
    );
}

#[test]
fn mode3_obj_contention_delays_vram_and_oam_release_into_hblank() {
    let mut bus_no_obj = setup_line2_mode3_bus_with_hidden_obj(false);
    let mut bus_with_obj = setup_line2_mode3_bus_with_hidden_obj(true);

    let ticks_to_mode0_no_obj = ticks_until_mode3_exit_on_current_line(&mut bus_no_obj);
    tick_n(&mut bus_with_obj, ticks_to_mode0_no_obj as usize);

    assert_eq!(bus_no_obj.read_byte(0xFF44), 2);
    assert_eq!(bus_no_obj.read_byte(0xFF41) & 0x03, 0);
    assert_eq!(bus_no_obj.read_byte(0x8000), 0x12);
    assert_eq!(bus_no_obj.read_byte(0xFE00), 18);

    assert_eq!(bus_with_obj.read_byte(0xFF44), 2);
    assert_eq!(bus_with_obj.read_byte(0xFF41) & 0x03, 3);
    assert_eq!(bus_with_obj.read_byte(0x8000), 0xFF);
    assert_eq!(bus_with_obj.read_byte(0xFE00), 0xFF);
}

#[test]
fn stat_mode0_irq_is_delayed_by_mode3_obj_contention() {
    let mut bus_no_obj = setup_line2_mode3_bus_with_hidden_obj(false);
    let mut bus_with_obj = setup_line2_mode3_bus_with_hidden_obj(true);

    bus_no_obj.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus_with_obj.write_byte(0xFF41, 0x08); // mode0 STAT source
    bus_no_obj.set_interrupt_flags(bus_no_obj.interrupt_flags() & !(1 << 1));
    bus_with_obj.set_interrupt_flags(bus_with_obj.interrupt_flags() & !(1 << 1));

    let no_obj_ticks = ticks_until_stat_irq(&mut bus_no_obj);
    let with_obj_ticks = ticks_until_stat_irq(&mut bus_with_obj);
    assert!(
        with_obj_ticks > no_obj_ticks,
        "expected mode3 OBJ contention to delay mode0 STAT IRQ (no_obj={no_obj_ticks}, with_obj={with_obj_ticks})"
    );
}

#[test]
fn mode3_wx_write_before_trigger_point_starts_window_midline() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX=167 => no window trigger in visible area
    bus.write_byte(0xFF40, 0xB9); // LCD on + BG + Window + BG map 9C00, tile data 8000

    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // Re-arm window before trigger point x=80 (WX=87).
    bus.tick(8);
    bus.write_byte(0xFF4B, 87);

    // Wait until LY=2 line is complete.
    wait_for_transition(&mut bus, 2, 3, 0);
    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    assert_eq!(frame[line2 + 32], 0xFF); // BG before trigger
    assert_eq!(frame[line2 + 120], 0x00); // Window after trigger
}

#[test]
fn mode3_wx_write_after_trigger_point_does_not_start_window_this_line() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP

    // BG tile map row (9C00) uses white tile.
    for i in 0..32u16 {
        bus.write_byte(0x9C00 + i, 0x00);
    }
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map row uses black tile.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0xA7); // WX=167 => no window trigger in visible area
    bus.write_byte(0xFF40, 0xB9); // LCD on + BG + Window + BG map 9C00, tile data 8000

    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // Pass trigger point x=9 (WX=16), then write WX to that already-past value.
    tick_n(&mut bus, 96);
    bus.write_byte(0xFF4B, 16);

    // Window must not retro-trigger after its comparator point has already passed.
    wait_for_transition(&mut bus, 2, 3, 0);
    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    assert_eq!(frame[line2 + 120], 0xFF);
}

#[test]
fn framebuffer_bg_disabled_forces_white_backdrop_ignoring_bgp() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xFF); // map all BG color IDs to shade 3 (black)

    // LCD on with BG/window disabled (LCDC.0=0).
    bus.write_byte(0xFF40, 0x90);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0xFF);
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
fn framebuffer_window_at_x0_does_not_inherit_bg_scx_discard() {
    fn render_top_line_with_scx(scx: u8) -> [u8; 160] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, scx); // SCX varies
        bus.write_byte(0xFF47, 0xE4); // identity palette

        // BG tiles vary so any visible coupling to SCX is easy to detect.
        for i in 0..32u16 {
            bus.write_byte(0x9800 + i, (i & 1) as u8);
        }
        // tile 0 => white
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);
        // tile 1 => black
        bus.write_byte(0x8010, 0xFF);
        bus.write_byte(0x8011, 0xFF);

        // Window map: constant distinct pattern (tile 2 then 3 repeating), independent of BG.
        for i in 0..32u16 {
            bus.write_byte(0x9C00 + i, if (i & 1) == 0 { 2 } else { 3 });
        }
        // tile 2 row0 => color id 1 across row
        bus.write_byte(0x8020, 0xFF);
        bus.write_byte(0x8021, 0x00);
        // tile 3 row0 => color id 2 across row
        bus.write_byte(0x8030, 0x00);
        bus.write_byte(0x8031, 0xFF);

        bus.write_byte(0xFF4A, 0x00); // WY=0
        bus.write_byte(0xFF4B, 0x07); // WX=7 => window starts at x=0

        // LCD on + window + BG, use window map 9C00 and tile data 8000.
        bus.write_byte(0xFF40, 0xF1);
        wait_for_next_frame(&mut bus);
        wait_for_next_frame(&mut bus);

        let mut line = [0u8; 160];
        line.copy_from_slice(&bus.framebuffer()[..160]);
        line
    }

    let line_scx0 = render_top_line_with_scx(0);
    let line_scx3 = render_top_line_with_scx(3);

    for x in 0..32usize {
        assert_eq!(
            line_scx0[x], line_scx3[x],
            "window pixels at x=0 (WX=7) must not inherit BG SCX discard (x={x})"
        );
    }
}

#[test]
fn framebuffer_window_restart_mid_sprite_does_not_corrupt_obj_pixels_when_bg_matches_window() {
    fn render_line_with_window_enabled(window_enabled: bool) -> [u8; 160] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, 0x00); // SCX
        bus.write_byte(0xFF47, 0xE4); // identity BGP
        bus.write_byte(0xFF48, 0xE4); // identity OBP0

        // BG and window both use the same white tiles so any visible difference
        // around WX restart must come from OBJ corruption, not background content.
        for i in 0..64u16 {
            bus.write_byte(0x9800 + i, 0x00);
            bus.write_byte(0x9C00 + i, 0x00);
        }
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);

        // Sprite tile with distinct per-column pattern (non-zero colors across row0).
        // low = 1010_1010, high = 1100_1100
        bus.write_byte(0x8020, 0xAA);
        bus.write_byte(0x8021, 0xCC);

        // Sprite visible at y=0, x=20..27, so WX=31 (x=24) crosses the sprite.
        bus.write_byte(0xFE00, 16); // Y => top at row 0
        bus.write_byte(0xFE01, 28); // X => left at x=20
        bus.write_byte(0xFE02, 2); // tile
        bus.write_byte(0xFE03, 0x00); // attrs

        bus.write_byte(0xFF4A, 0x00); // WY
        bus.write_byte(0xFF4B, 31); // WX => window starts at x=24

        let mut lcdc = 0x93; // LCD on + BG + OBJ
        if window_enabled {
            lcdc |= 0x20; // window enable
        }
        // Use BG map 9C00 + tile data 8000 for consistency with earlier window tests.
        lcdc |= 0x08 | 0x10;
        bus.write_byte(0xFF40, lcdc);

        // Render a stable frame (skip LCD-on startup quirks frame).
        wait_for_next_frame(&mut bus);
        wait_for_next_frame(&mut bus);

        let mut line = [0u8; 160];
        let frame = bus.framebuffer();
        line.copy_from_slice(&frame[..160]);
        line
    }

    let line_no_window = render_line_with_window_enabled(false);
    let line_with_window = render_line_with_window_enabled(true);

    // Compare a small span around the sprite/window overlap.
    for x in 16..32usize {
        assert_eq!(
            line_with_window[x], line_no_window[x],
            "window restart at WX boundary should not corrupt overlapping OBJ pixels when BG/window content matches (x={x})"
        );
    }
}

#[test]
fn framebuffer_window_restart_mid_multisprite_obj_does_not_split_columns_when_bg_matches_window() {
    fn render_line_with_window_enabled(window_enabled: bool) -> [u8; 160] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, 0x00); // SCX
        bus.write_byte(0xFF47, 0xE4); // identity BGP
        bus.write_byte(0xFF48, 0xE4); // identity OBP0

        for i in 0..64u16 {
            bus.write_byte(0x9800 + i, 0x00);
            bus.write_byte(0x9C00 + i, 0x00);
        }
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);

        // Two sprite tiles with different column patterns to catch column splits.
        // Tile 2 row0 pattern: 1,2,1,2,1,2,1,2
        bus.write_byte(0x8020, 0xAA); // low
        bus.write_byte(0x8021, 0x00); // high
        // Tile 3 row0 pattern: 3,1,3,1,3,1,3,1
        bus.write_byte(0x8030, 0xFF); // low
        bus.write_byte(0x8031, 0xAA); // high

        // 16px-wide object composed from 2 sprites spanning x=20..35.
        // WX=31 => window starts at x=24, crossing inside sprite 0 and before sprite 1.
        bus.write_byte(0xFE00, 16); // Y
        bus.write_byte(0xFE01, 28); // X => left=20
        bus.write_byte(0xFE02, 2); // tile 2
        bus.write_byte(0xFE03, 0x00);

        bus.write_byte(0xFE04, 16); // Y
        bus.write_byte(0xFE05, 36); // X => left=28
        bus.write_byte(0xFE06, 3); // tile 3
        bus.write_byte(0xFE07, 0x00);

        bus.write_byte(0xFF4A, 0x00); // WY
        bus.write_byte(0xFF4B, 31); // WX => x=24

        let mut lcdc = 0x93 | 0x08 | 0x10; // LCD+BG+OBJ + 9C00 map + 8000 tile data
        if window_enabled {
            lcdc |= 0x20;
        }
        bus.write_byte(0xFF40, lcdc);

        wait_for_next_frame(&mut bus);
        wait_for_next_frame(&mut bus);

        let mut line = [0u8; 160];
        let frame = bus.framebuffer();
        line.copy_from_slice(&frame[..160]);
        line
    }

    let line_no_window = render_line_with_window_enabled(false);
    let line_with_window = render_line_with_window_enabled(true);

    for x in 18..38usize {
        assert_eq!(
            line_with_window[x], line_no_window[x],
            "window restart should not split multi-sprite OBJ columns when BG/window content matches (x={x})"
        );
    }
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
