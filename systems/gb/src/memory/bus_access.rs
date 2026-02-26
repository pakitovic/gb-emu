mod address_decode;
mod storage;

pub(in crate::memory) use address_decode::{AddressSegment, address_segment};
pub(in crate::memory) use storage::SegmentAccess;
pub(super) use storage::{VRAM_STORAGE_BYTES, WRAM_STORAGE_BYTES};
