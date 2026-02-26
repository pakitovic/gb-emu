use super::Bus;
use crate::hardware::HardwareModel;

#[cfg(test)]
mod tests;

const STAT_MODE_HBLANK: u8 = 0;
const STAT_MODE_VBLANK: u8 = 1;
const STAT_MODE_OAM: u8 = 2;
const STAT_MODE_TRANSFER: u8 = 3;
const STARTUP_MODE0_DOTS: u16 = 80;
const STARTUP_LINE_DOTS: u16 = 452;
const DMG_SHADE_TO_LUMA: [u8; 4] = [0xFF, 0xAA, 0x55, 0x00];
const MAX_SPRITES_PER_LINE: usize = 10;
const MODE3_BG_WARMUP_DOTS: u8 = 12;
const BG_FIFO_CAPACITY: usize = 16;
const BG_FETCH_TILE_DOTS: u8 = 6;
const BG_FETCH_PHASE_DOTS: u8 = 2;
const MODE3_WINDOW_RESTART_DOTS: u16 = BG_FETCH_TILE_DOTS as u16;
const OBJ_FETCH_BASE_DOTS: u8 = 6;
const OBJ_SESSION_SHUTDOWN_PENALTY: [u8; 8] = [3, 2, 3, 2, 3, 2, 2, 2];

mod bus;
mod host;
mod mmio;
mod mode3;
mod modes;
mod render;
mod state;
mod step;

pub(in crate::memory) use modes::{PpuMode, PpuModeEdgeEvents};
pub(in crate::memory) use state::PpuState;

use state::{
    BgFetchPhase, BgFifoPixel, BgPushSubstate, CgbBgTileAttrsScaffold, CgbObjAttrsScaffold,
    DmgPaletteSelector, Mode3CgbPixelMetaScaffold, Mode3ObjSprite, Mode3PixelMeta,
    Mode3PixelPriorityFlags, Mode3PixelSource, ObjCandidate, ObjFifoPixel,
};
