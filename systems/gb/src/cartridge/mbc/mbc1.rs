use super::super::Cartridge;

impl Cartridge {
    pub(super) fn write_mbc1_rom_control(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                self.mbc1_rom_bank_low5 = value & 0x1F;
            }
            0x4000..=0x5FFF => self.mbc1_bank_high2 = value & 0x03,
            0x6000..=0x7FFF => self.mbc1_mode = value & 0x01,
            _ => {}
        }
    }
}
