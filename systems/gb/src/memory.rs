mod api;
mod apu_bus;
mod bus_access;
mod cpu_bus;
mod cpu_context;
mod devices;
mod dma;
mod init;
mod mmio;
mod ppu;
mod scheduler;

use crate::apu::ApuState;
use crate::bootrom::BootRomData;
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;
use crate::timing::ClockRatios;
use bus_access::{VRAM_STORAGE_BYTES, WRAM_STORAGE_BYTES};
use devices::{JoypadState, SerialState, TimerState};
use dma::DmaState;
use mmio::CgbMmioState;
use ppu::PpuState;
use std::collections::VecDeque;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;
pub const LCD_FRAME_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;
const RECENT_KEY_MMIO_WRITES_LEN: usize = 16;
const KEY_MMIO_WRITE_EVENT_QUEUE_CAPACITY: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyMmioWriteEvent {
    pub tcycle: u64,
    pub addr: u16,
    pub value: u8,
}

pub struct Bus {
    cartridge: Cartridge,
    vram: [u8; VRAM_STORAGE_BYTES],
    wram: [u8; WRAM_STORAGE_BYTES],
    oam: [u8; 0x00A0],
    io: [u8; 0x0080],
    hram: [u8; 0x007F],
    ie: u8,
    timer: TimerState,
    ppu: PpuState,
    dma: DmaState,
    apu: ApuState,
    serial: SerialState,
    joypad: JoypadState,
    framebuffer: [u8; LCD_FRAME_PIXELS],
    framebuffer_palette_selectors: [u8; LCD_FRAME_PIXELS],
    clock_ratios: ClockRatios,
    hardware_model: HardwareModel,
    cgb_mmio: CgbMmioState,
    boot_rom: BootRomData,
    boot_rom_active: bool,
    recent_key_mmio_writes: [(u16, u8); RECENT_KEY_MMIO_WRITES_LEN],
    recent_key_mmio_writes_head: usize,
    recent_key_mmio_writes_len: usize,
    key_mmio_write_events: VecDeque<KeyMmioWriteEvent>,
    emulated_tcycles: u64,
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
