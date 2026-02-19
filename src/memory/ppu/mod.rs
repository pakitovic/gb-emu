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
}

impl Bus {
    pub(super) fn lcd_enabled(&self) -> bool {
        (self.io[0x40] & 0x80) != 0
    }

    fn ppu_mode(&self) -> u8 {
        self.io[0x41] & 0x03
    }

    pub(super) fn ppu_startup_mode0_slice_active(&self) -> bool {
        self.ppu.post_enable_phase > 0
            && self.io[0x44] > 0
            && self.io[0x44] < 144
            && self.ppu_mode() == STAT_MODE_HBLANK
            && self.ppu.ly_counter < 4
    }

    pub(super) fn ppu_startup_mode2_tail_active(&self) -> bool {
        self.ppu.post_enable_phase > 0
            && self.io[0x44] > 0
            && self.io[0x44] < 144
            && self.ppu_mode() == STAT_MODE_OAM
            && (80..84).contains(&self.ppu.ly_counter)
    }

    pub(super) fn ppu_blocks_oam_read(&self) -> bool {
        self.dma.active
            || self.ppu_startup_mode0_slice_active()
            || (self.lcd_enabled() && matches!(self.ppu_mode(), STAT_MODE_OAM | STAT_MODE_TRANSFER))
    }

    pub(super) fn ppu_blocks_oam_write(&self) -> bool {
        self.dma.active || !self.ppu_allows_oam_access()
    }

    pub(super) fn ppu_blocks_vram_read(&self) -> bool {
        self.ppu_startup_mode2_tail_active() || !self.ppu_allows_vram_access()
    }

    pub(super) fn ppu_blocks_vram_write(&self) -> bool {
        !self.ppu_allows_vram_access()
    }

    pub(super) fn stat_read_value(&self) -> u8 {
        let mut value = self.io[0x41];
        if self.ppu_startup_mode0_slice_active() {
            value &= !0x04;
        }
        value
    }

    pub(super) fn ppu_allows_oam_access(&self) -> bool {
        if !self.lcd_enabled() {
            return true;
        }
        // DMG LCD-on startup quirk: OAM remains briefly accessible in mode 2
        // around dot 80 on the first lines after enabling LCD.
        if self.ppu_startup_mode2_tail_active() {
            return true;
        }
        !matches!(self.ppu_mode(), STAT_MODE_OAM | STAT_MODE_TRANSFER)
    }

    pub(super) fn ppu_allows_vram_access(&self) -> bool {
        if !self.lcd_enabled() {
            return true;
        }
        (self.io[0x41] & 0x03) != STAT_MODE_TRANSFER
    }

    pub(super) fn write_lcdc(&mut self, value: u8) {
        let was_enabled = self.lcd_enabled();
        let now_enabled = (value & 0x80) != 0;
        self.io[0x40] = value;

        match (was_enabled, now_enabled) {
            (true, false) => {
                self.io[0x44] = 0;
                self.ppu.ly_counter = 0;
                self.ppu.startup_line = false;
                self.ppu.post_enable_phase = 0;
                self.ppu.enable_delay = 0;
                self.ppu.stat_mode0_enabled_this_line = false;
                self.set_stat_mode(STAT_MODE_HBLANK);
                // LY=LYC flag is retained while LCD is disabled.
                self.update_stat_irq_line();
            }
            (false, true) => {
                self.io[0x44] = 0;
                self.ppu.ly_counter = 0;
                self.ppu.startup_line = true;
                self.ppu.post_enable_phase = 0;
                self.ppu.enable_delay = 0;
                self.ppu.stat_mode0_enabled_this_line = false;
                self.set_stat_mode(STAT_MODE_HBLANK);
                self.update_lyc_flag();
                self.update_stat_irq_line();
            }
            _ => {}
        }
    }

    pub(super) fn write_stat(&mut self, value: u8) {
        // Bits 3..6 are writable. Bits 0..2 are PPU-generated.
        let old_mode0_source = (self.io[0x41] & 0x08) != 0;
        self.io[0x41] = (self.io[0x41] & 0x07) | (value & 0x78);
        let new_mode0_source = (self.io[0x41] & 0x08) != 0;
        if !old_mode0_source && new_mode0_source && self.lcd_enabled() && self.io[0x44] < 144 {
            self.ppu.stat_mode0_enabled_this_line = true;
        } else if !new_mode0_source {
            self.ppu.stat_mode0_enabled_this_line = false;
        }
        self.update_stat_irq_line();
    }

    pub(super) fn write_lyc(&mut self, value: u8) {
        self.io[0x45] = value;
        if self.lcd_enabled() {
            self.update_lyc_flag();
            self.update_stat_irq_line();
        }
    }

    pub(super) fn write_ly(&mut self, value: u8) {
        let _ = value;
        self.io[0x44] = 0;
        self.ppu.ly_counter = 0;
        self.ppu.startup_line = false;
        self.ppu.post_enable_phase = 0;
        self.ppu.enable_delay = 0;
        self.ppu.stat_mode0_enabled_this_line = false;
        if self.lcd_enabled() {
            self.set_stat_mode(STAT_MODE_HBLANK);
            self.update_lyc_flag();
        } else {
            self.set_stat_mode(STAT_MODE_HBLANK);
        }
        self.update_stat_irq_line();
    }

    pub(super) fn step_ppu(&mut self) {
        if !self.lcd_enabled() {
            return;
        }

        if self.ppu.enable_delay > 0 {
            self.ppu.enable_delay -= 1;
            self.update_lyc_flag();
            self.update_stat_irq_line();
            return;
        }

        let ly = self.io[0x44];
        let line_length = self.line_length_tcycles(ly);
        self.ppu.ly_counter = self.ppu.ly_counter.wrapping_add(1);
        if self.ppu.ly_counter >= line_length {
            self.ppu.ly_counter = 0;
            let next_ly = if ly >= 153 { 0 } else { ly.wrapping_add(1) };
            self.io[0x44] = next_ly;
            self.ppu.stat_mode0_enabled_this_line = false;

            if self.ppu.startup_line && ly == 0 {
                self.ppu.startup_line = false;
                self.ppu.post_enable_phase = 2;
            } else if self.ppu.post_enable_phase > 0 {
                self.ppu.post_enable_phase -= 1;
            }
            if next_ly == 144 {
                let iflags = self.interrupt_flags() | (1 << 0);
                self.set_interrupt_flags(iflags);
            }
        }

        let ly = self.io[0x44];
        let mode = if ly >= 144 {
            STAT_MODE_VBLANK
        } else {
            self.mode_for_visible_line(ly, self.ppu.ly_counter, self.ppu.startup_line && ly == 0)
        };
        self.set_stat_mode(mode);
        self.update_lyc_flag();
        self.update_stat_irq_line();
    }

    fn line_length_tcycles(&self, ly: u8) -> u16 {
        if self.ppu.startup_line && ly == 0 {
            STARTUP_LINE_DOTS
        } else {
            456
        }
    }

    fn mode_for_visible_line(&self, ly: u8, line_cycle: u16, startup_line: bool) -> u8 {
        let mode3_dots = self.mode3_length_tcycles(ly, startup_line);
        if startup_line {
            if line_cycle < STARTUP_MODE0_DOTS {
                STAT_MODE_HBLANK
            } else if line_cycle < STARTUP_MODE0_DOTS.saturating_add(mode3_dots) {
                STAT_MODE_TRANSFER
            } else {
                STAT_MODE_HBLANK
            }
        } else {
            let mode2_end = match self.ppu.post_enable_phase {
                2 => 84u16,
                1 => 84u16,
                _ => 80u16,
            };

            if self.ppu.post_enable_phase == 0 {
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

    fn mode3_length_tcycles(&self, ly: u8, startup_line: bool) -> u16 {
        let extra = self.mode3_extra_tcycles(ly);
        let base = 172u16.saturating_add(extra);
        let line_len = self.line_length_tcycles(ly);
        if startup_line {
            // Startup line starts in mode 0 and skips mode 2.
            base.min(line_len.saturating_sub(STARTUP_MODE0_DOTS))
        } else {
            // Visible lines always spend 80 dots in mode 2.
            base.min(line_len.saturating_sub(80))
        }
    }

    fn mode3_extra_tcycles(&self, ly: u8) -> u16 {
        let scx_penalty = (self.io[0x43] & 0x07) as u16;
        scx_penalty.saturating_add(self.mode3_obj_penalty_tcycles(ly))
    }

    fn mode3_obj_penalty_tcycles(&self, ly: u8) -> u16 {
        // Objects disabled.
        if (self.io[0x40] & 0x02) == 0 {
            return 0;
        }

        let sprite_height = if (self.io[0x40] & 0x04) != 0 {
            16i16
        } else {
            8i16
        };

        // DMG pipeline considers up to 10 sprites per scanline.
        let mut sprites: [(u8, u8); 10] = [(0, 0); 10]; // (x, oam_index)
        let mut sprite_count = 0usize;
        for oam_index in 0u8..40u8 {
            let base = (oam_index as usize) * 4;
            let y = self.oam[base] as i16 - 16;
            let x = self.oam[base + 1];

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

    fn set_stat_mode(&mut self, mode: u8) {
        self.io[0x41] = (self.io[0x41] & !0x03) | (mode & 0x03);
    }

    fn update_lyc_flag(&mut self) {
        let lyc_match = self.io[0x44] == self.io[0x45];
        if lyc_match {
            self.io[0x41] |= 0x04;
        } else {
            self.io[0x41] &= !0x04;
        }
    }

    pub(super) fn stat_irq_source_active(&self) -> bool {
        let stat = self.io[0x41];
        let mode = stat & 0x03;
        let lyc = (stat & 0x04) != 0;
        // DMG quirk: if mode 2 interrupt is enabled, entering LY=144 also
        // raises STAT alongside VBlank.
        let mode2_or_vblank_start = mode == STAT_MODE_OAM
            || (mode == STAT_MODE_VBLANK && self.io[0x44] == 144 && self.ppu.ly_counter == 0);
        let mode0_active = mode == STAT_MODE_HBLANK && self.mode0_stat_source_active_now();
        ((stat & 0x40) != 0 && lyc)
            || ((stat & 0x20) != 0 && mode2_or_vblank_start)
            || ((stat & 0x10) != 0 && mode == STAT_MODE_VBLANK)
            || ((stat & 0x08) != 0 && mode0_active)
    }

    fn mode0_stat_source_active_now(&self) -> bool {
        let ly = self.io[0x44];
        if ly >= 144 {
            return false;
        }

        let startup_line = self.ppu.startup_line && ly == 0;
        let mode3_end = if startup_line {
            STARTUP_MODE0_DOTS.saturating_add(self.mode3_length_tcycles(ly, true))
        } else {
            80u16.saturating_add(self.mode3_length_tcycles(ly, false))
        };

        let delay_tcycles = if self.ppu.stat_mode0_enabled_this_line {
            0
        } else {
            4
        };
        self.ppu.ly_counter >= mode3_end.saturating_add(delay_tcycles)
    }

    fn update_stat_irq_line(&mut self) {
        let high = self.stat_irq_source_active();
        if high && !self.ppu.stat_irq_line {
            let iflags = self.interrupt_flags() | (1 << 1);
            self.set_interrupt_flags(iflags);
        }
        self.ppu.stat_irq_line = high;
    }
}
