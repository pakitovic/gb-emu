mod dma;
mod io;
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
    dma_cycles_remaining: u16,
    dma_start_delay: u8,
    dma_cycle_accum: u8,
    dma_index: u8,
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        let mut bus = Self {
            cartridge,
            vram: [0; 0x2000],
            eram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0x00A0],
            io: [0; 0x0080],
            hram: [0; 0x007F],
            ie: 0,
            serial_output: String::new(),
            div_counter: 0,
            ly_counter: 0,
            tima_reload_delay: 0,
            tima_reload_block: 0,
            dma_active: false,
            dma_source: 0,
            dma_cycles_remaining: 0,
            dma_start_delay: 0,
            dma_cycle_accum: 0,
            dma_index: 0,
        };
        bus.io[0x0F] = 0xE1; // IF post-boot default
        bus
    }

    pub fn rom_title(&self) -> &str {
        self.cartridge.title()
    }

    pub fn serial_output(&self) -> &str {
        &self.serial_output
    }

    pub fn interrupt_enable(&self) -> u8 {
        self.ie
    }

    pub fn interrupt_flags(&self) -> u8 {
        self.io[0x0F] & 0x1F
    }

    pub fn set_interrupt_flags(&mut self, value: u8) {
        self.io[0x0F] = (value & 0x1F) | 0xE0;
    }

    pub fn pending_interrupts(&self) -> u8 {
        self.interrupt_enable() & self.interrupt_flags() & 0x1F
    }
}

#[cfg(test)]
mod tests;
