use gb_emu::gameboy::GameBoy;
use gb_emu::hardware::HardwareModel;
use gb_runtime::cartridge_debug::format_cartridge_debug_report;
use gb_runtime::cartridge_persistence::load_cartridge_from_file;
use std::env;
use std::error::Error;

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

    let mut gb = GameBoy::new_with_model(cartridge, options.model);
    execute_cli_mode(&mut gb, &options)?;

    persistence.flush_gameboy(&mut gb)?;
    Ok(())
}
