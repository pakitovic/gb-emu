use super::Bus;

impl Bus {
    pub fn read_byte(&self, addr: u16) -> u8 {
        if matches!(addr, 0xFE00..=0xFE9F) && self.ppu_blocks_oam_read() {
            return 0xFF;
        }
        if matches!(addr, 0x8000..=0x9FFF) && self.ppu_blocks_vram_read() {
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
                if is_unmapped_io(addr) {
                    return 0xFF;
                }
                let index = (addr - 0xFF00) as usize;
                let value = if addr == 0xFF04 {
                    (self.div_counter >> 8) as u8
                } else if addr == 0xFF41 {
                    self.stat_read_value()
                } else {
                    self.io[index]
                };
                value | io_unused_bits_mask(addr)
            }
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie,
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        if matches!(addr, 0xFE00..=0xFE9F) && self.ppu_blocks_oam_write() {
            return;
        }
        if matches!(addr, 0x8000..=0x9FFF) && self.ppu_blocks_vram_write() {
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
            }
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            0xFFFF => self.ie = value,
        }
    }

    fn write_io_register(&mut self, addr: u16, value: u8) {
        if is_unmapped_io(addr) {
            return;
        }

        let index = (addr - 0xFF00) as usize;
        if addr == 0xFF0F {
            self.io[index] = (value & 0x1F) | 0xE0;
        } else if addr == 0xFF40 {
            self.write_lcdc(value);
        } else if addr == 0xFF41 {
            self.write_stat(value);
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
        } else if addr == 0xFF02 {
            self.write_sc(value);
        } else if addr == 0xFF07 {
            let old_input = self.timer_input_high();
            self.io[index] = value;
            let new_input = self.timer_input_high();
            if old_input && !new_input {
                self.increment_tima();
            }
        } else if addr == 0xFF44 {
            self.write_ly(value);
        } else if addr == 0xFF45 {
            self.write_lyc(value);
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

fn io_unused_bits_mask(addr: u16) -> u8 {
    match addr {
        0xFF00 => 0xC0, // P1
        0xFF02 => 0x7E, // SC
        0xFF07 => 0xF8, // TAC
        0xFF0F => 0xE0, // IF
        0xFF10 => 0x80, // NR10
        0xFF1A => 0x7F, // NR30
        0xFF1C => 0x9F, // NR32
        0xFF20 => 0xC0, // NR41
        0xFF23 => 0x3F, // NR44
        0xFF26 => 0x70, // NR52
        0xFF41 => 0x80, // STAT
        _ => 0x00,
    }
}

fn is_unmapped_io(addr: u16) -> bool {
    matches!(
        addr,
        0xFF03
            | 0xFF08..=0xFF0E
            | 0xFF15
            | 0xFF1F
            | 0xFF27..=0xFF2F
            | 0xFF4C..=0xFF7F
    )
}
