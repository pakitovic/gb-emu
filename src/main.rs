mod cpu;
mod memory;
mod gameboy;

use gameboy::GameBoy;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Usage: cargo run <rom_file>");
    }

    let rom_path = &args[1];
    let rom = gameboy::load_rom(rom_path);

    let mut gb = GameBoy::new(rom);

    gb.run();
}
