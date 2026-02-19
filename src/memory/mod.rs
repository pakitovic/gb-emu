mod bus_access;
mod bus_router;
mod cpu_context;
mod dma;
mod init;
mod interrupts;
mod io;
mod io_map;
mod io_router;
mod map;
mod ppu;
mod scheduler;
mod serial;
mod timer;

use crate::cartridge::Cartridge;
use dma::DmaState;
use ppu::PpuState;
use serial::SerialState;
use timer::TimerState;

pub struct Bus {
    cartridge: Cartridge,
    vram: [u8; 0x2000],
    eram: [u8; 0x2000],
    wram: [u8; 0x2000],
    oam: [u8; 0x00A0],
    io: [u8; 0x0080],
    hram: [u8; 0x007F],
    ie: u8,
    timer: TimerState,
    ppu: PpuState,
    dma: DmaState,
    serial: SerialState,
}

#[cfg(test)]
mod tests;
