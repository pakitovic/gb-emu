mod dma;
mod init;
mod interrupts;
mod io;
mod map;
mod ppu;
mod serial;
mod timer;

use crate::cartridge::Cartridge;

pub struct Bus {
    cartridge: Cartridge,
    vram: [u8; 0x2000],
    eram: [u8; 0x2000],
    wram: [u8; 0x2000],
    oam: [u8; 0x00A0],
    io: [u8; 0x0080],
    hram: [u8; 0x007F],
    ie: u8,
    serial_output: String,
    div_counter: u16,
    ly_counter: u16,
    tima_reload_delay: u8,
    tima_reload_block: u8,
    dma_active: bool,
    dma_source: u16,
    dma_pending_source: u16,
    dma_cycles_remaining: u16,
    dma_start_delay: u8,
    dma_cycle_accum: u8,
    dma_index: u8,
    serial_bits_remaining: u8,
    serial_tx_byte: u8,
    ppu_startup_line: bool,
    stat_irq_line: bool,
}

#[cfg(test)]
mod tests;
