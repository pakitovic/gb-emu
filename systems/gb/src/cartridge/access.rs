use super::{Cartridge, MapperType, ROM_BANK_BYTES};

impl Cartridge {
    pub fn read_rom_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.read_from_rom_bank(self.rom_bank_zero_index(), addr as usize),
            0x4000..=0x7FFF => self.read_from_rom_bank(
                self.rom_bank_switchable_index(),
                (addr as usize).saturating_sub(ROM_BANK_BYTES),
            ),
            _ => 0xFF,
        }
    }

    pub fn write_rom_control(&mut self, addr: u16, value: u8) {
        match self.mapper {
            MapperType::RomOnly => {}
            MapperType::Mbc1 => match addr {
                0x0000..=0x1FFF => {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                }
                0x2000..=0x3FFF => {
                    self.mbc1_rom_bank_low5 = value & 0x1F;
                }
                0x4000..=0x5FFF => self.mbc1_bank_high2 = value & 0x03,
                0x6000..=0x7FFF => self.mbc1_mode = value & 0x01,
                _ => {}
            },
            MapperType::Mbc2 => {
                if (0x0000..=0x3FFF).contains(&addr) {
                    if (addr & 0x0100) == 0 {
                        self.ram_enabled = (value & 0x0F) == 0x0A;
                    } else {
                        self.mbc2_rom_bank_low4 = value & 0x0F;
                    }
                }
            }
            MapperType::Mbc3 => match addr {
                0x0000..=0x1FFF => {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                }
                0x2000..=0x3FFF => {
                    self.mbc3_rom_bank_low7 = value & 0x7F;
                }
                0x4000..=0x5FFF => {
                    self.mbc3_ram_bank_or_rtc = value;
                }
                0x6000..=0x7FFF => {
                    let now_epoch_secs = self.current_rtc_epoch_secs();
                    if let Some(rtc) = self.rtc.as_mut() {
                        rtc.tick_to_epoch(now_epoch_secs);
                        rtc.latch_command(value);
                    }
                }
                _ => {}
            },
            MapperType::Mbc5 => match addr {
                0x0000..=0x1FFF => {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                }
                0x2000..=0x2FFF => {
                    self.mbc5_rom_bank = (self.mbc5_rom_bank & 0x100) | value as u16;
                }
                0x3000..=0x3FFF => {
                    self.mbc5_rom_bank =
                        (self.mbc5_rom_bank & 0x00FF) | (((value & 0x01) as u16) << 8);
                }
                0x4000..=0x5FFF => {
                    if self.has_rumble {
                        self.rumble_active = (value & 0x08) != 0;
                        self.mbc5_ram_bank = value & 0x07;
                    } else {
                        self.mbc5_ram_bank = value & 0x0F;
                    }
                }
                _ => {}
            },
        }
    }

    pub fn read_ram_byte(&self, addr: u16) -> u8 {
        if !(0xA000..=0xBFFF).contains(&addr) {
            return 0xFF;
        }
        if self.ram_enable_required && !self.ram_enabled {
            return 0xFF;
        }

        match self.mapper {
            MapperType::Mbc2 => {
                if self.ram.is_empty() {
                    return 0xFF;
                }
                let index = ((addr as usize).saturating_sub(0xA000)) & 0x01FF;
                let value = self.ram.get(index).copied().unwrap_or(0x0F) & 0x0F;
                value | 0xF0
            }
            MapperType::Mbc3 if self.mbc3_ram_bank_or_rtc >= 0x08 => {
                let now_epoch_secs = self.current_rtc_epoch_secs();
                match self.rtc.as_ref() {
                    Some(rtc) if rtc.has_latched_snapshot => {
                        rtc.read_register(self.mbc3_ram_bank_or_rtc, true)
                    }
                    Some(rtc) => {
                        let live = rtc.live_registers_at_epoch(now_epoch_secs);
                        let index = (self.mbc3_ram_bank_or_rtc.saturating_sub(0x08)) as usize;
                        live.get(index).copied().unwrap_or(0xFF)
                    }
                    None => 0xFF,
                }
            }
            _ => {
                if self.ram.is_empty() {
                    return 0xFF;
                }
                let Some(index) = self.ram_index(addr) else {
                    return 0xFF;
                };
                self.ram.get(index).copied().unwrap_or(0xFF)
            }
        }
    }

    pub fn write_ram_byte(&mut self, addr: u16, value: u8) {
        if !(0xA000..=0xBFFF).contains(&addr) {
            return;
        }
        if self.ram_enable_required && !self.ram_enabled {
            return;
        }

        match self.mapper {
            MapperType::Mbc2 => {
                if self.ram.is_empty() {
                    return;
                }
                let index = ((addr as usize).saturating_sub(0xA000)) & 0x01FF;
                if let Some(slot) = self.ram.get_mut(index) {
                    let next = value & 0x0F;
                    if *slot != next {
                        *slot = next;
                        self.save_dirty = true;
                    }
                }
            }
            MapperType::Mbc3 if self.mbc3_ram_bank_or_rtc >= 0x08 => {
                let now_epoch_secs = self.current_rtc_epoch_secs();
                if let Some(rtc) = self.rtc.as_mut() {
                    rtc.write_register(self.mbc3_ram_bank_or_rtc, value, now_epoch_secs);
                    self.save_dirty = true;
                }
            }
            _ => {
                if self.ram.is_empty() {
                    return;
                }
                let Some(index) = self.ram_index(addr) else {
                    return;
                };
                if let Some(slot) = self.ram.get_mut(index)
                    && *slot != value
                {
                    *slot = value;
                    self.save_dirty = true;
                }
            }
        }
    }
}
