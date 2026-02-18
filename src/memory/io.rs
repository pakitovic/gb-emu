use super::Bus;
use std::io::{self, Write};

impl Bus {
    pub fn read_byte(&self, addr: u16) -> u8 {
        if self.dma_active && matches!(addr, 0xFE00..=0xFE9F) {
            return 0xFF;
        }

        self.read_byte_raw(addr)
    }

    pub(super) fn read_byte_raw(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cartridge.read_rom_byte(addr),
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize],
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00..=0xFF7F => {
                let index = (addr - 0xFF00) as usize;
                if addr == 0xFF04 {
                    (self.div_counter >> 8) as u8
                } else if addr == 0xFF0F {
                    self.io[index] | 0xE0
                } else {
                    self.io[index]
                }
            }
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie,
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        if self.dma_active && matches!(addr, 0xFE00..=0xFE9F) {
            return;
        }

        match addr {
            0x0000..=0x7FFF => self.cartridge.write_rom_control(addr, value),
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize] = value,
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,
            0xFEA0..=0xFEFF => {}
            0xFF00..=0xFF7F => {
                self.write_io_register(addr, value);

                if addr == 0xFF02 && value == 0x81 {
                    let ch = self.io[0x01] as char;
                    self.serial_output.push(ch);
                    print!("{ch}");
                    let _ = io::stdout().flush();
                }
            }
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            0xFFFF => self.ie = value,
        }
    }

    fn write_io_register(&mut self, addr: u16, value: u8) {
        let index = (addr - 0xFF00) as usize;
        if addr == 0xFF0F {
            self.io[index] = (value & 0x1F) | 0xE0;
        } else if addr == 0xFF04 {
            let old_input = self.timer_input_high();
            self.div_counter = 0;
            let new_input = self.timer_input_high();
            if old_input && !new_input {
                self.increment_tima();
            }
        } else if addr == 0xFF46 {
            self.io[index] = value;
            self.start_oam_dma(value);
        } else if addr == 0xFF07 {
            let old_input = self.timer_input_high();
            self.io[index] = value;
            let new_input = self.timer_input_high();
            if old_input && !new_input {
                self.increment_tima();
            }
        } else if addr == 0xFF44 {
            self.io[index] = 0;
            self.ly_counter = 0;
        } else if addr == 0xFF05 {
            if self.tima_reload_block > 0 {
                // ignored
            } else if self.tima_reload_delay > 0 {
                self.io[index] = value;
                self.tima_reload_delay = 0;
            } else {
                self.io[index] = value;
            }
        } else if addr == 0xFF06 {
            self.io[index] = value;
            if self.tima_reload_block > 0 {
                self.io[0x05] = value;
            }
        } else {
            self.io[index] = value;
        }
    }
}
