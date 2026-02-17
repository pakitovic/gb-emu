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
    let mut rom_path: Option<String> = None;
    let mut trace = false;
    let mut blargg = false;
    let mut max_steps: usize = 20_000_000;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            "--blargg" => blargg = true,
            "--max-steps" => {
                let Some(value) = args.next() else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--max-steps requires a numeric value",
                    )
                    .into());
                };
                max_steps = value.parse::<usize>().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Invalid --max-steps value")
                })?;
            }
            _ => rom_path = Some(arg),
        }
    }

    let Some(rom_path) = rom_path else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: cargo run -- [--trace] [--blargg] [--max-steps N] <rom_file>",
        )
        .into());
    };

    let cartridge = Cartridge::from_file(&rom_path)?;

    let mut gb = GameBoy::new(cartridge);

    if blargg {
        match gb.run_blargg(max_steps, trace).as_deref() {
            Some("Passed") => {
                println!("\nBlargg result: Passed");
            }
            Some("Failed") => {
                println!("\nBlargg result: Failed");
                return Err(io::Error::other("Blargg test reported Failed").into());
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Blargg test did not finish within max steps",
                )
                .into());
            }
        }
    } else {
        gb.run(trace);
    }

    Ok(())
}
