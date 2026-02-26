mod cgb;
mod map;
mod router;

pub(in crate::memory) use cgb::CgbMmioState;

#[cfg(test)]
pub(in crate::memory) use cgb::{CgbMmioRegister, cgb_mmio_register};
