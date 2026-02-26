use super::super::Cartridge;

impl Cartridge {
    pub(super) fn write_mbc3_rom_control(&mut self, addr: u16, value: u8) {
        match addr {
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
        }
    }

    pub(super) fn read_mbc3_ram_byte(&self, addr: u16) -> u8 {
        if self.mbc3_ram_bank_or_rtc >= 0x08 {
            let now_epoch_secs = self.current_rtc_epoch_secs();
            return match self.rtc.as_ref() {
                Some(rtc) if rtc.has_latched_snapshot => {
                    rtc.read_register(self.mbc3_ram_bank_or_rtc, true)
                }
                Some(rtc) => {
                    let live = rtc.live_registers_at_epoch(now_epoch_secs);
                    let index = (self.mbc3_ram_bank_or_rtc.saturating_sub(0x08)) as usize;
                    live.get(index).copied().unwrap_or(0xFF)
                }
                None => 0xFF,
            };
        }

        self.read_external_ram_byte(addr)
    }

    pub(super) fn write_mbc3_ram_byte(&mut self, addr: u16, value: u8) {
        if self.mbc3_ram_bank_or_rtc >= 0x08 {
            let now_epoch_secs = self.current_rtc_epoch_secs();
            if let Some(rtc) = self.rtc.as_mut() {
                rtc.write_register(self.mbc3_ram_bank_or_rtc, value, now_epoch_secs);
                self.save_dirty = true;
            }
            return;
        }

        self.write_external_ram_byte(addr, value);
    }
}
