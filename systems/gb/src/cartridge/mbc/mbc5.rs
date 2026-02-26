use super::super::Cartridge;

impl Cartridge {
    pub(super) fn write_mbc5_rom_control(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x2FFF => {
                self.mbc5_rom_bank = (self.mbc5_rom_bank & 0x100) | value as u16;
            }
            0x3000..=0x3FFF => {
                self.mbc5_rom_bank = (self.mbc5_rom_bank & 0x00FF) | (((value & 0x01) as u16) << 8);
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
        }
    }
}
