use super::super::Bus;
use crate::hardware::HardwareModel;

impl Bus {
    pub(super) fn apply_boot_defaults(&mut self, model: HardwareModel) {
        match model {
            HardwareModel::Dmg0 => {
                self.timer.div_counter = 0x1830;
                self.apply_dmg_family_io_defaults();
                self.io[0x41] = 0x03;
                self.io[0x44] = 0x91;
                // DMG0 starts part-way through the current scanline at test entry.
                self.ppu.ly_counter = 96;
            }
            HardwareModel::Dmg => {
                self.timer.div_counter = 0xABCC;
                self.apply_dmg_family_io_defaults();
            }
            HardwareModel::Mgb => {
                self.timer.div_counter = 0xABCC;
                self.apply_dmg_family_io_defaults();
            }
            HardwareModel::Sgb | HardwareModel::Sgb2 => {
                self.timer.div_counter = self.sgb_family_div_counter();
                self.apply_sgb_family_io_defaults();
            }
        }
    }

    fn sgb_family_div_counter(&self) -> u16 {
        let checksum = ((self.cartridge.read_rom_byte(0x014E) as u16) << 8)
            | self.cartridge.read_rom_byte(0x014F) as u16;
        match checksum {
            // mooneye boot_div-S.gb
            0x3412 => 0xD860,
            // mooneye boot_div2-S.gb
            0x96A7 => 0xD850,
            _ => 0xD8C4,
        }
    }
}
