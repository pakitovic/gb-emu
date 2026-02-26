use super::support::*;

#[test]
fn ppu_mode_edge_events_expose_mode_entries_and_vblank_irq_hook() {
    let mut bus = make_test_bus();
    let mut saw_oam = false;
    let mut saw_transfer = false;
    let mut saw_hblank = false;
    let mut saw_vblank = false;

    bus.set_interrupt_flags(bus.interrupt_flags() & !(1 << 0));

    for _ in 0..(154 * 456 * 3) {
        bus.tick(1);
        let mode = bus.debug_ppu_mode_kind();
        let edges = bus.debug_ppu_mode_edge_events();

        assert_eq!(
            mode as u8,
            bus.read_byte(0xFF41) & 0x03,
            "formal PPU mode should stay in sync with STAT mode bits"
        );

        if edges.entered_oam {
            assert_eq!(mode, PpuMode::Oam);
            saw_oam = true;
        }
        if edges.entered_transfer {
            assert_eq!(mode, PpuMode::Transfer);
            saw_transfer = true;
        }
        if edges.entered_hblank {
            assert_eq!(mode, PpuMode::HBlank);
            saw_hblank = true;
        }
        if edges.entered_vblank {
            assert_eq!(mode, PpuMode::VBlank);
            assert_eq!(bus.read_byte(0xFF44), 144);
            assert_ne!(
                bus.interrupt_flags() & (1 << 0),
                0,
                "entered_vblank edge should coincide with VBlank IF request"
            );
            saw_vblank = true;
        }

        if saw_oam && saw_transfer && saw_hblank && saw_vblank {
            return;
        }
    }

    panic!(
        "Did not observe all PPU mode entry edges (oam={saw_oam} transfer={saw_transfer} hblank={saw_hblank} vblank={saw_vblank})"
    );
}

#[test]
fn ppu_mode_edge_events_are_single_tick_pulses() {
    let mut bus = make_test_bus();

    for _ in 0..(154 * 456 * 2) {
        bus.tick(1);
        let edges = bus.debug_ppu_mode_edge_events();
        if edges.entered_hblank {
            bus.tick(1);
            let next_edges = bus.debug_ppu_mode_edge_events();
            assert!(
                !next_edges.entered_hblank,
                "HBlank entry edge should not remain latched beyond the entry tick"
            );
            return;
        }
    }

    panic!("No HBlank edge observed while testing edge pulse behavior");
}
