use super::bus_access::SegmentAccess;
use super::{Bus, KEY_MMIO_WRITE_EVENT_QUEUE_CAPACITY, KeyMmioWriteEvent};
use crate::cartridge::{CartridgeCapabilities, CartridgeMetadata, CartridgeModelCompatibility};
use crate::hardware::HardwareModel;

impl Bus {
    pub fn rom_title(&self) -> &str {
        self.cartridge.title()
    }

    pub fn serial_output(&self) -> &str {
        &self.serial.output
    }

    pub fn frame_counter(&self) -> u64 {
        self.ppu.frame_counter
    }

    pub fn hardware_model(&self) -> HardwareModel {
        self.hardware_model
    }

    pub fn recent_key_mmio_writes(&self) -> Vec<(u16, u8)> {
        let mut writes = Vec::with_capacity(self.recent_key_mmio_writes_len);
        let start = if self.recent_key_mmio_writes_len == self.recent_key_mmio_writes.len() {
            self.recent_key_mmio_writes_head
        } else {
            0
        };

        for i in 0..self.recent_key_mmio_writes_len {
            let idx = (start + i) % self.recent_key_mmio_writes.len();
            writes.push(self.recent_key_mmio_writes[idx]);
        }

        writes
    }

    pub fn drain_key_mmio_write_events(&mut self) -> Vec<KeyMmioWriteEvent> {
        self.key_mmio_write_events.drain(..).collect()
    }

    pub(in crate::memory) fn record_key_mmio_write(&mut self, addr: u16, value: u8) {
        self.recent_key_mmio_writes[self.recent_key_mmio_writes_head] = (addr, value);
        self.recent_key_mmio_writes_head =
            (self.recent_key_mmio_writes_head + 1) % self.recent_key_mmio_writes.len();
        if self.recent_key_mmio_writes_len < self.recent_key_mmio_writes.len() {
            self.recent_key_mmio_writes_len += 1;
        }

        if self.key_mmio_write_events.len() >= KEY_MMIO_WRITE_EVENT_QUEUE_CAPACITY {
            let _ = self.key_mmio_write_events.pop_front();
        }
        self.key_mmio_write_events.push_back(KeyMmioWriteEvent {
            tcycle: self.emulated_tcycles,
            addr,
            value,
        });
    }

    pub fn framebuffer(&self) -> &[u8; super::LCD_FRAME_PIXELS] {
        &self.framebuffer
    }

    pub fn framebuffer_palette_selectors(&self) -> &[u8; super::LCD_FRAME_PIXELS] {
        &self.framebuffer_palette_selectors
    }

    pub fn rom_header_crc32(&self) -> u32 {
        self.cartridge.header_crc32()
    }

    pub fn copy_vram_hardware_block(&self, start_addr: u16, dst: &mut [u8]) -> bool {
        let Some(end_addr) = start_addr.checked_add(dst.len() as u16) else {
            return false;
        };
        if !(0x8000..=0xA000).contains(&start_addr) || end_addr > 0xA000 {
            return false;
        }

        for (offset, slot) in dst.iter_mut().enumerate() {
            *slot = self.read_vram(start_addr + offset as u16, SegmentAccess::Hardware);
        }

        true
    }

    pub fn cartridge_battery_save_dirty(&self) -> bool {
        self.cartridge.battery_save_dirty()
    }

    pub fn export_cartridge_save_ram_bytes(&self) -> Option<Vec<u8>> {
        self.cartridge.export_save_ram_bytes()
    }

    pub fn import_cartridge_save_ram_bytes(&mut self, data: &[u8]) {
        self.cartridge.import_save_ram_bytes(data);
    }

    pub fn export_cartridge_rtc_persistence_bytes(&mut self) -> Option<Vec<u8>> {
        self.cartridge.export_rtc_persistence_bytes()
    }

    pub fn import_cartridge_rtc_persistence_bytes(&mut self, data: &[u8]) -> bool {
        self.cartridge.import_rtc_persistence_bytes(data)
    }

    pub fn set_cartridge_host_rtc_epoch_secs(&mut self, epoch_secs: Option<u64>) {
        self.cartridge.set_host_rtc_epoch_secs(epoch_secs);
    }

    pub fn mark_cartridge_persistence_clean(&mut self) {
        self.cartridge.mark_persistence_clean();
    }

    pub fn cartridge_metadata(&self) -> CartridgeMetadata {
        self.cartridge.metadata()
    }

    pub(crate) fn cartridge_capabilities(&self) -> CartridgeCapabilities {
        self.cartridge.capabilities()
    }

    pub fn cartridge_model_compatibility(&self) -> CartridgeModelCompatibility {
        self.cartridge.compatibility_for_model(self.hardware_model)
    }

    pub fn cartridge_has_rumble(&self) -> bool {
        self.cartridge_capabilities().has_rumble
    }

    pub fn rumble_active(&self) -> bool {
        self.cartridge.rumble_active()
    }
}
