use super::WebEmulator;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WebEmulator {
    pub fn cartridge_battery_save_dirty(&self) -> bool {
        self.gb.cartridge_battery_save_dirty()
    }

    pub fn export_cartridge_save_ram_bytes(&self) -> Option<Vec<u8>> {
        self.gb.export_cartridge_save_ram_bytes()
    }

    pub fn import_cartridge_save_ram_bytes(&mut self, data: &[u8]) {
        self.gb.import_cartridge_save_ram_bytes(data);
    }

    pub fn export_cartridge_rtc_persistence_bytes(&mut self) -> Option<Vec<u8>> {
        self.gb.export_cartridge_rtc_persistence_bytes()
    }

    pub fn import_cartridge_rtc_persistence_bytes(&mut self, data: &[u8]) -> bool {
        self.gb.import_cartridge_rtc_persistence_bytes(data)
    }

    pub fn mark_cartridge_persistence_clean(&mut self) {
        self.gb.mark_cartridge_persistence_clean();
    }
}
