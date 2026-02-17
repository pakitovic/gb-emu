mod cartridge;
mod cpu;
mod gameboy;
mod memory;

use cartridge::Cartridge;
use gameboy::GameBoy;
use std::env;
use std::error::Error;
use std::io;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "Usage: cargo run <rom_file>").into(),
        );
    }

    let rom_path = &args[1];
    let cartridge = Cartridge::from_file(rom_path)?;

    let mut gb = GameBoy::new(cartridge);

    gb.run();

    Ok(())
}
