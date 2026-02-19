use super::Bus;
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self::new_with_model(cartridge, HardwareModel::default())
    }

    pub fn new_with_model(cartridge: Cartridge, model: HardwareModel) -> Self {
        let mut bus = Self {
            cartridge,
            vram: [0; 0x2000],
            eram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0x00A0],
            io: [0; 0x0080],
            hram: [0; 0x007F],
            ie: 0,
            timer: Default::default(),
            ppu: Default::default(),
            dma: Default::default(),
            serial: Default::default(),
            joypad: Default::default(),
            framebuffer: [0xFF; super::LCD_FRAME_PIXELS],
        };
        bus.apply_boot_defaults(model);
        bus.ppu.stat_irq_line = bus.stat_irq_source_active();
        bus
    }

    fn apply_boot_defaults(&mut self, model: HardwareModel) {
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

    fn apply_dmg_family_io_defaults(&mut self) {
        self.ie = 0x00;

        // FF00..FF07
        self.io[0x00] = 0x0F; // P1 low bits; high bits read back as 1
        self.io[0x01] = 0x00; // SB
        self.io[0x02] = 0x00; // SC (unused bits read back as 1)
        self.io[0x05] = 0x00; // TIMA
        self.io[0x06] = 0x00; // TMA
        self.io[0x07] = 0x00; // TAC (unused bits read back as 1)

        // FF0F
        self.io[0x0F] = 0xE1; // IF post-boot default

        // FF10..FF26 (APU)
        self.io[0x10] = 0x80;
        self.io[0x11] = 0xBF;
        self.io[0x12] = 0xF3;
        self.io[0x13] = 0xFF;
        self.io[0x14] = 0xBF;
        self.io[0x15] = 0xFF;
        self.io[0x16] = 0x3F;
        self.io[0x17] = 0x00;
        self.io[0x18] = 0xFF;
        self.io[0x19] = 0xBF;
        self.io[0x1A] = 0x00;
        self.io[0x1B] = 0xFF;
        self.io[0x1C] = 0x00;
        self.io[0x1D] = 0xFF;
        self.io[0x1E] = 0xBF;
        self.io[0x1F] = 0xFF;
        self.io[0x20] = 0xFF;
        self.io[0x21] = 0x00;
        self.io[0x22] = 0x00;
        self.io[0x23] = 0x80;
        self.io[0x24] = 0x77;
        self.io[0x25] = 0xF3;
        self.io[0x26] = 0x81;

        // FF40..FF4B
        self.io[0x40] = 0x91; // LCDC
        self.io[0x41] = 0x00; // STAT (bit7 reads as 1)
        self.io[0x42] = 0x00; // SCY
        self.io[0x43] = 0x00; // SCX
        self.io[0x44] = 0x00; // LY (advances during execution)
        self.io[0x45] = 0x00; // LYC
        self.io[0x46] = 0xFF; // DMA
        self.io[0x47] = 0xFC; // BGP
        self.io[0x48] = 0xFF; // OBP0
        self.io[0x49] = 0xFF; // OBP1
        self.io[0x4A] = 0x00; // WY
        self.io[0x4B] = 0x00; // WX
    }

    fn apply_sgb_family_io_defaults(&mut self) {
        self.ie = 0x00;

        // FF00..FF07
        self.io[0x00] = 0x3F; // P1 low bits; high bits read back as 1
        self.io[0x01] = 0x00; // SB
        self.io[0x02] = 0x00; // SC (unused bits read back as 1)
        self.io[0x05] = 0x00; // TIMA
        self.io[0x06] = 0x00; // TMA
        self.io[0x07] = 0x00; // TAC (unused bits read back as 1)

        // FF0F
        self.io[0x0F] = 0xE1; // IF post-boot default

        // FF10..FF26 (APU)
        self.io[0x10] = 0x80;
        self.io[0x11] = 0xBF;
        self.io[0x12] = 0xF3;
        self.io[0x13] = 0xFF;
        self.io[0x14] = 0xBF;
        self.io[0x15] = 0xFF;
        self.io[0x16] = 0x3F;
        self.io[0x17] = 0x00;
        self.io[0x18] = 0xFF;
        self.io[0x19] = 0xBF;
        self.io[0x1A] = 0x00;
        self.io[0x1B] = 0xFF;
        self.io[0x1C] = 0x00;
        self.io[0x1D] = 0xFF;
        self.io[0x1E] = 0xBF;
        self.io[0x1F] = 0xFF;
        self.io[0x20] = 0xFF;
        self.io[0x21] = 0x00;
        self.io[0x22] = 0x00;
        self.io[0x23] = 0x80;
        self.io[0x24] = 0x77;
        self.io[0x25] = 0xF3;
        self.io[0x26] = 0x80;

        // FF40..FF4B
        self.io[0x40] = 0xFF; // LCDC
        self.io[0x41] = 0x00; // STAT
        self.io[0x42] = 0x00; // SCY
        self.io[0x43] = 0x00; // SCX
        self.io[0x44] = 0x00; // LY
        self.io[0x45] = 0x00; // LYC
        self.io[0x46] = 0xFF; // DMA
        self.io[0x47] = 0xFC; // BGP
        self.io[0x48] = 0xFF; // OBP0
        self.io[0x49] = 0xFF; // OBP1
        self.io[0x4A] = 0x00; // WY
        self.io[0x4B] = 0x00; // WX
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
