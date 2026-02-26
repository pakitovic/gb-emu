use super::{Cartridge, MapperType, RAM_BANK_BYTES, ROM_BANK_BYTES};

impl Cartridge {
    pub(super) fn current_rtc_epoch_secs(&self) -> u64 {
        self.host_rtc_epoch_secs
            .unwrap_or_else(|| self.clock.now_epoch_secs())
    }

    pub(super) fn read_from_rom_bank(&self, bank_index: usize, bank_offset: usize) -> u8 {
        let bank = if self.rom_bank_count == 0 {
            0
        } else {
            bank_index % self.rom_bank_count
        };
        let offset = bank
            .saturating_mul(ROM_BANK_BYTES)
            .saturating_add(bank_offset % ROM_BANK_BYTES);
        self.rom.get(offset).copied().unwrap_or(0xFF)
    }

    pub(super) fn rom_bank_zero_index(&self) -> usize {
        match self.mapper {
            MapperType::RomOnly | MapperType::Mbc2 | MapperType::Mbc3 | MapperType::Mbc5 => 0,
            MapperType::Mbc1 => {
                if self.mbc1_mode == 0 {
                    0
                } else {
                    ((self.mbc1_bank_high2 as usize) << 5) % self.rom_bank_count.max(1)
                }
            }
        }
    }

    pub(super) fn rom_bank_switchable_index(&self) -> usize {
        match self.mapper {
            MapperType::RomOnly => 1 % self.rom_bank_count.max(1),
            MapperType::Mbc1 => {
                let low = self.mbc1_rom_bank_low5 & 0x1F;
                let low_nonzero = if low == 0 { 1 } else { low };
                (((self.mbc1_bank_high2 as usize) << 5) | (low_nonzero as usize))
                    % self.rom_bank_count.max(1)
            }
            MapperType::Mbc2 => {
                let low = self.mbc2_rom_bank_low4 & 0x0F;
                let low_nonzero = if low == 0 { 1 } else { low };
                (low_nonzero as usize) % self.rom_bank_count.max(1)
            }
            MapperType::Mbc3 => {
                let low = self.mbc3_rom_bank_low7 & 0x7F;
                let low_nonzero = if low == 0 { 1 } else { low };
                (low_nonzero as usize) % self.rom_bank_count.max(1)
            }
            MapperType::Mbc5 => (self.mbc5_rom_bank as usize) % self.rom_bank_count.max(1),
        }
    }

    pub(super) fn ram_bank_index(&self) -> usize {
        if self.ram_bank_count <= 1 {
            return 0;
        }
        match self.mapper {
            MapperType::RomOnly => 0,
            MapperType::Mbc1 => {
                if self.mbc1_mode == 0 {
                    0
                } else {
                    (self.mbc1_bank_high2 as usize) % self.ram_bank_count
                }
            }
            MapperType::Mbc2 => 0,
            MapperType::Mbc3 => {
                if self.mbc3_ram_bank_or_rtc <= 0x03 {
                    (self.mbc3_ram_bank_or_rtc as usize) % self.ram_bank_count
                } else {
                    0
                }
            }
            MapperType::Mbc5 => (self.mbc5_ram_bank as usize) % self.ram_bank_count,
        }
    }

    pub(super) fn ram_index(&self, addr: u16) -> Option<usize> {
        if self.ram.is_empty() {
            return None;
        }
        let bank_offset = ((addr as usize).saturating_sub(0xA000)) % RAM_BANK_BYTES;
        let index = self
            .ram_bank_index()
            .saturating_mul(RAM_BANK_BYTES)
            .saturating_add(bank_offset)
            % self.ram.len();
        Some(index)
    }
}
