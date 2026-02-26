use super::Bus;

const VRAM_BANK_SIZE: usize = 0x2000;
const WRAM_BANK_SIZE: usize = 0x1000;

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
    fn vram_bank_offset(addr: u16) -> usize {
        debug_assert!((0x8000..=0x9FFF).contains(&addr));
        (addr - 0x8000) as usize
    }

    fn vram_linear_index(&self, bank: u8, bank_offset: usize) -> usize {
        debug_assert!(bank_offset < VRAM_BANK_SIZE);
        debug_assert_eq!(
            bank, 0,
            "DMG VRAM banking scaffold should still resolve to bank 0"
        );
        bank_offset
    }

    fn selected_vram_bank_for_bus_access(&self) -> u8 {
        self.cgb_mmio.dmg_effective_vram_bank()
    }

    fn vram_linear_index_for_addr(&self, addr: u16) -> usize {
        self.vram_linear_index(
            self.selected_vram_bank_for_bus_access(),
            Self::vram_bank_offset(addr),
        )
    }

    fn wram_bank_slot_and_offset(&self, addr: u16) -> (u8, usize) {
        match addr {
            0xC000..=0xCFFF => (0, (addr - 0xC000) as usize),
            0xD000..=0xDFFF => (
                self.cgb_mmio.dmg_effective_wram_bank_slot(),
                (addr - 0xD000) as usize,
            ),
            0xE000..=0xEFFF => (0, (addr - 0xE000) as usize),
            0xF000..=0xFDFF => (
                self.cgb_mmio.dmg_effective_wram_bank_slot(),
                (addr - 0xF000) as usize,
            ),
            _ => {
                debug_assert!(false, "WRAM helper used with non-WRAM address {:04X}", addr);
                (0, 0)
            }
        }
    }

    fn wram_linear_index(&self, bank_slot: u8, bank_offset: usize) -> usize {
        debug_assert!(bank_offset < WRAM_BANK_SIZE);
        debug_assert!(
            bank_slot <= 1,
            "DMG WRAM banking scaffold should only resolve to slots 0/1"
        );
        (bank_slot as usize) * WRAM_BANK_SIZE + bank_offset
    }

    fn wram_linear_index_for_addr(&self, addr: u16) -> usize {
        let (bank_slot, bank_offset) = self.wram_bank_slot_and_offset(addr);
        self.wram_linear_index(bank_slot, bank_offset)
    }

    fn oam_offset(addr: u16) -> usize {
        debug_assert!((0xFE00..=0xFE9F).contains(&addr));
        (addr - 0xFE00) as usize
    }

    pub(in crate::memory) fn read_vram(&self, addr: u16, access: SegmentAccess) -> u8 {
        if matches!(access, SegmentAccess::Cpu) && self.ppu_blocks_vram_read() {
            return 0xFF;
        }
        self.read_vram_index_internal(self.vram_linear_index_for_addr(addr))
    }

    pub(in crate::memory) fn write_vram(&mut self, addr: u16, value: u8, access: SegmentAccess) {
        if matches!(access, SegmentAccess::Cpu) && self.ppu_blocks_vram_write() {
            return;
        }
        self.write_vram_index_internal(self.vram_linear_index_for_addr(addr), value);
    }

    pub(in crate::memory) fn read_wram(&self, addr: u16) -> u8 {
        self.read_wram_index_internal(self.wram_linear_index_for_addr(addr))
    }

    pub(in crate::memory) fn write_wram(&mut self, addr: u16, value: u8) {
        self.write_wram_index_internal(self.wram_linear_index_for_addr(addr), value);
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
