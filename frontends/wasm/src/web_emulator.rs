use gb_runtime::audio_queue::AudioQueueController;
use gb_runtime::session::RuntimeSession;
use wasm_bindgen::prelude::*;

mod audio;
mod core;
mod debug;
mod input;
mod persistence;
mod video;

const FRAME_STEP_LIMIT: usize = 250_000;

#[wasm_bindgen]
pub struct WebEmulator {
    session: RuntimeSession,
    audio_queue_controller: AudioQueueController,
    audio_queue_clock_ms: u64,
}

impl WebEmulator {
    fn run_frame_and_capture_audio(&mut self) -> Result<u64, JsValue> {
        self.session
            .run_frame_with_limit(FRAME_STEP_LIMIT)
            .ok_or_else(|| {
                JsValue::from_str("PPU frame was not produced within the web frame step budget")
            })
    }
}

#[cfg(test)]
mod tests;
