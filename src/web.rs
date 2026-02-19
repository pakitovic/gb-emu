use crate::cartridge::Cartridge;
use crate::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::hardware::HardwareModel;
use crate::input::Button;
use wasm_bindgen::prelude::*;

const FRAME_STEP_LIMIT: usize = 250_000;

#[wasm_bindgen]
pub struct WebEmulator {
    gb: GameBoy,
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(rom_bytes: &[u8], model: Option<String>) -> Result<WebEmulator, JsValue> {
        let model = match model {
            Some(value) => value
                .parse::<HardwareModel>()
                .map_err(|message| JsValue::from_str(&message))?,
            None => HardwareModel::default(),
        };

        let cartridge = Cartridge::from_bytes(rom_bytes.to_vec())
            .map_err(|err| JsValue::from_str(&err.to_string()))?;

        Ok(Self {
            gb: GameBoy::new_with_model(cartridge, model),
        })
    }

    pub fn screen_width(&self) -> u32 {
        SCREEN_WIDTH as u32
    }

    pub fn screen_height(&self) -> u32 {
        SCREEN_HEIGHT as u32
    }

    pub fn frame_counter(&self) -> u64 {
        self.gb.frame_counter()
    }

    pub fn run_frame(&mut self) -> Result<u64, JsValue> {
        self.gb
            .run_frame_with_limit(false, FRAME_STEP_LIMIT)
            .ok_or_else(|| {
                JsValue::from_str("PPU frame was not produced within the web frame step budget")
            })
    }

    pub fn grayscale_frame(&self) -> Vec<u8> {
        self.gb.framebuffer().to_vec()
    }

    pub fn serial_output(&self) -> String {
        self.gb.serial_output().to_string()
    }

    pub fn rom_title(&self) -> String {
        self.gb.rom_title().to_string()
    }

    pub fn set_button(&mut self, button: u8, pressed: bool) -> Result<(), JsValue> {
        let button = Button::from_index(button)
            .ok_or_else(|| JsValue::from_str("Invalid button index (expected 0..7)"))?;
        self.gb.set_button_pressed(button, pressed);
        Ok(())
    }
}
