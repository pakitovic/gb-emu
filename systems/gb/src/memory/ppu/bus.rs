use super::super::{Bus, LCD_WIDTH};

// Shared-bus register/framebuffer accessors used by the PPU state machine.
const IO_SCY: usize = 0x42;
const IO_SCX: usize = 0x43;
const IO_LY: usize = 0x44;
const IO_LYC: usize = 0x45;
const IO_BGP: usize = 0x47;
const IO_OBP0: usize = 0x48;
const IO_OBP1: usize = 0x49;
const IO_WY: usize = 0x4A;
const IO_WX: usize = 0x4B;
const IO_LCDC: usize = 0x40;
const IO_STAT: usize = 0x41;

impl Bus {
    pub(super) fn ppu_lcdc(&self) -> u8 {
        self.io[IO_LCDC]
    }

    pub(super) fn ppu_set_lcdc(&mut self, value: u8) {
        self.io[IO_LCDC] = value;
    }

    pub(super) fn ppu_stat(&self) -> u8 {
        self.io[IO_STAT]
    }

    pub(super) fn ppu_set_stat(&mut self, value: u8) {
        self.io[IO_STAT] = value;
    }

    pub(super) fn ppu_set_stat_mode_bits(&mut self, mode_bits: u8) {
        self.ppu_set_stat((self.ppu_stat() & !0x03) | (mode_bits & 0x03));
    }

    pub(super) fn ppu_set_stat_bits(&mut self, mask: u8) {
        self.ppu_set_stat(self.ppu_stat() | mask);
    }

    pub(super) fn ppu_clear_stat_bits(&mut self, mask: u8) {
        self.ppu_set_stat(self.ppu_stat() & !mask);
    }

    pub(super) fn ppu_ly(&self) -> u8 {
        self.io[IO_LY]
    }

    pub(super) fn ppu_set_ly(&mut self, value: u8) {
        self.io[IO_LY] = value;
    }

    pub(super) fn ppu_lyc(&self) -> u8 {
        self.io[IO_LYC]
    }

    pub(super) fn ppu_set_lyc(&mut self, value: u8) {
        self.io[IO_LYC] = value;
    }

    pub(super) fn ppu_scy(&self) -> u8 {
        self.io[IO_SCY]
    }

    pub(super) fn ppu_scx(&self) -> u8 {
        self.io[IO_SCX]
    }

    pub(super) fn ppu_bgp(&self) -> u8 {
        self.io[IO_BGP]
    }

    pub(super) fn ppu_obp0(&self) -> u8 {
        self.io[IO_OBP0]
    }

    pub(super) fn ppu_obp1(&self) -> u8 {
        self.io[IO_OBP1]
    }

    pub(super) fn ppu_wy(&self) -> u8 {
        self.io[IO_WY]
    }

    pub(super) fn ppu_wx(&self) -> u8 {
        self.io[IO_WX]
    }

    pub(super) fn ppu_clear_framebuffer_line(&mut self, ly: u8, luma: u8) {
        let row_start = (ly as usize) * LCD_WIDTH;
        self.framebuffer[row_start..row_start + LCD_WIDTH].fill(luma);
    }

    pub(super) fn ppu_write_framebuffer_pixel(&mut self, y: usize, x: usize, luma: u8) {
        self.framebuffer[y * LCD_WIDTH + x] = luma;
    }
}
