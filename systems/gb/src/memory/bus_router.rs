use super::Bus;

#[derive(Clone, Copy)]
enum MainRegion {
    Rom,
    Vram,
    Eram,
    Wram,
    Echo,
    Oam,
    NotUsable,
    Io,
    Hram,
    Ie,
}

fn main_region(addr: u16) -> MainRegion {
    match addr {
        0x0000..=0x7FFF => MainRegion::Rom,
        0x8000..=0x9FFF => MainRegion::Vram,
        0xA000..=0xBFFF => MainRegion::Eram,
        0xC000..=0xDFFF => MainRegion::Wram,
        0xE000..=0xFDFF => MainRegion::Echo,
        0xFE00..=0xFE9F => MainRegion::Oam,
        0xFEA0..=0xFEFF => MainRegion::NotUsable,
        0xFF00..=0xFF7F => MainRegion::Io,
        0xFF80..=0xFFFE => MainRegion::Hram,
        0xFFFF => MainRegion::Ie,
    }
}

impl Bus {
    pub(super) fn read_byte_raw(&self, addr: u16) -> u8 {
        match main_region(addr) {
            MainRegion::Rom => self.cartridge.read_rom_byte(addr),
            MainRegion::Vram => self.vram[(addr - 0x8000) as usize],
            MainRegion::Eram => self.cartridge.read_ram_byte(addr),
            MainRegion::Wram => self.wram[(addr - 0xC000) as usize],
            MainRegion::Echo => self.wram[(addr - 0xE000) as usize],
            MainRegion::Oam => self.oam[(addr - 0xFE00) as usize],
            MainRegion::NotUsable => 0xFF,
            MainRegion::Io => self.read_io_register(addr),
            MainRegion::Hram => self.hram[(addr - 0xFF80) as usize],
            MainRegion::Ie => self.ie,
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        let region = main_region(addr);
        if matches!(region, MainRegion::Oam) && self.ppu_blocks_oam_write() {
            return;
        }
        if matches!(region, MainRegion::Vram) && self.ppu_blocks_vram_write() {
            return;
        }

        match region {
            MainRegion::Rom => self.cartridge.write_rom_control(addr, value),
            MainRegion::Vram => self.vram[(addr - 0x8000) as usize] = value,
            MainRegion::Eram => self.cartridge.write_ram_byte(addr, value),
            MainRegion::Wram => self.wram[(addr - 0xC000) as usize] = value,
            MainRegion::Echo => self.wram[(addr - 0xE000) as usize] = value,
            MainRegion::Oam => self.oam[(addr - 0xFE00) as usize] = value,
            MainRegion::NotUsable => {}
            MainRegion::Io => self.write_io_register(addr, value),
            MainRegion::Hram => self.hram[(addr - 0xFF80) as usize] = value,
            MainRegion::Ie => self.ie = value,
        }
    }
}
