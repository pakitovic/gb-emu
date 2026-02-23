use crate::audio::AnalogCalibrationProfile;
use crate::cartridge::Cartridge;
use crate::cartridge::CartridgeError;
use crate::cartridge::CartridgeMetadata;
use crate::cpu::Cpu;
use crate::hardware::HardwareModel;
use crate::input::Button;
use crate::memory::Bus;

pub const SCREEN_WIDTH: usize = crate::memory::LCD_WIDTH;
pub const SCREEN_HEIGHT: usize = crate::memory::LCD_HEIGHT;

pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new(cartridge: Cartridge) -> Self {
        Self::new_with_model(cartridge, HardwareModel::default())
    }

    pub fn new_with_model(cartridge: Cartridge, model: HardwareModel) -> Self {
        Self {
            cpu: Cpu::new_with_model(model),
            bus: Bus::new_with_model(cartridge, model),
        }
    }

    // Execute one CPU step
    pub fn step(&mut self) -> u8 {
        self.cpu.step(&mut self.bus)
    }

    pub fn rom_title(&self) -> &str {
        self.bus.rom_title()
    }

    pub fn serial_output(&self) -> &str {
        self.bus.serial_output()
    }

    pub fn frame_counter(&self) -> u64 {
        self.bus.frame_counter()
    }

    pub fn framebuffer(&self) -> &[u8; SCREEN_WIDTH * SCREEN_HEIGHT] {
        self.bus.framebuffer()
    }

    pub fn flush_battery_save(&mut self) -> Result<(), CartridgeError> {
        self.bus.flush_battery_save()
    }

    pub fn cartridge_metadata(&self) -> CartridgeMetadata {
        self.bus.cartridge_metadata()
    }

    pub fn cartridge_has_rumble(&self) -> bool {
        self.bus.cartridge_has_rumble()
    }

    pub fn rumble_active(&self) -> bool {
        self.bus.rumble_active()
    }

    pub fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        self.bus.drain_audio_tcycle_samples()
    }

    pub fn set_audio_tcycle_stream_enabled(&mut self, enabled: bool) {
        self.bus.set_audio_tcycle_stream_enabled(enabled);
    }

    pub fn set_audio_analog_calibration(&mut self, calibration: AnalogCalibrationProfile) {
        self.bus.set_apu_analog_calibration(calibration);
    }

    pub fn run_frame_with_limit(&mut self, max_steps: usize) -> Option<u64> {
        let start_frame = self.frame_counter();
        let mut total_cycles = 0u64;
        for _ in 0..max_steps {
            let cycles = self.step();
            total_cycles = total_cycles.wrapping_add(cycles as u64);
            if self.frame_counter() != start_frame {
                return Some(total_cycles);
            }
        }
        None
    }

    pub fn set_button_pressed(&mut self, button: Button, pressed: bool) {
        self.bus.set_button_pressed(button, pressed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;

    fn make_rom_32kb() -> Vec<u8> {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00; // ROM-only
        rom[0x0148] = 0x00; // 32KB
        rom
    }

    #[test]
    fn run_frame_with_limit_returns_none_if_budget_is_too_small() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let mut gb = GameBoy::new(cartridge);
        let start = gb.frame_counter();

        let result = gb.run_frame_with_limit(1);

        assert!(result.is_none());
        assert_eq!(gb.frame_counter(), start);
    }

    #[test]
    fn run_frame_with_limit_advances_frame_counter() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let mut gb = GameBoy::new(cartridge);
        let start = gb.frame_counter();

        let cycles = gb
            .run_frame_with_limit(50_000)
            .expect("frame should be produced within step budget");

        assert!(cycles > 0);
        assert!(gb.frame_counter() > start);
    }
}
