use super::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::audio::AnalogCalibrationProfile;
use crate::cartridge::{CartridgeMetadata, CartridgeModelCompatibility};
use crate::cpu::Cpu;
use crate::hardware::HardwareModel;
use crate::input::Button;
use crate::memory::KeyMmioWriteEvent;

impl GameBoy {
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
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

    pub fn hardware_model(&self) -> HardwareModel {
        self.bus.hardware_model()
    }

    pub fn recent_pc_trace(&self) -> Vec<u16> {
        let mut trace = Vec::with_capacity(self.recent_pc_trace_len);
        let start = if self.recent_pc_trace_len == self.recent_pc_trace.len() {
            self.recent_pc_trace_head
        } else {
            0
        };

        for i in 0..self.recent_pc_trace_len {
            let idx = (start + i) % self.recent_pc_trace.len();
            trace.push(self.recent_pc_trace[idx]);
        }

        trace
    }

    pub fn recent_key_mmio_writes(&self) -> Vec<(u16, u8)> {
        self.bus.recent_key_mmio_writes()
    }

    pub fn drain_key_mmio_write_events(&mut self) -> Vec<KeyMmioWriteEvent> {
        self.bus.drain_key_mmio_write_events()
    }

    pub fn framebuffer(&self) -> &[u8; SCREEN_WIDTH * SCREEN_HEIGHT] {
        self.bus.framebuffer()
    }

    pub fn framebuffer_palette_selectors(&self) -> &[u8; SCREEN_WIDTH * SCREEN_HEIGHT] {
        self.bus.framebuffer_palette_selectors()
    }

    pub fn rom_header_crc32(&self) -> u32 {
        self.bus.rom_header_crc32()
    }

    pub fn copy_vram_hardware_block(&self, start_addr: u16, dst: &mut [u8]) -> bool {
        self.bus.copy_vram_hardware_block(start_addr, dst)
    }

    pub fn cartridge_battery_save_dirty(&self) -> bool {
        self.bus.cartridge_battery_save_dirty()
    }

    pub fn export_cartridge_save_ram_bytes(&self) -> Option<Vec<u8>> {
        self.bus.export_cartridge_save_ram_bytes()
    }

    pub fn import_cartridge_save_ram_bytes(&mut self, data: &[u8]) {
        self.bus.import_cartridge_save_ram_bytes(data);
    }

    pub fn export_cartridge_rtc_persistence_bytes(&mut self) -> Option<Vec<u8>> {
        self.bus.export_cartridge_rtc_persistence_bytes()
    }

    pub fn import_cartridge_rtc_persistence_bytes(&mut self, data: &[u8]) -> bool {
        self.bus.import_cartridge_rtc_persistence_bytes(data)
    }

    pub fn set_cartridge_host_rtc_epoch_secs(&mut self, epoch_secs: Option<u64>) {
        self.bus.set_cartridge_host_rtc_epoch_secs(epoch_secs);
    }

    pub fn mark_cartridge_persistence_clean(&mut self) {
        self.bus.mark_cartridge_persistence_clean();
    }

    pub fn cartridge_metadata(&self) -> CartridgeMetadata {
        self.bus.cartridge_metadata()
    }

    pub fn cartridge_model_compatibility(&self) -> CartridgeModelCompatibility {
        self.bus.cartridge_model_compatibility()
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

    pub fn set_button_pressed(&mut self, button: Button, pressed: bool) {
        self.bus.set_button_pressed(button, pressed);
    }

    pub fn set_player_button_pressed(
        &mut self,
        player_index: usize,
        button: Button,
        pressed: bool,
    ) -> bool {
        self.bus
            .set_player_button_pressed(player_index, button, pressed)
    }

    pub fn joypad_player_count(&self) -> u8 {
        self.bus.joypad_player_count()
    }

    pub fn current_joypad_player_index(&self) -> u8 {
        self.bus.current_joypad_player_index()
    }
}
