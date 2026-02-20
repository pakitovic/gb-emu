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

        let mut gb = GameBoy::new_with_model(cartridge, model);
        gb.set_audio_tcycle_stream_enabled(true);

        let mut audio_mixer = AudioMixer::new(48_000);
        audio_mixer.set_source(MixerSource::CoreApu);

        Ok(Self {
            gb,
            pacer: FramePacer::default(),
            audio_mixer,
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
        let tcycle_samples = self.gb.drain_audio_tcycle_samples();
        self.audio_mixer.push_core_tcycle_samples(&tcycle_samples);
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
            let tcycle_samples = self.gb.drain_audio_tcycle_samples();
            self.audio_mixer.push_core_tcycle_samples(&tcycle_samples);
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
            MixerSource::CoreApu
        });
    }

    pub fn drain_audio_samples(&mut self, max_samples: u32) -> Vec<f32> {
        let pending_tcycles = self.pacer.drain_audio_tcycles();
        self.audio_mixer
            .drain_synced_samples(pending_tcycles, max_samples as usize)
    }

    pub fn drain_audio_samples_realtime(&mut self, block_samples: u32) -> Vec<f32> {
        let pending_tcycles = self.pacer.drain_audio_tcycles();
        self.audio_mixer
            .drain_realtime_block(pending_tcycles, block_samples as usize)
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

    pub fn cartridge_debug_report(&self) -> String {
        self.gb.cartridge_metadata().debug_report()
    }

    pub fn cartridge_warning_count(&self) -> u32 {
        self.gb.cartridge_metadata().header_warnings.len() as u32
    }

    pub fn set_button(&mut self, button: u8, pressed: bool) -> Result<(), JsValue> {
        let button = Button::from_index(button)
            .ok_or_else(|| JsValue::from_str("Invalid button index (expected 0..7)"))?;
        self.gb.set_button_pressed(button, pressed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rom_32kb() -> Vec<u8> {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00; // ROM-only
        rom[0x0148] = 0x00; // 32KB
        rom
    }

    #[test]
    fn drain_audio_samples_realtime_returns_fixed_block_len() {
        let rom = make_rom_32kb();
        let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
        web.run_frame().expect("a frame should be produced");

        let samples = web.drain_audio_samples_realtime(512);
        assert_eq!(samples.len(), 512);
    }

    #[test]
    fn drain_audio_samples_realtime_can_emit_core_apu_signal() {
        let rom = make_rom_32kb();
        let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");

        web.gb.bus.write_byte(0xFF26, 0x00);
        web.gb.bus.write_byte(0xFF26, 0x80);
        web.gb.bus.write_byte(0xFF24, 0x77);
        web.gb.bus.write_byte(0xFF25, 0x11);
        web.gb.bus.write_byte(0xFF11, 0x80);
        web.gb.bus.write_byte(0xFF12, 0xF0);
        web.gb.bus.write_byte(0xFF13, 0xFC);
        web.gb.bus.write_byte(0xFF14, 0x87);

        web.run_frame().expect("a frame should be produced");
        let samples = web.drain_audio_samples_realtime(512);
        assert_eq!(samples.len(), 512);
        assert!(samples.iter().any(|sample| *sample != 0.0));
    }

    #[test]
    fn set_button_accepts_valid_index() {
        let rom = make_rom_32kb();
        let mut web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
        assert!(web.set_button(4, true).is_ok());
        assert!(web.set_button(4, false).is_ok());
    }

    #[test]
    fn cartridge_debug_report_exposes_metadata_summary() {
        let rom = make_rom_32kb();
        let web = WebEmulator::new(&rom, None).expect("web emulator should initialize");
        let report = web.cartridge_debug_report();

        assert!(report.contains("Cartridge Metadata"));
        assert!(report.contains("Type: 0x00 (ROM-only)"));
        assert!(report.contains("Header warnings"));
        assert!(report.contains("Nintendo logo mismatch"));
        assert!(web.cartridge_warning_count() >= 1);
    }
}
