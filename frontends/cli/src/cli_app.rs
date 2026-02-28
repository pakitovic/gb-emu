use gb_emu::gameboy::GameBoy;
use gb_emu::hardware::HardwareModel;
use gb_runtime::bootrom::{load_boot_rom_for_model, load_boot_rom_for_model_from_dir};
use gb_runtime::cartridge_debug::format_cartridge_debug_report;
use gb_runtime::cartridge_persistence::load_cartridge_from_file;
use std::env;
use std::error::Error;
use std::path::PathBuf;

mod args;
mod dispatch;
mod runners;

use args::parse_args;
use dispatch::execute_cli_mode;

const MOONEYE_LOOP_WINDOW: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    rom_path: String,
    trace: bool,
    blargg: bool,
    mooneye: bool,
    cart_info: bool,
    model: HardwareModel,
    max_steps: usize,
    no_bootrom: bool,
    bootrom_dir: Option<PathBuf>,
}

pub(crate) fn main_entry() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_args(env::args().skip(1))?;

    let (cartridge, persistence) = load_cartridge_from_file(&options.rom_path)?;
    if options.cart_info {
        println!("{}", format_cartridge_debug_report(&cartridge.metadata()));
        return Ok(());
    }

    let boot_rom = resolve_boot_rom(&options);
    let mut gb = GameBoy::new_with_model_and_boot_rom(cartridge, options.model, boot_rom);
    execute_cli_mode(&mut gb, &options)?;

    persistence.flush_gameboy(&mut gb)?;
    Ok(())
}

fn should_auto_load_boot_rom(options: &CliOptions) -> bool {
    if options.cart_info || options.no_bootrom {
        return false;
    }

    if options.blargg || options.mooneye {
        // Keep deterministic test-suite behavior unless the user explicitly
        // requests a custom boot ROM directory for those modes.
        return options.bootrom_dir.is_some();
    }

    true
}

fn resolve_boot_rom(options: &CliOptions) -> Option<gb_emu::bootrom::BootRomData> {
    if !should_auto_load_boot_rom(options) {
        return None;
    }

    match options.bootrom_dir.as_ref() {
        Some(path) => load_boot_rom_for_model_from_dir(options.model, path),
        None => load_boot_rom_for_model(options.model),
    }
}

#[cfg(test)]
mod tests {
    use super::{CliOptions, should_auto_load_boot_rom};
    use gb_emu::hardware::HardwareModel;
    use std::path::PathBuf;

    fn base_options() -> CliOptions {
        CliOptions {
            rom_path: "test.gb".to_string(),
            trace: false,
            blargg: false,
            mooneye: false,
            cart_info: false,
            model: HardwareModel::Dmg,
            max_steps: 20_000_000,
            no_bootrom: false,
            bootrom_dir: None,
        }
    }

    #[test]
    fn auto_load_boot_rom_is_enabled_for_default_game_runs() {
        assert!(should_auto_load_boot_rom(&base_options()));
    }

    #[test]
    fn auto_load_boot_rom_is_disabled_for_test_modes() {
        let mut options = base_options();
        options.blargg = true;
        assert!(!should_auto_load_boot_rom(&options));

        options.blargg = false;
        options.mooneye = true;
        assert!(!should_auto_load_boot_rom(&options));
    }

    #[test]
    fn auto_load_boot_rom_can_be_disabled_explicitly() {
        let mut options = base_options();
        options.no_bootrom = true;
        assert!(!should_auto_load_boot_rom(&options));
    }

    #[test]
    fn auto_load_boot_rom_in_test_modes_requires_explicit_directory() {
        let mut options = base_options();
        options.blargg = true;
        options.bootrom_dir = Some(PathBuf::from("roms/bootrom"));
        assert!(should_auto_load_boot_rom(&options));
    }
}
