use super::WebEmulator;
use gb_emu::input::Button;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WebEmulator {
    pub fn set_button(&mut self, button: u8, pressed: bool) -> Result<(), JsValue> {
        let button = Button::from_index(button)
            .ok_or_else(|| JsValue::from_str("Invalid button index (expected 0..7)"))?;
        self.session
            .gameboy_mut()
            .set_button_pressed(button, pressed);
        Ok(())
    }

    pub fn set_player_button(
        &mut self,
        player_index: u8,
        button: u8,
        pressed: bool,
    ) -> Result<(), JsValue> {
        let button = Button::from_index(button)
            .ok_or_else(|| JsValue::from_str("Invalid button index (expected 0..7)"))?;
        if self.session.gameboy_mut().set_player_button_pressed(
            player_index as usize,
            button,
            pressed,
        ) {
            return Ok(());
        }

        Err(JsValue::from_str(
            "Invalid player index (expected 0..3 for SGB multiplayer)",
        ))
    }

    pub fn joypad_player_count(&self) -> u8 {
        self.session.gameboy().joypad_player_count()
    }

    pub fn current_joypad_player_index(&self) -> u8 {
        self.session.gameboy().current_joypad_player_index()
    }
}
