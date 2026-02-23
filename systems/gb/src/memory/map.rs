use super::Bus;
use crate::cartridge::{CartridgeError, CartridgeMetadata};

impl Bus {
    pub fn rom_title(&self) -> &str {
        self.cartridge.title()
    }

    pub fn serial_output(&self) -> &str {
        &self.serial.output
    }

    pub fn frame_counter(&self) -> u64 {
        self.ppu.frame_counter
    }

    pub fn framebuffer(&self) -> &[u8; super::LCD_FRAME_PIXELS] {
        &self.framebuffer
    }

    pub fn flush_battery_save(&mut self) -> Result<(), CartridgeError> {
        self.cartridge.flush_save()
    }

    pub fn cartridge_metadata(&self) -> CartridgeMetadata {
        self.cartridge.metadata()
    }

    pub fn cartridge_has_rumble(&self) -> bool {
        self.cartridge.has_rumble()
    }

    pub fn rumble_active(&self) -> bool {
        self.cartridge.rumble_active()
    }
}
