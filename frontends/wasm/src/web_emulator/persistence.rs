use super::WebEmulator;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WebEmulator {
    pub fn cartridge_battery_save_dirty(&self) -> bool {
        self.session.gameboy().cartridge_battery_save_dirty()
    }

    pub fn export_cartridge_save_ram_bytes(&self) -> Option<Vec<u8>> {
        self.session.gameboy().export_cartridge_save_ram_bytes()
    }

    pub fn import_cartridge_save_ram_bytes(&mut self, data: &[u8]) {
        self.session
            .gameboy_mut()
            .import_cartridge_save_ram_bytes(data);
    }

    pub fn export_cartridge_rtc_persistence_bytes(&mut self) -> Option<Vec<u8>> {
        self.session
            .gameboy_mut()
            .export_cartridge_rtc_persistence_bytes()
    }

    pub fn import_cartridge_rtc_persistence_bytes(&mut self, data: &[u8]) -> bool {
        self.session
            .gameboy_mut()
            .import_cartridge_rtc_persistence_bytes(data)
    }

    pub fn mark_cartridge_persistence_clean(&mut self) {
        self.session
            .gameboy_mut()
            .mark_cartridge_persistence_clean();
    }
}
