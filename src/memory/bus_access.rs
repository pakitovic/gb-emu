use super::Bus;

impl Bus {
    pub(super) fn blocked_read_value(&self, addr: u16) -> Option<u8> {
        if matches!(addr, 0xFE00..=0xFE9F) && self.ppu_blocks_oam_read() {
            return Some(0xFF);
        }
        if matches!(addr, 0x8000..=0x9FFF) && self.ppu_blocks_vram_read() {
            return Some(0xFF);
        }
        None
    }
}
