use super::Bus;
use crate::cartridge::{CartridgeCapabilities, CartridgeMetadata, CartridgeModelCompatibility};

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

    pub fn cartridge_battery_save_dirty(&self) -> bool {
        self.cartridge.battery_save_dirty()
    }

    pub fn export_cartridge_save_ram_bytes(&self) -> Option<Vec<u8>> {
        self.cartridge.export_save_ram_bytes()
    }

    pub fn import_cartridge_save_ram_bytes(&mut self, data: &[u8]) {
        self.cartridge.import_save_ram_bytes(data);
    }

    pub fn export_cartridge_rtc_persistence_bytes(&mut self) -> Option<Vec<u8>> {
        self.cartridge.export_rtc_persistence_bytes()
    }

    pub fn import_cartridge_rtc_persistence_bytes(&mut self, data: &[u8]) -> bool {
        self.cartridge.import_rtc_persistence_bytes(data)
    }

    pub fn set_cartridge_host_rtc_epoch_secs(&mut self, epoch_secs: Option<u64>) {
        self.cartridge.set_host_rtc_epoch_secs(epoch_secs);
    }

    pub fn mark_cartridge_persistence_clean(&mut self) {
        self.cartridge.mark_persistence_clean();
    }

    pub fn cartridge_metadata(&self) -> CartridgeMetadata {
        self.cartridge.metadata()
    }

    pub(crate) fn cartridge_capabilities(&self) -> CartridgeCapabilities {
        self.cartridge.capabilities()
    }

    pub fn cartridge_model_compatibility(&self) -> CartridgeModelCompatibility {
        self.cartridge.compatibility_for_model(self.hardware_model)
    }

    pub fn cartridge_has_rumble(&self) -> bool {
        self.cartridge_capabilities().has_rumble
    }

    pub fn rumble_active(&self) -> bool {
        self.cartridge.rumble_active()
    }
}
