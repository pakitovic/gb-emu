use super::Bus;

const STAT_MODE_HBLANK: u8 = 0;
const STAT_MODE_VBLANK: u8 = 1;
const STAT_MODE_OAM: u8 = 2;
const STAT_MODE_TRANSFER: u8 = 3;

impl Bus {
    pub(super) fn lcd_enabled(&self) -> bool {
        (self.io[0x40] & 0x80) != 0
    }

    pub(super) fn ppu_allows_oam_access(&self) -> bool {
        if !self.lcd_enabled() {
            return true;
        }
        !matches!(self.io[0x41] & 0x03, STAT_MODE_OAM | STAT_MODE_TRANSFER)
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
                self.ly_counter = 0;
                self.ppu_startup_line = false;
                self.set_stat_mode(STAT_MODE_HBLANK);
                // LY=LYC flag is retained while LCD is disabled.
                self.update_stat_irq_line();
            }
            (false, true) => {
                self.io[0x44] = 0;
                self.ly_counter = 0;
                self.ppu_startup_line = true;
                self.set_stat_mode(STAT_MODE_HBLANK);
                self.update_lyc_flag();
                self.update_stat_irq_line();
            }
            _ => {}
        }
    }

    pub(super) fn write_stat(&mut self, value: u8) {
        // Bits 3..6 are writable. Bits 0..2 are PPU-generated.
        self.io[0x41] = (self.io[0x41] & 0x07) | (value & 0x78);
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
        self.ly_counter = 0;
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

        let ly = self.io[0x44];
        let line_length = self.line_length_tcycles(ly);
        self.ly_counter = self.ly_counter.wrapping_add(1);
        if self.ly_counter >= line_length {
            self.ly_counter = 0;
            let next_ly = if ly >= 153 { 0 } else { ly.wrapping_add(1) };
            self.io[0x44] = next_ly;

            if self.ppu_startup_line && ly == 0 {
                self.ppu_startup_line = false;
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
            self.mode_for_visible_line(self.ly_counter, self.ppu_startup_line && ly == 0)
        };
        self.set_stat_mode(mode);
        self.update_lyc_flag();
        self.update_stat_irq_line();
    }

    fn line_length_tcycles(&self, ly: u8) -> u16 {
        if self.ppu_startup_line && ly == 0 {
            452
        } else {
            456
        }
    }

    fn mode_for_visible_line(&self, line_cycle: u16, startup_line: bool) -> u8 {
        if startup_line {
            if line_cycle < 76 {
                STAT_MODE_HBLANK
            } else if line_cycle < 248 {
                STAT_MODE_TRANSFER
            } else {
                STAT_MODE_HBLANK
            }
        } else if line_cycle < 80 {
            STAT_MODE_OAM
        } else if line_cycle < 252 {
            STAT_MODE_TRANSFER
        } else {
            STAT_MODE_HBLANK
        }
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
        ((stat & 0x40) != 0 && lyc)
            || ((stat & 0x20) != 0 && mode == STAT_MODE_OAM)
            || ((stat & 0x10) != 0 && mode == STAT_MODE_VBLANK)
            || ((stat & 0x08) != 0 && mode == STAT_MODE_HBLANK)
    }

    fn update_stat_irq_line(&mut self) {
        let high = self.stat_irq_source_active();
        if high && !self.stat_irq_line {
            let iflags = self.interrupt_flags() | (1 << 1);
            self.set_interrupt_flags(iflags);
        }
        self.stat_irq_line = high;
    }
}
