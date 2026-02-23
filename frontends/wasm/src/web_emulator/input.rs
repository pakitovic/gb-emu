use super::WebEmulator;
use gb_emu::input::Button;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WebEmulator {
    pub fn set_button(&mut self, button: u8, pressed: bool) -> Result<(), JsValue> {
        let button = Button::from_index(button)
            .ok_or_else(|| JsValue::from_str("Invalid button index (expected 0..7)"))?;
        self.gb.set_button_pressed(button, pressed);
        Ok(())
    }
}
