use super::super::super::Bus;
use super::SegmentAccess;

impl Bus {
    fn oam_offset(addr: u16) -> usize {
        debug_assert!((0xFE00..=0xFE9F).contains(&addr));
        (addr - 0xFE00) as usize
    }

    pub(in crate::memory) fn read_oam(&self, addr: u16, access: SegmentAccess) -> u8 {
        if matches!(access, SegmentAccess::Cpu) && self.ppu_blocks_oam_read() {
            return 0xFF;
        }
        self.read_oam_index_internal(Self::oam_offset(addr))
    }

    pub(in crate::memory) fn write_oam(&mut self, addr: u16, value: u8, access: SegmentAccess) {
        if matches!(access, SegmentAccess::Cpu) && self.ppu_blocks_oam_write() {
            return;
        }
        self.write_oam_index_internal(Self::oam_offset(addr), value);
    }

    pub(in crate::memory) fn read_oam_index_internal(&self, index: usize) -> u8 {
        self.oam[index]
    }

    pub(in crate::memory) fn write_oam_index_internal(&mut self, index: usize, value: u8) {
        self.oam[index] = value;
    }
}
