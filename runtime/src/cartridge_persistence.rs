use gb_emu::cartridge::{Cartridge, CartridgeError};
use gb_emu::gameboy::GameBoy;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SAVE_FILE_EXTENSION: &str = "sav";
const RTC_FILE_EXTENSION: &str = "rtc";

#[derive(Debug, Clone, Default)]
pub struct FileBackedCartridgePersistence {
    save_path: Option<PathBuf>,
    rtc_path: Option<PathBuf>,
}

pub fn load_cartridge_from_file(
    path: impl AsRef<Path>,
) -> Result<(Cartridge, FileBackedCartridgePersistence), CartridgeError> {
    FileBackedCartridgePersistence::load_cartridge_from_file(path)
}

impl FileBackedCartridgePersistence {
    pub fn load_cartridge_from_file(
        path: impl AsRef<Path>,
    ) -> Result<(Cartridge, FileBackedCartridgePersistence), CartridgeError> {
        let path_ref = path.as_ref();
        let rom = fs::read(path_ref).map_err(CartridgeError::Io)?;
        let mut cartridge = Cartridge::from_bytes(rom)?;
        let persistence = Self::from_rom_path(path_ref, &cartridge);
        persistence.attach_persistence_bytes(&mut cartridge)?;
        cartridge.mark_persistence_clean();
        Ok((cartridge, persistence))
    }

    pub fn flush_cartridge(&self, cartridge: &mut Cartridge) -> Result<(), CartridgeError> {
        if cartridge.battery_save_dirty()
            && let Some(path) = self.save_path.as_ref()
            && let Some(ram_bytes) = cartridge.export_save_ram_bytes()
        {
            write_file_atomic(path, &ram_bytes).map_err(CartridgeError::SaveIo)?;
            cartridge.mark_persistence_clean();
        }

        if let Some(path) = self.rtc_path.as_ref()
            && let Some(rtc_bytes) = cartridge.export_rtc_persistence_bytes()
        {
            write_file_atomic(path, &rtc_bytes).map_err(CartridgeError::SaveIo)?;
        }

        Ok(())
    }

    pub fn flush_gameboy(&self, gb: &mut GameBoy) -> Result<(), CartridgeError> {
        if gb.cartridge_battery_save_dirty()
            && let Some(path) = self.save_path.as_ref()
            && let Some(ram_bytes) = gb.export_cartridge_save_ram_bytes()
        {
            write_file_atomic(path, &ram_bytes).map_err(CartridgeError::SaveIo)?;
            gb.mark_cartridge_persistence_clean();
        }

        if let Some(path) = self.rtc_path.as_ref()
            && let Some(rtc_bytes) = gb.export_cartridge_rtc_persistence_bytes()
        {
            write_file_atomic(path, &rtc_bytes).map_err(CartridgeError::SaveIo)?;
        }

        Ok(())
    }

    fn from_rom_path(rom_path: &Path, cartridge: &Cartridge) -> Self {
        let metadata = cartridge.metadata();
        let save_path = (metadata.has_battery && metadata.effective_ram_size_bytes > 0)
            .then(|| rom_path.with_extension(SAVE_FILE_EXTENSION));
        let rtc_path = (metadata.has_battery && metadata.has_timer)
            .then(|| rom_path.with_extension(RTC_FILE_EXTENSION));
        Self {
            save_path,
            rtc_path,
        }
    }

    fn attach_persistence_bytes(&self, cartridge: &mut Cartridge) -> Result<(), CartridgeError> {
        if let Some(save_path) = self.save_path.as_ref() {
            match fs::read(save_path) {
                Ok(data) => cartridge.import_save_ram_bytes(&data),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(CartridgeError::SaveIo(err)),
            }
        }

        if let Some(rtc_path) = self.rtc_path.as_ref() {
            match fs::read(rtc_path) {
                Ok(data) => {
                    let _ = cartridge.import_rtc_persistence_bytes(&data);
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(CartridgeError::SaveIo(err)),
            }
        }

        Ok(())
    }
}

fn write_file_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const ROM_ONLY: u8 = 0x00;
    const MBC1_RAM: u8 = 0x02;
    const MBC1_RAM_BATTERY: u8 = 0x03;
    const MBC2_BATTERY: u8 = 0x06;
    const MBC3_TIMER_BATTERY: u8 = 0x0F;

    fn make_rom(size: usize, cart_type: u8, rom_size_code: u8, ram_size_code: u8) -> Vec<u8> {
        let mut rom = vec![0; size];
        rom[0x0147] = cart_type;
        rom[0x0148] = rom_size_code;
        rom[0x0149] = ram_size_code;
        rom
    }

    fn unique_temp_file_path(name: &str, ext: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("gb_emu_runtime_{name}_{pid}_{nanos}.{ext}"))
    }

    #[test]
    fn persists_battery_backed_ram_to_sav_file() {
        let rom_path = unique_temp_file_path("save_roundtrip", "gb");
        let save_path = rom_path.with_extension("sav");
        let rom = make_rom(64 * 1024, MBC1_RAM_BATTERY, 0x01, 0x02);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let (mut first_load, persistence) =
            load_cartridge_from_file(&rom_path).expect("cartridge should load");
        first_load.write_rom_control(0x0000, 0x0A);
        first_load.write_ram_byte(0xA000, 0x5A);
        persistence
            .flush_cartridge(&mut first_load)
            .expect("flush should persist save");
        assert!(save_path.exists());

        let (mut second_load, _) = load_cartridge_from_file(&rom_path).expect("reload should work");
        second_load.write_rom_control(0x0000, 0x0A);
        assert_eq!(second_load.read_ram_byte(0xA000), 0x5A);

        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(rom_path);
    }

    #[test]
    fn non_battery_carts_do_not_write_save_files() {
        let rom_path = unique_temp_file_path("save_non_battery", "gb");
        let save_path = rom_path.with_extension("sav");
        let rom = make_rom(64 * 1024, MBC1_RAM, 0x01, 0x02);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let (mut cart, persistence) =
            load_cartridge_from_file(&rom_path).expect("load should work");
        cart.write_rom_control(0x0000, 0x0A);
        cart.write_ram_byte(0xA000, 0x33);
        persistence
            .flush_cartridge(&mut cart)
            .expect("flush should not fail");
        assert!(!save_path.exists());

        let _ = fs::remove_file(rom_path);
    }

    #[test]
    fn persists_mbc2_battery_ram_to_sav_file() {
        let rom_path = unique_temp_file_path("mbc2_save", "gb");
        let save_path = rom_path.with_extension("sav");
        let rom = make_rom(64 * 1024, MBC2_BATTERY, 0x01, 0x00);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let (mut first, persistence) =
            load_cartridge_from_file(&rom_path).expect("load should work");
        first.write_rom_control(0x0000, 0x0A);
        first.write_ram_byte(0xA123, 0xA5);
        persistence
            .flush_cartridge(&mut first)
            .expect("flush should persist save");
        assert!(save_path.exists());

        let (mut second, _) = load_cartridge_from_file(&rom_path).expect("reload should work");
        second.write_rom_control(0x0000, 0x0A);
        assert_eq!(second.read_ram_byte(0xA123), 0xF5);

        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(rom_path);
    }

    #[test]
    fn mbc3_timer_battery_persists_rtc_sidecar() {
        let rom_path = unique_temp_file_path("mbc3_timer_rtc", "gb");
        let save_path = rom_path.with_extension("sav");
        let rtc_path = rom_path.with_extension("rtc");
        let rom = make_rom(32 * 1024, MBC3_TIMER_BATTERY, 0x00, 0x00);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let (mut first_load, persistence) =
            load_cartridge_from_file(&rom_path).expect("cartridge should load");
        first_load.write_rom_control(0x0000, 0x0A);
        first_load.write_rom_control(0x4000, 0x0C);
        first_load.write_ram_byte(0xA000, 0x40);
        first_load.write_rom_control(0x4000, 0x08);
        first_load.write_ram_byte(0xA000, 33);
        persistence
            .flush_cartridge(&mut first_load)
            .expect("flush should persist rtc");
        assert!(!save_path.exists());
        assert!(rtc_path.exists());

        let (mut second_load, _) =
            load_cartridge_from_file(&rom_path).expect("cartridge should reload");
        second_load.write_rom_control(0x0000, 0x0A);
        second_load.write_rom_control(0x4000, 0x0C);
        assert_eq!(second_load.read_ram_byte(0xA000) & 0x40, 0x40);
        second_load.write_rom_control(0x4000, 0x08);
        assert_eq!(second_load.read_ram_byte(0xA000), 33);

        let _ = fs::remove_file(rtc_path);
        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(rom_path);
    }

    #[test]
    fn atomic_save_writer_replaces_existing_file_without_temp_leaks() {
        let save_path = unique_temp_file_path("atomic_save_replace", "sav");
        fs::write(&save_path, [0xAA, 0xBB]).expect("initial write should work");

        write_file_atomic(&save_path, &[0x11, 0x22, 0x33]).expect("atomic write should work");
        let data = fs::read(&save_path).expect("read should work");
        assert_eq!(data, vec![0x11, 0x22, 0x33]);

        let parent = save_path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = save_path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("temp save path should have a utf8 name");
        let tmp_prefix = format!(".{file_name}.tmp.");
        let has_temp_files = fs::read_dir(parent)
            .expect("read_dir should work")
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .any(|name| name.starts_with(&tmp_prefix));
        assert!(!has_temp_files);

        let _ = fs::remove_file(save_path);
    }

    #[test]
    fn rom_only_carts_do_not_create_file_backed_paths() {
        let rom_path = unique_temp_file_path("rom_only_paths", "gb");
        let rom = make_rom(32 * 1024, ROM_ONLY, 0x00, 0x00);
        fs::write(&rom_path, rom).expect("ROM file write should work");

        let (mut cartridge, persistence) =
            load_cartridge_from_file(&rom_path).expect("cartridge should load");
        persistence
            .flush_cartridge(&mut cartridge)
            .expect("flush should succeed");
        assert!(!rom_path.with_extension("sav").exists());
        assert!(!rom_path.with_extension("rtc").exists());

        let _ = fs::remove_file(rom_path);
    }
}
