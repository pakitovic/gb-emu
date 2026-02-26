use super::*;

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
