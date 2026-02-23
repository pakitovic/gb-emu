use super::WebEmulator;
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
        self.gb.cartridge_metadata().debug_report()
    }

    pub fn cartridge_warning_count(&self) -> u32 {
        self.gb.cartridge_metadata().header_warnings.len() as u32
    }
}
