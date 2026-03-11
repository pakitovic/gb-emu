use gb_emu::palette_override::PaletteOverrideDb;
use gb_emu::video::VideoPalette;
use gb_runtime::audio_queue::AudioQueueController;
use gb_runtime::session::RuntimeSession;
use wasm_bindgen::prelude::*;

mod audio;
mod core;
mod debug;
mod input;
mod persistence;
mod video;

use self::video::VideoPaletteSelection;

const FRAME_STEP_LIMIT: usize = 250_000;

#[wasm_bindgen]
pub struct WebEmulator {
    session: RuntimeSession,
    audio_queue_controller: AudioQueueController,
    audio_queue_clock_ms: u64,
    default_video_palette: VideoPalette,
    video_palette_selection: VideoPaletteSelection,
    palette_overrides: Option<PaletteOverrideDb>,
}

#[wasm_bindgen(js_name = parsePaletteOverridesIniEntryCount)]
pub fn parse_palette_overrides_ini_entry_count(ini: &str) -> Result<u32, JsValue> {
    PaletteOverrideDb::parse_ini(ini)
        .map(|overrides| overrides.entry_count() as u32)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

impl WebEmulator {
    fn run_frame_and_capture_audio(&mut self) -> Result<u64, JsValue> {
        self.session
            .run_frame_with_limit(FRAME_STEP_LIMIT)
            .ok_or_else(|| {
                let diagnostics = self.session.frame_step_timeout_diagnostics();
                JsValue::from_str(&format!(
                    "PPU frame was not produced within the web frame step budget; {diagnostics}"
                ))
            })
    }
}

#[cfg(test)]
mod tests;
