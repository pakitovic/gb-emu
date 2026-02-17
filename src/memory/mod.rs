pub struct Bus {
    rom: Vec<u8>,
}

impl Bus {
    pub fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }

    // ---------------- Lectura ----------------
    pub fn read_byte(&self, addr: u16) -> u8 {
        self.rom[addr as usize]
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr + 1) as u16;
        (high << 8) | low
    }

    // ---------------- Escritura ----------------
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        self.rom[addr as usize] = value;
    }
}
