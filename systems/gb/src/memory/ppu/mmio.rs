use super::*;

impl PpuState {
    pub(in crate::memory) fn configure_model_gates(bus: &mut Bus, model: HardwareModel) {
        bus.ppu.cgb_scaffold_runtime_enabled = Self::model_supports_cgb_scaffold(model);
    }

    pub(in crate::memory) fn write_lcdc(bus: &mut Bus, value: u8) {
        let was_enabled = Self::lcd_enabled(bus);
        let now_enabled = (value & 0x80) != 0;
        bus.io[0x40] = value;

        match (was_enabled, now_enabled) {
            (true, false) => {
                bus.io[0x44] = 0;
                bus.ppu.ly_counter = 0;
                bus.ppu.startup_line = false;
                bus.ppu.post_enable_phase = 0;
                bus.ppu.enable_delay = 0;
                bus.ppu.stat_mode0_enabled_this_line = false;
                bus.ppu.window_line_counter = 0;
                bus.ppu.window_triggered_this_line = false;
                bus.ppu.window_trigger_pending = false;
                bus.ppu.mode3_dots_latched = 0;
                bus.ppu.mode3_fifo.reset();
                bus.ppu.bg_color_ids_line.fill(0);
                Self::force_ppu_mode(bus, PpuMode::HBlank);
                // LY=LYC flag is retained while LCD is disabled.
                Self::update_stat_irq_line(bus);
            }
            (false, true) => {
                bus.io[0x44] = 0;
                bus.ppu.ly_counter = 0;
                bus.ppu.startup_line = true;
                bus.ppu.post_enable_phase = 0;
                bus.ppu.enable_delay = 0;
                bus.ppu.stat_mode0_enabled_this_line = false;
                bus.ppu.window_line_counter = 0;
                bus.ppu.window_triggered_this_line = false;
                bus.ppu.window_trigger_pending = false;
                bus.ppu.mode3_dots_latched = 0;
                bus.ppu.mode3_fifo.reset();
                bus.ppu.bg_color_ids_line.fill(0);
                Self::force_ppu_mode(bus, PpuMode::HBlank);
                Self::update_lyc_flag(bus);
                Self::update_stat_irq_line(bus);
            }
            _ => {}
        }
    }

    pub(in crate::memory) fn write_stat(bus: &mut Bus, value: u8) {
        // Bits 3..6 are writable. Bits 0..2 are PPU-generated.
        let old_mode0_source = (bus.io[0x41] & 0x08) != 0;
        bus.io[0x41] = (bus.io[0x41] & 0x07) | (value & 0x78);
        let new_mode0_source = (bus.io[0x41] & 0x08) != 0;
        if !old_mode0_source && new_mode0_source && Self::lcd_enabled(bus) && bus.io[0x44] < 144 {
            bus.ppu.stat_mode0_enabled_this_line = true;
        } else if !new_mode0_source {
            bus.ppu.stat_mode0_enabled_this_line = false;
        }
        Self::update_stat_irq_line(bus);
    }

    pub(in crate::memory) fn write_lyc(bus: &mut Bus, value: u8) {
        bus.io[0x45] = value;
        if Self::lcd_enabled(bus) {
            Self::update_lyc_flag(bus);
            Self::update_stat_irq_line(bus);
        }
    }

    pub(in crate::memory) fn write_ly(bus: &mut Bus, value: u8) {
        let _ = value;
        bus.io[0x44] = 0;
        bus.ppu.ly_counter = 0;
        bus.ppu.startup_line = false;
        bus.ppu.post_enable_phase = 0;
        bus.ppu.enable_delay = 0;
        bus.ppu.stat_mode0_enabled_this_line = false;
        bus.ppu.window_line_counter = 0;
        bus.ppu.window_triggered_this_line = false;
        bus.ppu.window_trigger_pending = false;
        bus.ppu.mode3_dots_latched = 0;
        bus.ppu.mode3_fifo.reset();
        bus.ppu.bg_color_ids_line.fill(0);
        Self::force_ppu_mode(bus, PpuMode::HBlank);
        if Self::lcd_enabled(bus) {
            Self::update_lyc_flag(bus);
        }
        Self::update_stat_irq_line(bus);
    }

    pub(super) fn lcd_enabled(bus: &Bus) -> bool {
        (bus.io[0x40] & 0x80) != 0
    }

    pub(super) fn ppu_mode(bus: &Bus) -> u8 {
        bus.ppu.mode.stat_mode_bits()
    }

    pub(super) fn ppu_startup_mode0_slice_active(bus: &Bus) -> bool {
        bus.ppu.post_enable_phase > 0
            && bus.io[0x44] > 0
            && bus.io[0x44] < 144
            && Self::ppu_mode(bus) == STAT_MODE_HBLANK
            && bus.ppu.ly_counter < 4
    }

    pub(super) fn ppu_startup_mode2_tail_active(bus: &Bus) -> bool {
        bus.ppu.post_enable_phase > 0
            && bus.io[0x44] > 0
            && bus.io[0x44] < 144
            && Self::ppu_mode(bus) == STAT_MODE_OAM
            && (80..84).contains(&bus.ppu.ly_counter)
    }

    pub(in crate::memory) fn ppu_blocks_oam_read(bus: &Bus) -> bool {
        bus.dma_blocks_oam_cpu_read()
            || Self::ppu_startup_mode0_slice_active(bus)
            || (Self::lcd_enabled(bus)
                && matches!(Self::ppu_mode(bus), STAT_MODE_OAM | STAT_MODE_TRANSFER))
    }

    pub(in crate::memory) fn ppu_blocks_oam_write(bus: &Bus) -> bool {
        bus.dma_blocks_oam_cpu_write() || !Self::ppu_allows_oam_access(bus)
    }

    pub(in crate::memory) fn ppu_blocks_vram_read(bus: &Bus) -> bool {
        Self::ppu_startup_mode2_tail_active(bus) || !Self::ppu_allows_vram_access(bus)
    }

    pub(in crate::memory) fn ppu_blocks_vram_write(bus: &Bus) -> bool {
        !Self::ppu_allows_vram_access(bus)
    }

    pub(in crate::memory) fn stat_read_value(bus: &Bus) -> u8 {
        let mut value = bus.io[0x41];
        if Self::ppu_startup_mode0_slice_active(bus) {
            value &= !0x04;
        }
        value
    }

    pub(super) fn ppu_allows_oam_access(bus: &Bus) -> bool {
        if !Self::lcd_enabled(bus) {
            return true;
        }
        // DMG LCD-on startup quirk: OAM remains briefly accessible in mode 2
        // around dot 80 on the first lines after enabling LCD.
        if Self::ppu_startup_mode2_tail_active(bus) {
            return true;
        }
        !matches!(Self::ppu_mode(bus), STAT_MODE_OAM | STAT_MODE_TRANSFER)
    }

    pub(super) fn ppu_allows_vram_access(bus: &Bus) -> bool {
        if !Self::lcd_enabled(bus) {
            return true;
        }
        (bus.io[0x41] & 0x03) != STAT_MODE_TRANSFER
    }

    pub(super) fn force_ppu_mode(bus: &mut Bus, mode: PpuMode) {
        bus.ppu.mode = mode;
        bus.ppu.mode_edge_events = PpuModeEdgeEvents::default();
        bus.io[0x41] = (bus.io[0x41] & !0x03) | mode.stat_mode_bits();
    }

    pub(super) fn set_ppu_mode(bus: &mut Bus, mode: PpuMode) -> PpuModeEdgeEvents {
        if bus.ppu.mode == mode {
            bus.io[0x41] = (bus.io[0x41] & !0x03) | mode.stat_mode_bits();
            bus.ppu.mode_edge_events = PpuModeEdgeEvents::default();
            return bus.ppu.mode_edge_events;
        }

        let edges = PpuModeEdgeEvents::for_entered_mode(mode);
        bus.ppu.mode = mode;
        bus.ppu.mode_edge_events = edges;
        bus.io[0x41] = (bus.io[0x41] & !0x03) | mode.stat_mode_bits();
        edges
    }

    pub(super) fn update_lyc_flag(bus: &mut Bus) {
        let lyc_match = bus.io[0x44] == bus.io[0x45];
        if lyc_match {
            bus.io[0x41] |= 0x04;
        } else {
            bus.io[0x41] &= !0x04;
        }
    }

    pub(in crate::memory) fn stat_irq_source_active(bus: &Bus) -> bool {
        let stat = bus.io[0x41];
        let mode = bus.ppu_mode_kind().stat_mode_bits();
        let lyc = (stat & 0x04) != 0;
        let mode_edges = bus.ppu_mode_edge_events();
        // DMG quirk: if mode 2 interrupt is enabled, entering LY=144 also
        // raises STAT alongside VBlank.
        let mode2_or_vblank_start =
            mode == STAT_MODE_OAM || (mode == STAT_MODE_VBLANK && mode_edges.entered_vblank);
        let mode0_active = mode == STAT_MODE_HBLANK && Self::mode0_stat_source_active_now(bus);
        ((stat & 0x40) != 0 && lyc)
            || ((stat & 0x20) != 0 && mode2_or_vblank_start)
            || ((stat & 0x10) != 0 && mode == STAT_MODE_VBLANK)
            || ((stat & 0x08) != 0 && mode0_active)
    }

    pub(super) fn mode0_stat_source_active_now(bus: &Bus) -> bool {
        let ly = bus.io[0x44];
        if ly >= 144 {
            return false;
        }

        let startup_line = bus.ppu.startup_line && ly == 0;
        let mode3_end = if startup_line {
            STARTUP_MODE0_DOTS.saturating_add(bus.ppu.mode3_dots_latched)
        } else {
            80u16.saturating_add(bus.ppu.mode3_dots_latched)
        };

        let delay_tcycles = if bus.ppu.stat_mode0_enabled_this_line {
            0
        } else {
            4
        };
        bus.ppu.ly_counter >= mode3_end.saturating_add(delay_tcycles)
    }

    pub(super) fn update_stat_irq_line(bus: &mut Bus) {
        let high = Self::stat_irq_source_active(bus);
        if high && !bus.ppu.stat_irq_line {
            let iflags = bus.interrupt_flags() | (1 << 1);
            bus.set_interrupt_flags(iflags);
        }
        bus.ppu.stat_irq_line = high;
    }
}
