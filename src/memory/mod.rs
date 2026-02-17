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
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            vram: [0; 0x2000],
            eram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0x00A0],
            io: [0; 0x0080],
            hram: [0; 0x007F],
            ie: 0,
        }
    }

    // ---------------- Lectura ----------------
    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // ROM: 0000-7FFF
            0x0000..=0x7FFF => self.cartridge.read_rom_byte(addr),

            // VRAM: 8000-9FFF
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],

            // External RAM: A000-BFFF
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize],

            // Work RAM: C000-DFFF
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],

            // Echo RAM: E000-FDFF (mirror of C000-DDFF)
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],

            // OAM: FE00-FE9F
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],

            // Unusable: FEA0-FEFF
            0xFEA0..=0xFEFF => 0xFF,

            // IO: FF00-FF7F
            0xFF00..=0xFF7F => self.io[(addr - 0xFF00) as usize],

            // HRAM: FF80-FFFE
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],

            // IE register: FFFF
            0xFFFF => self.ie,
        }
    }

    pub fn rom_title(&self) -> &str {
        self.cartridge.title()
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    // ---------------- Escritura ----------------
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            // ROM area (MBC not implemented yet)
            0x0000..=0x7FFF => {}

            // VRAM: 8000-9FFF
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,

            // External RAM: A000-BFFF
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize] = value,

            // Work RAM: C000-DFFF
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = value,

            // Echo RAM: E000-FDFF (mirror of C000-DDFF)
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = value,

            // OAM: FE00-FE9F
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,

            // Unusable: FEA0-FEFF
            0xFEA0..=0xFEFF => {}

            // IO: FF00-FF7F
            0xFF00..=0xFF7F => self.io[(addr - 0xFF00) as usize] = value,

            // HRAM: FF80-FFFE
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,

            // IE register: FFFF
            0xFFFF => self.ie = value,
        }
    }
}
