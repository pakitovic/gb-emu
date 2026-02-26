use super::super::Cartridge;

impl Cartridge {
    pub(super) fn write_mbc2_rom_control(&mut self, addr: u16, value: u8) {
        if !(0x0000..=0x3FFF).contains(&addr) {
            return;
        }
        if (addr & 0x0100) == 0 {
            self.ram_enabled = (value & 0x0F) == 0x0A;
        } else {
            self.mbc2_rom_bank_low4 = value & 0x0F;
        }
    }

    pub(super) fn read_mbc2_ram_byte(&self, addr: u16) -> u8 {
        if self.ram.is_empty() {
            return 0xFF;
        }
        let index = ((addr as usize).saturating_sub(0xA000)) & 0x01FF;
        let value = self.ram.get(index).copied().unwrap_or(0x0F) & 0x0F;
        value | 0xF0
    }

    pub(super) fn write_mbc2_ram_byte(&mut self, addr: u16, value: u8) {
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
}
