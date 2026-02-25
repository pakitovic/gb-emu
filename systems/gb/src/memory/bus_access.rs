use super::Bus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum AddressSegment {
    Rom,
    Vram,
    Eram,
    Wram,
    EchoWram,
    Oam,
    NotUsable,
    Io,
    Hram,
    Ie,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum SegmentAccess {
    Cpu,
    Hardware,
}

pub(in crate::memory) fn address_segment(addr: u16) -> AddressSegment {
    match addr {
        0x0000..=0x7FFF => AddressSegment::Rom,
        0x8000..=0x9FFF => AddressSegment::Vram,
        0xA000..=0xBFFF => AddressSegment::Eram,
        0xC000..=0xDFFF => AddressSegment::Wram,
        0xE000..=0xFDFF => AddressSegment::EchoWram,
        0xFE00..=0xFE9F => AddressSegment::Oam,
        0xFEA0..=0xFEFF => AddressSegment::NotUsable,
        0xFF00..=0xFF7F => AddressSegment::Io,
        0xFF80..=0xFFFE => AddressSegment::Hram,
        0xFFFF => AddressSegment::Ie,
    }
}

impl Bus {
    fn vram_offset(addr: u16) -> usize {
        debug_assert!((0x8000..=0x9FFF).contains(&addr));
        (addr - 0x8000) as usize
    }

    fn wram_offset(addr: u16) -> usize {
        match addr {
            0xC000..=0xDFFF => (addr - 0xC000) as usize,
            0xE000..=0xFDFF => (addr - 0xE000) as usize,
            _ => {
                debug_assert!(false, "WRAM helper used with non-WRAM address {:04X}", addr);
                0
            }
        }
    }

    fn oam_offset(addr: u16) -> usize {
        debug_assert!((0xFE00..=0xFE9F).contains(&addr));
        (addr - 0xFE00) as usize
    }

    pub(in crate::memory) fn read_vram(&self, addr: u16, access: SegmentAccess) -> u8 {
        if matches!(access, SegmentAccess::Cpu) && self.ppu_blocks_vram_read() {
            return 0xFF;
        }
        self.read_vram_index_internal(Self::vram_offset(addr))
    }

    pub(in crate::memory) fn write_vram(&mut self, addr: u16, value: u8, access: SegmentAccess) {
        if matches!(access, SegmentAccess::Cpu) && self.ppu_blocks_vram_write() {
            return;
        }
        self.write_vram_index_internal(Self::vram_offset(addr), value);
    }

    pub(in crate::memory) fn read_wram(&self, addr: u16) -> u8 {
        self.read_wram_index_internal(Self::wram_offset(addr))
    }

    pub(in crate::memory) fn write_wram(&mut self, addr: u16, value: u8) {
        self.write_wram_index_internal(Self::wram_offset(addr), value);
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

    pub(in crate::memory) fn read_vram_index_internal(&self, index: usize) -> u8 {
        self.vram[index]
    }

    pub(in crate::memory) fn write_vram_index_internal(&mut self, index: usize, value: u8) {
        self.vram[index] = value;
    }

    pub(in crate::memory) fn read_wram_index_internal(&self, index: usize) -> u8 {
        self.wram[index]
    }

    pub(in crate::memory) fn write_wram_index_internal(&mut self, index: usize, value: u8) {
        self.wram[index] = value;
    }

    pub(in crate::memory) fn read_oam_index_internal(&self, index: usize) -> u8 {
        self.oam[index]
    }

    pub(in crate::memory) fn write_oam_index_internal(&mut self, index: usize, value: u8) {
        self.oam[index] = value;
    }
}
