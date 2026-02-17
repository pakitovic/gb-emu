use crate::cartridge::Cartridge;
use std::io::{self, Write};

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
    timer_counter: u16,
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
            timer_counter: 0,
        };
        bus.io[0x0F] = 0xE1; // IF post-boot default
        bus
    }

    // ---------------- Read ----------------
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
            0xFF00..=0xFF7F => {
                let index = (addr - 0xFF00) as usize;
                if addr == 0xFF0F {
                    self.io[index] | 0xE0
                } else {
                    self.io[index]
                }
            }

            // HRAM: FF80-FFFE
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],

            // IE register: FFFF
            0xFFFF => self.ie,
        }
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

    pub fn tick(&mut self, cycles: u8) {
        let c = cycles as u16;

        // DIV increments at 16384 Hz => every 256 t-cycles.
        self.div_counter = self.div_counter.wrapping_add(c);
        while self.div_counter >= 256 {
            self.div_counter -= 256;
            self.io[0x04] = self.io[0x04].wrapping_add(1);
        }

        // Timer control (TAC)
        let tac = self.io[0x07];
        let timer_enabled = (tac & 0x04) != 0;
        if !timer_enabled {
            return;
        }

        let period = match tac & 0x03 {
            0x00 => 1024, // 4096 Hz
            0x01 => 16,   // 262144 Hz
            0x02 => 64,   // 65536 Hz
            0x03 => 256,  // 16384 Hz
            _ => unreachable!(),
        };

        self.timer_counter = self.timer_counter.wrapping_add(c);
        while self.timer_counter >= period {
            self.timer_counter -= period;

            let (next_tima, overflow) = self.io[0x05].overflowing_add(1);
            if overflow {
                self.io[0x05] = self.io[0x06]; // TIMA = TMA
                let iflags = self.interrupt_flags() | (1 << 2);
                self.set_interrupt_flags(iflags);
            } else {
                self.io[0x05] = next_tima;
            }
        }
    }

    // ---------------- Write ----------------
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            // ROM area (MBC not implemented yet)
            0x0000..=0x7FFF => self.cartridge.write_rom_control(addr, value),

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
            0xFF00..=0xFF7F => {
                let index = (addr - 0xFF00) as usize;
                if addr == 0xFF0F {
                    self.io[index] = (value & 0x1F) | 0xE0;
                } else if addr == 0xFF04 {
                    // Writing to DIV resets it.
                    self.io[index] = 0;
                    self.div_counter = 0;
                } else {
                    self.io[index] = value;
                }

                // Serial transfer control (FF02): when value is 0x81, emit byte from FF01.
                if addr == 0xFF02 && value == 0x81 {
                    let ch = self.io[0x01] as char;
                    self.serial_output.push(ch);
                    print!("{ch}");
                    let _ = io::stdout().flush();
                }
            }

            // HRAM: FF80-FFFE
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,

            // IE register: FFFF
            0xFFFF => self.ie = value,
        }
    }
}
