use super::Bus;

const STAT_MODE_HBLANK: u8 = 0;
const STAT_MODE_VBLANK: u8 = 1;
const STAT_MODE_OAM: u8 = 2;
const STAT_MODE_TRANSFER: u8 = 3;
const STARTUP_MODE0_DOTS: u16 = 80;
const STARTUP_LINE_DOTS: u16 = 452;

#[derive(Default)]
pub(super) struct PpuState {
    pub(super) ly_counter: u16,
    pub(super) startup_line: bool,
    pub(super) post_enable_phase: u8,
    pub(super) enable_delay: u8,
    pub(super) stat_irq_line: bool,
    pub(super) stat_mode0_enabled_this_line: bool,
    pub(super) frame_counter: u64,
}

impl PpuState {
    pub(super) fn write_lcdc(bus: &mut Bus, value: u8) {
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
                Self::set_stat_mode(bus, STAT_MODE_HBLANK);
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
                Self::set_stat_mode(bus, STAT_MODE_HBLANK);
                Self::update_lyc_flag(bus);
                Self::update_stat_irq_line(bus);
            }
            _ => {}
        }
    }

    pub(super) fn write_stat(bus: &mut Bus, value: u8) {
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

    pub(super) fn write_lyc(bus: &mut Bus, value: u8) {
        bus.io[0x45] = value;
        if Self::lcd_enabled(bus) {
            Self::update_lyc_flag(bus);
            Self::update_stat_irq_line(bus);
        }
    }

    pub(super) fn write_ly(bus: &mut Bus, value: u8) {
        let _ = value;
        bus.io[0x44] = 0;
        bus.ppu.ly_counter = 0;
        bus.ppu.startup_line = false;
        bus.ppu.post_enable_phase = 0;
        bus.ppu.enable_delay = 0;
        bus.ppu.stat_mode0_enabled_this_line = false;
        Self::set_stat_mode(bus, STAT_MODE_HBLANK);
        if Self::lcd_enabled(bus) {
            Self::update_lyc_flag(bus);
        }
        Self::update_stat_irq_line(bus);
    }

    pub(super) fn step(bus: &mut Bus) {
        if !Self::lcd_enabled(bus) {
            return;
        }

        if bus.ppu.enable_delay > 0 {
            bus.ppu.enable_delay -= 1;
            Self::update_lyc_flag(bus);
            Self::update_stat_irq_line(bus);
            return;
        }

        let ly = bus.io[0x44];
        let line_length = Self::line_length_tcycles(bus, ly);
        bus.ppu.ly_counter = bus.ppu.ly_counter.wrapping_add(1);
        if bus.ppu.ly_counter >= line_length {
            bus.ppu.ly_counter = 0;
            let next_ly = if ly >= 153 { 0 } else { ly.wrapping_add(1) };
            bus.io[0x44] = next_ly;
            bus.ppu.stat_mode0_enabled_this_line = false;

            if bus.ppu.startup_line && ly == 0 {
                bus.ppu.startup_line = false;
                bus.ppu.post_enable_phase = 2;
            } else if bus.ppu.post_enable_phase > 0 {
                bus.ppu.post_enable_phase -= 1;
            }
            if next_ly == 144 {
                let iflags = bus.interrupt_flags() | (1 << 0);
                bus.set_interrupt_flags(iflags);
                bus.ppu.frame_counter = bus.ppu.frame_counter.wrapping_add(1);
                Self::render_placeholder_frame(bus);
            }
        }

        let ly = bus.io[0x44];
        let mode = if ly >= 144 {
            STAT_MODE_VBLANK
        } else {
            Self::mode_for_visible_line(
                bus,
                ly,
                bus.ppu.ly_counter,
                bus.ppu.startup_line && ly == 0,
            )
        };
        Self::set_stat_mode(bus, mode);
        Self::update_lyc_flag(bus);
        Self::update_stat_irq_line(bus);
    }

    fn lcd_enabled(bus: &Bus) -> bool {
        (bus.io[0x40] & 0x80) != 0
    }

    fn ppu_mode(bus: &Bus) -> u8 {
        bus.io[0x41] & 0x03
    }

    fn ppu_startup_mode0_slice_active(bus: &Bus) -> bool {
        bus.ppu.post_enable_phase > 0
            && bus.io[0x44] > 0
            && bus.io[0x44] < 144
            && Self::ppu_mode(bus) == STAT_MODE_HBLANK
            && bus.ppu.ly_counter < 4
    }

    fn ppu_startup_mode2_tail_active(bus: &Bus) -> bool {
        bus.ppu.post_enable_phase > 0
            && bus.io[0x44] > 0
            && bus.io[0x44] < 144
            && Self::ppu_mode(bus) == STAT_MODE_OAM
            && (80..84).contains(&bus.ppu.ly_counter)
    }

    pub(super) fn ppu_blocks_oam_read(bus: &Bus) -> bool {
        bus.dma.active
            || Self::ppu_startup_mode0_slice_active(bus)
            || (Self::lcd_enabled(bus)
                && matches!(Self::ppu_mode(bus), STAT_MODE_OAM | STAT_MODE_TRANSFER))
    }

    pub(super) fn ppu_blocks_oam_write(bus: &Bus) -> bool {
        bus.dma.active || !Self::ppu_allows_oam_access(bus)
    }

    pub(super) fn ppu_blocks_vram_read(bus: &Bus) -> bool {
        Self::ppu_startup_mode2_tail_active(bus) || !Self::ppu_allows_vram_access(bus)
    }

    pub(super) fn ppu_blocks_vram_write(bus: &Bus) -> bool {
        !Self::ppu_allows_vram_access(bus)
    }

    pub(super) fn stat_read_value(bus: &Bus) -> u8 {
        let mut value = bus.io[0x41];
        if Self::ppu_startup_mode0_slice_active(bus) {
            value &= !0x04;
        }
        value
    }

    fn ppu_allows_oam_access(bus: &Bus) -> bool {
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

    fn ppu_allows_vram_access(bus: &Bus) -> bool {
        if !Self::lcd_enabled(bus) {
            return true;
        }
        (bus.io[0x41] & 0x03) != STAT_MODE_TRANSFER
    }

    fn line_length_tcycles(bus: &Bus, ly: u8) -> u16 {
        if bus.ppu.startup_line && ly == 0 {
            STARTUP_LINE_DOTS
        } else {
            456
        }
    }

    fn mode_for_visible_line(bus: &Bus, ly: u8, line_cycle: u16, startup_line: bool) -> u8 {
        let mode3_dots = Self::mode3_length_tcycles(bus, ly, startup_line);
        if startup_line {
            if line_cycle < STARTUP_MODE0_DOTS {
                STAT_MODE_HBLANK
            } else if line_cycle < STARTUP_MODE0_DOTS.saturating_add(mode3_dots) {
                STAT_MODE_TRANSFER
            } else {
                STAT_MODE_HBLANK
            }
        } else {
            let mode2_end = match bus.ppu.post_enable_phase {
                2 => 84u16,
                1 => 84u16,
                _ => 80u16,
            };

            if bus.ppu.post_enable_phase == 0 {
                if line_cycle < 80 {
                    STAT_MODE_OAM
                } else if line_cycle < 80u16.saturating_add(mode3_dots) {
                    STAT_MODE_TRANSFER
                } else {
                    STAT_MODE_HBLANK
                }
            } else if line_cycle < 4 {
                STAT_MODE_HBLANK
            } else if line_cycle < mode2_end {
                STAT_MODE_OAM
            } else if line_cycle < mode2_end.saturating_add(mode3_dots) {
                STAT_MODE_TRANSFER
            } else {
                STAT_MODE_HBLANK
            }
        }
    }

    fn mode3_length_tcycles(bus: &Bus, ly: u8, startup_line: bool) -> u16 {
        let extra = Self::mode3_extra_tcycles(bus, ly);
        let base = 172u16.saturating_add(extra);
        let line_len = Self::line_length_tcycles(bus, ly);
        if startup_line {
            // Startup line starts in mode 0 and skips mode 2.
            base.min(line_len.saturating_sub(STARTUP_MODE0_DOTS))
        } else {
            // Visible lines always spend 80 dots in mode 2.
            base.min(line_len.saturating_sub(80))
        }
    }

    fn mode3_extra_tcycles(bus: &Bus, ly: u8) -> u16 {
        let scx_penalty = (bus.io[0x43] & 0x07) as u16;
        scx_penalty.saturating_add(Self::mode3_obj_penalty_tcycles(bus, ly))
    }

    fn mode3_obj_penalty_tcycles(bus: &Bus, ly: u8) -> u16 {
        // Objects disabled.
        if (bus.io[0x40] & 0x02) == 0 {
            return 0;
        }

        let sprite_height = if (bus.io[0x40] & 0x04) != 0 {
            16i16
        } else {
            8i16
        };

        // DMG pipeline considers up to 10 sprites per scanline.
        let mut sprites: [(u8, u8); 10] = [(0, 0); 10]; // (x, oam_index)
        let mut sprite_count = 0usize;
        for oam_index in 0u8..40u8 {
            let base = (oam_index as usize) * 4;
            let y = bus.oam[base] as i16 - 16;
            let x = bus.oam[base + 1];

            // Drawn sprites are X=0 special case or X in 1..=167.
            if x != 0 && x >= 168 {
                continue;
            }

            let ly_i = ly as i16;
            if ly_i < y || ly_i >= y + sprite_height {
                continue;
            }

            if sprite_count < sprites.len() {
                sprites[sprite_count] = (x, oam_index);
                sprite_count += 1;
            }
        }

        // Penalty order is left-to-right; ties broken by OAM index.
        sprites[..sprite_count].sort_unstable();

        // DMG sprite timing heuristic compatible with mooneye acceptance:
        // - sprites are evaluated in sessions
        // - first sprite in a session has startup adjustment based on X mod 8
        // - every additional sprite in the same session costs 6 dots
        // - every ended (non-final) session incurs a shutdown adjustment
        //
        // A new session starts when sprite X has a gap of 8+ pixels compared
        // to the previous sprite.
        const SHUTDOWN_PENALTY: [u16; 8] = [3, 2, 3, 2, 3, 2, 2, 2];

        let mut penalty = 0u16;
        let mut i = 0usize;
        while i < sprite_count {
            let mut j = i;
            while j + 1 < sprite_count {
                let x = sprites[j].0;
                let next_x = sprites[j + 1].0;
                if next_x.wrapping_sub(x) < 8 {
                    j += 1;
                } else {
                    break;
                }
            }

            let first_x_mod = (sprites[i].0 & 0x07) as i16;
            let startup_adjust = match first_x_mod {
                0 | 1 => 2,
                4..=7 => -2,
                _ => 0,
            };
            let first_penalty = (6i16 + startup_adjust) as u16;
            penalty = penalty.saturating_add(first_penalty);

            let additional_sprites = j - i;
            penalty = penalty.saturating_add((additional_sprites as u16).saturating_mul(6));

            if j + 1 < sprite_count {
                let last_x_mod = (sprites[j].0 & 0x07) as usize;
                penalty = penalty.saturating_add(SHUTDOWN_PENALTY[last_x_mod]);
            }

            i = j + 1;
        }

        penalty
    }

    fn render_placeholder_frame(bus: &mut Bus) {
        // Frontend bootstrap placeholder until full pixel pipeline is exposed.
        let phase = (bus.ppu.frame_counter & 0x1F) as usize;
        for y in 0..super::LCD_HEIGHT {
            for x in 0..super::LCD_WIDTH {
                let stripe = ((x + phase) / 8) & 1;
                let checker = ((x / 16) ^ (y / 16)) & 1;
                let shade = match (stripe, checker) {
                    (0, 0) => 0xE0,
                    (0, 1) => 0xB0,
                    (1, 0) => 0x70,
                    _ => 0x30,
                };
                bus.framebuffer[y * super::LCD_WIDTH + x] = shade;
            }
        }
    }

    fn set_stat_mode(bus: &mut Bus, mode: u8) {
        bus.io[0x41] = (bus.io[0x41] & !0x03) | (mode & 0x03);
    }

    fn update_lyc_flag(bus: &mut Bus) {
        let lyc_match = bus.io[0x44] == bus.io[0x45];
        if lyc_match {
            bus.io[0x41] |= 0x04;
        } else {
            bus.io[0x41] &= !0x04;
        }
    }

    pub(super) fn stat_irq_source_active(bus: &Bus) -> bool {
        let stat = bus.io[0x41];
        let mode = stat & 0x03;
        let lyc = (stat & 0x04) != 0;
        // DMG quirk: if mode 2 interrupt is enabled, entering LY=144 also
        // raises STAT alongside VBlank.
        let mode2_or_vblank_start = mode == STAT_MODE_OAM
            || (mode == STAT_MODE_VBLANK && bus.io[0x44] == 144 && bus.ppu.ly_counter == 0);
        let mode0_active = mode == STAT_MODE_HBLANK && Self::mode0_stat_source_active_now(bus);
        ((stat & 0x40) != 0 && lyc)
            || ((stat & 0x20) != 0 && mode2_or_vblank_start)
            || ((stat & 0x10) != 0 && mode == STAT_MODE_VBLANK)
            || ((stat & 0x08) != 0 && mode0_active)
    }

    fn mode0_stat_source_active_now(bus: &Bus) -> bool {
        let ly = bus.io[0x44];
        if ly >= 144 {
            return false;
        }

        let startup_line = bus.ppu.startup_line && ly == 0;
        let mode3_end = if startup_line {
            STARTUP_MODE0_DOTS.saturating_add(Self::mode3_length_tcycles(bus, ly, true))
        } else {
            80u16.saturating_add(Self::mode3_length_tcycles(bus, ly, false))
        };

        let delay_tcycles = if bus.ppu.stat_mode0_enabled_this_line {
            0
        } else {
            4
        };
        bus.ppu.ly_counter >= mode3_end.saturating_add(delay_tcycles)
    }

    fn update_stat_irq_line(bus: &mut Bus) {
        let high = Self::stat_irq_source_active(bus);
        if high && !bus.ppu.stat_irq_line {
            let iflags = bus.interrupt_flags() | (1 << 1);
            bus.set_interrupt_flags(iflags);
        }
        bus.ppu.stat_irq_line = high;
    }
}

impl Bus {
    pub(super) fn ppu_blocks_oam_read(&self) -> bool {
        PpuState::ppu_blocks_oam_read(self)
    }

    pub(super) fn ppu_blocks_oam_write(&self) -> bool {
        PpuState::ppu_blocks_oam_write(self)
    }

    pub(super) fn ppu_blocks_vram_read(&self) -> bool {
        PpuState::ppu_blocks_vram_read(self)
    }

    pub(super) fn ppu_blocks_vram_write(&self) -> bool {
        PpuState::ppu_blocks_vram_write(self)
    }

    pub(super) fn stat_read_value(&self) -> u8 {
        PpuState::stat_read_value(self)
    }

    pub(super) fn write_lcdc(&mut self, value: u8) {
        PpuState::write_lcdc(self, value);
    }

    pub(super) fn write_stat(&mut self, value: u8) {
        PpuState::write_stat(self, value);
    }

    pub(super) fn write_lyc(&mut self, value: u8) {
        PpuState::write_lyc(self, value);
    }

    pub(super) fn write_ly(&mut self, value: u8) {
        PpuState::write_ly(self, value);
    }

    pub(super) fn step_ppu(&mut self) {
        PpuState::step(self);
    }

    pub(super) fn stat_irq_source_active(&self) -> bool {
        PpuState::stat_irq_source_active(self)
    }
}
