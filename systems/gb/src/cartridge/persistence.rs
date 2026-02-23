use super::clock::{RtcClock, SystemRtcClock};
use super::{Cartridge, CartridgeError, Mbc3Rtc};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SAVE_FILE_EXTENSION: &str = "sav";
const RTC_FILE_EXTENSION: &str = "rtc";

impl Cartridge {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CartridgeError> {
        Self::from_file_with_clock(path, Box::new(SystemRtcClock))
    }

    fn from_file_with_clock(
        path: impl AsRef<Path>,
        clock: Box<dyn RtcClock>,
    ) -> Result<Self, CartridgeError> {
        let path_ref = path.as_ref();
        let rom = fs::read(path_ref).map_err(CartridgeError::Io)?;
        let mut cartridge = Self::from_bytes_with_clock(rom, clock)?;
        cartridge.attach_save_from_rom_path(path_ref)?;
        Ok(cartridge)
    }

    pub fn flush_save(&mut self) -> Result<(), CartridgeError> {
        if !self.has_battery {
            return Ok(());
        }

        if self.save_dirty
            && let Some(path) = self.save_path.as_ref()
            && let Some(ram_bytes) = self.export_save_ram_bytes()
        {
            write_file_atomic(path, &ram_bytes).map_err(CartridgeError::SaveIo)?;
            self.mark_persistence_clean();
        }

        if let Some(rtc_bytes) = self.export_rtc_persistence_bytes()
            && let Some(path) = self.rtc_path.as_ref()
        {
            write_file_atomic(path, &rtc_bytes).map_err(CartridgeError::SaveIo)?;
        }

        Ok(())
    }

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

    fn attach_save_from_rom_path(&mut self, rom_path: &Path) -> Result<(), CartridgeError> {
        if !self.has_battery_save() {
            return Ok(());
        }

        if !self.ram.is_empty() {
            let save_path = rom_path.with_extension(SAVE_FILE_EXTENSION);
            match fs::read(&save_path) {
                Ok(data) => self.import_save_ram_bytes(&data),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(CartridgeError::SaveIo(err)),
            }
            self.save_path = Some(save_path);
        }

        if self.has_timer {
            let rtc_path = rom_path.with_extension(RTC_FILE_EXTENSION);
            match fs::read(&rtc_path) {
                Ok(data) => {
                    let _ = self.import_rtc_persistence_bytes(&data);
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(CartridgeError::SaveIo(err)),
            }
            self.rtc_path = Some(rtc_path);
        }

        self.mark_persistence_clean();
        Ok(())
    }

    fn mark_persistence_clean(&mut self) {
        self.save_dirty = false;
    }
}

pub(super) fn write_file_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut attempt = 0u32;
    loop {
        let temp_path = atomic_temp_path(path, attempt);
        attempt = attempt.saturating_add(1);

        let open_result = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path);
        let mut file = match open_result {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        };

        let write_result = (|| {
            file.write_all(data)?;
            file.sync_all()?;
            drop(file);
            match fs::rename(&temp_path, path) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    fs::remove_file(path)?;
                    fs::rename(&temp_path, path)
                }
                Err(err) => Err(err),
            }
        })();

        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }

        return write_result;
    }
}

fn atomic_temp_path(path: &Path, attempt: u32) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let base_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("save");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    parent.join(format!(".{base_name}.tmp.{pid}.{nanos}.{attempt}"))
}
