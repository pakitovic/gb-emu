use super::WebEmulator;
use gb_emu::cartridge::Cartridge;
use gb_emu::hardware::HardwareModel;
use gb_runtime::audio::{AudioMixer, MixerSource};
use std::time::Duration;
use wasm_bindgen::prelude::*;

impl WebEmulator {
    pub(super) fn new_internal(
        rom_bytes: &[u8],
        model: Option<&str>,
    ) -> Result<WebEmulator, String> {
        let model = match model {
            Some(value) => value.parse::<HardwareModel>()?,
            None => HardwareModel::default(),
        };

        let cartridge = Cartridge::from_bytes(rom_bytes.to_vec()).map_err(|err| err.to_string())?;

        let mut gb = gb_emu::gameboy::GameBoy::new_with_model(cartridge, model);
        gb.set_audio_tcycle_stream_enabled(true);

        let mut audio_mixer = AudioMixer::new(48_000);
        audio_mixer.set_source(MixerSource::CoreApu);

        Ok(Self {
            gb,
            pacer: gb_runtime::timing::FramePacer::default(),
            audio_mixer,
        })
    }
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(rom_bytes: &[u8], model: Option<String>) -> Result<WebEmulator, JsValue> {
        Self::new_internal(rom_bytes, model.as_deref())
            .map_err(|message| JsValue::from_str(&message))
    }

    pub fn frame_counter(&self) -> u64 {
        self.gb.frame_counter()
    }

    pub fn run_frame(&mut self) -> Result<u64, JsValue> {
        self.run_frame_and_capture_audio()
    }

    pub fn run_for_elapsed_micros(&mut self, elapsed_micros: u32) -> Result<u32, JsValue> {
        self.pacer
            .push_host_time(Duration::from_micros(elapsed_micros as u64));

        let mut ran_frames = 0u32;
        while self.pacer.has_frame_budget() {
            self.run_frame_and_capture_audio()?;
            ran_frames = ran_frames.saturating_add(1);
        }

        Ok(ran_frames)
    }

    pub fn pending_frame_budget(&self) -> u32 {
        self.pacer.frame_budget_count()
    }

    pub fn audio_clock_tcycles(&self) -> u64 {
        self.pacer.audio_clock_tcycles()
    }

    pub fn drain_audio_tcycles(&mut self) -> u64 {
        self.pacer.drain_audio_tcycles()
    }
}
