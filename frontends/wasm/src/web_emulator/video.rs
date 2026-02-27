use super::WebEmulator;
use gb_emu::gameboy::{SCREEN_HEIGHT, SCREEN_WIDTH};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WebEmulator {
    pub fn screen_width(&self) -> u32 {
        SCREEN_WIDTH as u32
    }

    pub fn screen_height(&self) -> u32 {
        SCREEN_HEIGHT as u32
    }

    pub fn grayscale_frame(&self) -> Vec<u8> {
        self.session.gameboy().framebuffer().to_vec()
    }
}
