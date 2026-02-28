use super::WebEmulator;
use gb_emu::bootrom::parse_boot_rom_prefix;
use gb_emu::cartridge::Cartridge;
use gb_emu::hardware::HardwareModel;
use gb_runtime::audio_queue::AudioQueueRefillConfig;
use gb_runtime::session::RuntimeSession;
use std::time::Duration;
use wasm_bindgen::prelude::*;

impl WebEmulator {
    pub(super) fn new_internal(
        rom_bytes: &[u8],
        model: Option<&str>,
        boot_rom_bytes: Option<&[u8]>,
    ) -> Result<WebEmulator, String> {
        let model = match model {
            Some(value) => value.parse::<HardwareModel>()?,
            None => HardwareModel::default(),
        };

        let initial_rtc_epoch_secs = host_wall_clock_epoch_secs();
        let cartridge = Cartridge::from_bytes_with_initial_rtc_epoch(
            rom_bytes.to_vec(),
            initial_rtc_epoch_secs,
        )
        .map_err(|err| err.to_string())?;

        let boot_rom = match boot_rom_bytes {
            Some(bytes) => Some(
                parse_boot_rom_prefix(bytes)
                    .ok_or_else(|| "Boot ROM must contain at least 256 bytes".to_string())?,
            ),
            None => None,
        };

        let mut gb =
            gb_emu::gameboy::GameBoy::new_with_model_and_boot_rom(cartridge, model, boot_rom);
        gb.set_cartridge_host_rtc_epoch_secs(Some(initial_rtc_epoch_secs));
        let session = RuntimeSession::new(gb, 48_000);
        let audio_queue_controller = gb_runtime::audio_queue::AudioQueueController::new(
            48_000,
            0,
            AudioQueueRefillConfig::default(),
        );

        Ok(Self {
            session,
            audio_queue_controller,
            audio_queue_clock_ms: 0,
        })
    }
}

fn host_wall_clock_epoch_secs() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        let millis = js_sys::Date::now();
        if !millis.is_finite() || millis.is_sign_negative() {
            return 0;
        }
        (millis / 1000.0) as u64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(rom_bytes: &[u8], model: Option<String>) -> Result<WebEmulator, JsValue> {
        Self::new_internal(rom_bytes, model.as_deref(), None)
            .map_err(|message| JsValue::from_str(&message))
    }

    #[wasm_bindgen(js_name = newWithBootRom)]
    pub fn new_with_boot_rom(
        rom_bytes: &[u8],
        model: Option<String>,
        boot_rom_bytes: &[u8],
    ) -> Result<WebEmulator, JsValue> {
        Self::new_internal(rom_bytes, model.as_deref(), Some(boot_rom_bytes))
            .map_err(|message| JsValue::from_str(&message))
    }

    pub fn frame_counter(&self) -> u64 {
        self.session.gameboy().frame_counter()
    }

    pub fn run_frame(&mut self) -> Result<u64, JsValue> {
        self.run_frame_and_capture_audio()
    }

    pub fn run_for_elapsed_micros(&mut self, elapsed_micros: u32) -> Result<u32, JsValue> {
        self.session
            .push_host_time(Duration::from_micros(elapsed_micros as u64));

        let mut ran_frames = 0u32;
        while self.session.has_frame_budget() {
            self.run_frame_and_capture_audio()?;
            ran_frames = ran_frames.saturating_add(1);
        }

        Ok(ran_frames)
    }

    pub fn pending_frame_budget(&self) -> u32 {
        self.session.frame_budget_count()
    }

    pub fn audio_clock_tcycles(&self) -> u64 {
        self.session.audio_clock_tcycles()
    }

    pub fn drain_audio_tcycles(&mut self) -> u64 {
        self.session.drain_audio_tcycles()
    }

    pub fn set_host_rtc_epoch_secs(&mut self, epoch_secs: f64) {
        if !epoch_secs.is_finite() || epoch_secs.is_sign_negative() {
            return;
        }
        self.session
            .gameboy_mut()
            .set_cartridge_host_rtc_epoch_secs(Some(epoch_secs.floor() as u64));
    }
}
