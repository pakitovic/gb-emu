use crate::audio::{AudioMixer, MixerSource};
use crate::cartridge::Cartridge;
use crate::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::hardware::HardwareModel;
use crate::input::Button;
use crate::timing::FramePacer;
use std::time::Duration;
use wasm_bindgen::prelude::*;

const FRAME_STEP_LIMIT: usize = 250_000;

#[wasm_bindgen]
pub struct WebEmulator {
    gb: GameBoy,
    pacer: FramePacer,
    audio_mixer: AudioMixer,
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
            pacer: FramePacer::default(),
            audio_mixer: AudioMixer::new(48_000),
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
        let cycles = self
            .gb
            .run_frame_with_limit(false, FRAME_STEP_LIMIT)
            .ok_or_else(|| {
                JsValue::from_str("PPU frame was not produced within the web frame step budget")
            })?;
        self.pacer.consume_emulated_cycles(cycles);
        Ok(cycles)
    }

    pub fn run_for_elapsed_micros(&mut self, elapsed_micros: u32) -> Result<u32, JsValue> {
        self.pacer
            .push_host_time(Duration::from_micros(elapsed_micros as u64));

        let mut ran_frames = 0u32;
        while self.pacer.has_frame_budget() {
            let cycles = self
                .gb
                .run_frame_with_limit(false, FRAME_STEP_LIMIT)
                .ok_or_else(|| {
                    JsValue::from_str("PPU frame was not produced within the web frame step budget")
                })?;
            self.pacer.consume_emulated_cycles(cycles);
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

    pub fn set_audio_sample_rate(&mut self, sample_rate_hz: u32) {
        let source = self.audio_mixer.source();
        self.audio_mixer = AudioMixer::new(sample_rate_hz.max(1));
        self.audio_mixer.set_source(source);
    }

    pub fn set_audio_test_tone_enabled(&mut self, enabled: bool) {
        self.audio_mixer.set_source(if enabled {
            MixerSource::TestTone
        } else {
            MixerSource::Silence
        });
    }

    pub fn drain_audio_samples(&mut self, max_samples: u32) -> Vec<f32> {
        let pending_tcycles = self.pacer.drain_audio_tcycles();
        self.audio_mixer.push_tcycles(pending_tcycles);
        self.audio_mixer.drain_samples(max_samples as usize)
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
