mod oam;
mod vram_wram;

pub(super) const CGB_VRAM_BANK_COUNT_SCAFFOLD: usize = 2;
pub(super) const CGB_WRAM_BANK_COUNT_SCAFFOLD: usize = 8;
pub(super) const VRAM_BANK_SIZE: usize = 0x2000;
pub(super) const WRAM_BANK_SIZE: usize = 0x1000;
pub(in crate::memory) const VRAM_STORAGE_BYTES: usize =
    CGB_VRAM_BANK_COUNT_SCAFFOLD * VRAM_BANK_SIZE;
pub(in crate::memory) const WRAM_STORAGE_BYTES: usize =
    CGB_WRAM_BANK_COUNT_SCAFFOLD * WRAM_BANK_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum SegmentAccess {
    Cpu,
    Hardware,
}
