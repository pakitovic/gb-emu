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
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;
use crate::timing::ClockRatios;
use bus_access::{VRAM_STORAGE_BYTES, WRAM_STORAGE_BYTES};
use devices::{JoypadState, SerialState, TimerState};
use dma::DmaState;
use mmio::CgbMmioState;
use ppu::PpuState;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;
pub const LCD_FRAME_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

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
    clock_ratios: ClockRatios,
    hardware_model: HardwareModel,
    cgb_mmio: CgbMmioState,
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
