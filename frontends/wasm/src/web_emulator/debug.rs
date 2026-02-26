use super::WebEmulator;
use gb_runtime::cartridge_debug::format_cartridge_debug_report;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WebEmulator {
    pub fn serial_output(&self) -> String {
        self.gb.serial_output().to_string()
    }

    pub fn rom_title(&self) -> String {
        self.gb.rom_title().to_string()
    }

    pub fn cartridge_debug_report(&self) -> String {
        format_cartridge_debug_report(&self.gb.cartridge_metadata())
    }

    pub fn cartridge_warning_count(&self) -> u32 {
        self.gb.cartridge_metadata().header_warnings.len() as u32
    }

    pub fn cartridge_has_battery_save(&self) -> bool {
        let metadata = self.gb.cartridge_metadata();
        metadata.has_battery && metadata.effective_ram_size_bytes > 0
    }

    pub fn cartridge_has_rtc_persistence(&self) -> bool {
        let metadata = self.gb.cartridge_metadata();
        metadata.has_battery && metadata.has_timer
    }
}
