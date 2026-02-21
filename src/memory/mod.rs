mod apu;
mod bus_access;
mod bus_router;
mod cpu_context;
mod dma;
mod init;
mod interrupts;
mod io;
mod io_map;
mod io_router;
mod joypad;
mod map;
mod ppu;
mod scheduler;
mod serial;
mod timer;

use crate::apu::ApuState;
use crate::cartridge::Cartridge;
use dma::DmaState;
use joypad::JoypadState;
use ppu::PpuState;
use serial::SerialState;
use timer::TimerState;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;
pub const LCD_FRAME_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

pub struct Bus {
    cartridge: Cartridge,
    vram: [u8; 0x2000],
    wram: [u8; 0x2000],
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
}

#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;
