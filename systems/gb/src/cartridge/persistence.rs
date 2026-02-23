use super::{Cartridge, Mbc3Rtc};

impl Cartridge {
    pub fn export_save_ram_bytes(&self) -> Option<Vec<u8>> {
        if self.ram.is_empty() || !self.has_battery {
            return None;
        }
        Some(self.ram.clone())
    }

    pub fn import_save_ram_bytes(&mut self, data: &[u8]) {
        let copy_len = self.ram.len().min(data.len());
        if copy_len > 0 {
            self.ram[..copy_len].copy_from_slice(&data[..copy_len]);
        }
    }

    pub fn export_rtc_persistence_bytes(&mut self) -> Option<Vec<u8>> {
        if !self.has_timer {
            return None;
        }
        let rtc = self.rtc.as_mut()?;
        let now_epoch_secs = self.clock.now_epoch_secs();
        Some(rtc.serialize(now_epoch_secs).to_vec())
    }

    pub fn import_rtc_persistence_bytes(&mut self, data: &[u8]) -> bool {
        let Some(rtc) = Mbc3Rtc::deserialize(data) else {
            return false;
        };
        self.rtc = Some(rtc);
        true
    }

    pub fn battery_save_dirty(&self) -> bool {
        self.save_dirty
    }

    pub fn mark_persistence_clean(&mut self) {
        self.save_dirty = false;
    }
}
