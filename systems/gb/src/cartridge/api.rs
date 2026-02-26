use super::{Cartridge, CartridgeHeaderWarning, CartridgeMetadata};

impl Cartridge {
    pub fn set_host_rtc_epoch_secs(&mut self, epoch_secs: Option<u64>) {
        self.host_rtc_epoch_secs = epoch_secs;
    }

    pub fn has_battery_save(&self) -> bool {
        self.has_battery && (!self.ram.is_empty() || self.has_timer)
    }

    pub fn metadata(&self) -> CartridgeMetadata {
        let capabilities = self.capabilities();
        CartridgeMetadata {
            title: self.title.clone(),
            cart_type_code: self.cart_type_code,
            mapper: capabilities.mapper,
            rom_size_code: self.rom_size_code,
            ram_size_code: self.ram_size_code,
            rom_size_bytes: self.rom.len(),
            rom_bank_count: self.rom_bank_count,
            declared_ram_size_bytes: self.declared_ram_size_bytes,
            effective_ram_size_bytes: self.ram.len(),
            ram_bank_count: self.ram_bank_count,
            compatibility_ram_mode: capabilities.compatibility_ram_mode,
            has_battery: capabilities.has_battery,
            has_timer: capabilities.has_timer,
            has_rumble: capabilities.has_rumble,
            has_battery_save: capabilities.has_battery_save,
            rumble_active: self.rumble_active(),
            header_warnings: self.header_warnings.clone(),
        }
    }

    pub fn header_warnings(&self) -> &[CartridgeHeaderWarning] {
        &self.header_warnings
    }

    pub fn has_rumble(&self) -> bool {
        self.has_rumble
    }

    pub fn rumble_active(&self) -> bool {
        self.has_rumble && self.rumble_active
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}
