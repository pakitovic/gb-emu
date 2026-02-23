use gb_emu::gameboy::GameBoy;
use gb_runtime::audio::AudioMixer;
use gb_runtime::timing::FramePacer;
use wasm_bindgen::prelude::*;

mod audio;
mod core;
mod debug;
mod input;
mod video;

const FRAME_STEP_LIMIT: usize = 250_000;

#[wasm_bindgen]
pub struct WebEmulator {
    gb: GameBoy,
    pacer: FramePacer,
    audio_mixer: AudioMixer,
}

impl WebEmulator {
    fn run_frame_and_capture_audio(&mut self) -> Result<u64, JsValue> {
        let cycles = self
            .gb
            .run_frame_with_limit(FRAME_STEP_LIMIT)
            .ok_or_else(|| {
                JsValue::from_str("PPU frame was not produced within the web frame step budget")
            })?;
        self.pacer.consume_emulated_cycles(cycles);
        let tcycle_samples = self.gb.drain_audio_tcycle_samples();
        self.audio_mixer.push_core_tcycle_samples(&tcycle_samples);
        Ok(cycles)
    }
}

#[cfg(test)]
mod tests;
