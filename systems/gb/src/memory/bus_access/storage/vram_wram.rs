use super::super::super::Bus;
use super::{
    CGB_VRAM_BANK_COUNT_SCAFFOLD, CGB_WRAM_BANK_COUNT_SCAFFOLD, SegmentAccess, VRAM_BANK_SIZE,
    WRAM_BANK_SIZE, WRAM_STORAGE_BYTES,
};

impl Bus {
    fn vram_bank_offset(addr: u16) -> usize {
        debug_assert!((0x8000..=0x9FFF).contains(&addr));
        (addr - 0x8000) as usize
    }

    fn vram_linear_index(&self, bank: u8, bank_offset: usize) -> usize {
        debug_assert!(bank_offset < VRAM_BANK_SIZE);
        debug_assert!((bank as usize) < CGB_VRAM_BANK_COUNT_SCAFFOLD);
        (bank as usize) * VRAM_BANK_SIZE + bank_offset
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
        debug_assert!((bank_slot as usize) < CGB_WRAM_BANK_COUNT_SCAFFOLD);
        (bank_slot as usize) * WRAM_BANK_SIZE + bank_offset
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
        let (bank_slot, bank_offset) = self.wram_bank_slot_and_offset(addr);
        self.read_wram_bank_index_internal(bank_slot, bank_offset)
    }

    pub(in crate::memory) fn write_wram(&mut self, addr: u16, value: u8) {
        let (bank_slot, bank_offset) = self.wram_bank_slot_and_offset(addr);
        self.write_wram_bank_index_internal(bank_slot, bank_offset, value);
    }

    pub(in crate::memory) fn read_vram_index_internal(&self, index: usize) -> u8 {
        self.read_vram_bank_index_internal(0, index)
    }

    pub(in crate::memory) fn write_vram_index_internal(&mut self, index: usize, value: u8) {
        self.write_vram_bank_index_internal(0, index, value);
    }

    pub(in crate::memory) fn read_wram_index_internal(&self, index: usize) -> u8 {
        debug_assert!(index < WRAM_STORAGE_BYTES);
        self.wram[index]
    }

    pub(in crate::memory) fn write_wram_index_internal(&mut self, index: usize, value: u8) {
        debug_assert!(index < WRAM_STORAGE_BYTES);
        self.wram[index] = value;
    }

    pub(in crate::memory) fn read_vram_bank_index_internal(&self, bank: u8, index: usize) -> u8 {
        debug_assert!(index < VRAM_BANK_SIZE);
        let linear = self.vram_linear_index(bank, index);
        self.vram[linear]
    }

    pub(in crate::memory) fn write_vram_bank_index_internal(
        &mut self,
        bank: u8,
        index: usize,
        value: u8,
    ) {
        debug_assert!(index < VRAM_BANK_SIZE);
        let linear = self.vram_linear_index(bank, index);
        self.vram[linear] = value;
    }

    pub(in crate::memory) fn read_wram_bank_index_internal(&self, bank: u8, index: usize) -> u8 {
        debug_assert!(index < WRAM_BANK_SIZE);
        let linear = self.wram_linear_index(bank, index);
        self.read_wram_index_internal(linear)
    }

    pub(in crate::memory) fn write_wram_bank_index_internal(
        &mut self,
        bank: u8,
        index: usize,
        value: u8,
    ) {
        debug_assert!(index < WRAM_BANK_SIZE);
        let linear = self.wram_linear_index(bank, index);
        self.write_wram_index_internal(linear, value);
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_storage_bank_backing_lengths(&self) -> (usize, usize) {
        (self.vram.len(), self.wram.len())
    }
}
