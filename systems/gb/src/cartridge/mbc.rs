mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;
mod rom_only;

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
            MapperType::RomOnly => self.write_rom_only_control(addr, value),
            MapperType::Mbc1 => self.write_mbc1_rom_control(addr, value),
            MapperType::Mbc2 => self.write_mbc2_rom_control(addr, value),
            MapperType::Mbc3 => self.write_mbc3_rom_control(addr, value),
            MapperType::Mbc5 => self.write_mbc5_rom_control(addr, value),
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
            MapperType::RomOnly => self.read_external_ram_byte(addr),
            MapperType::Mbc1 => self.read_external_ram_byte(addr),
            MapperType::Mbc2 => self.read_mbc2_ram_byte(addr),
            MapperType::Mbc3 => self.read_mbc3_ram_byte(addr),
            MapperType::Mbc5 => self.read_external_ram_byte(addr),
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
            MapperType::RomOnly => self.write_external_ram_byte(addr, value),
            MapperType::Mbc1 => self.write_external_ram_byte(addr, value),
            MapperType::Mbc2 => self.write_mbc2_ram_byte(addr, value),
            MapperType::Mbc3 => self.write_mbc3_ram_byte(addr, value),
            MapperType::Mbc5 => self.write_external_ram_byte(addr, value),
        }
    }

    fn read_external_ram_byte(&self, addr: u16) -> u8 {
        if self.ram.is_empty() {
            return 0xFF;
        }
        let Some(index) = self.ram_index(addr) else {
            return 0xFF;
        };
        self.ram.get(index).copied().unwrap_or(0xFF)
    }

    fn write_external_ram_byte(&mut self, addr: u16, value: u8) {
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
