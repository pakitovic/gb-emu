use gb_emu::cartridge::Cartridge;
use gb_emu::gameboy::GameBoy;
use gb_emu::hardware::HardwareModel;
use std::env;
use std::error::Error;
use std::io;

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

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn looks_like_tight_loop(pc_window: &[u16; MOONEYE_LOOP_WINDOW]) -> bool {
    let mut unique = [0u16; 4];
    let mut unique_len = 0usize;

    'outer: for &pc in pc_window {
        for &seen in unique.iter().take(unique_len) {
            if seen == pc {
                continue 'outer;
            }
        }

        if unique_len == unique.len() {
            return false;
        }
        unique[unique_len] = pc;
        unique_len += 1;
    }

    true
}

fn print_basic_trace(gb: &GameBoy, cycles: u8) {
    println!(
        "PC: {:04X}, A: {:02X}, cycles: {}",
        gb.cpu.registers.pc, gb.cpu.registers.a, cycles
    );
}

fn print_mooneye_trace(gb: &GameBoy, cycles: u8) {
    println!(
        "PC: {:04X}, A: {:02X}, B: {:02X}, C: {:02X}, D: {:02X}, E: {:02X}, H: {:02X}, L: {:02X}, cycles: {}",
        gb.cpu.registers.pc,
        gb.cpu.registers.a,
        gb.cpu.registers.b,
        gb.cpu.registers.c,
        gb.cpu.registers.d,
        gb.cpu.registers.e,
        gb.cpu.registers.h,
        gb.cpu.registers.l,
        cycles
    );
}

fn run_forever(gb: &mut GameBoy, trace: bool) -> ! {
    println!("ROM: {}", gb.rom_title());
    loop {
        let cycles = gb.step();
        if trace {
            print_basic_trace(gb, cycles);
        }
    }
}

fn run_blargg(gb: &mut GameBoy, max_steps: usize, trace: bool) -> Option<&'static str> {
    println!("ROM: {}", gb.rom_title());
    for _ in 0..max_steps {
        let cycles = gb.step();
        if trace {
            print_basic_trace(gb, cycles);
        }

        let serial = gb.serial_output();
        if serial.contains("Passed") {
            return Some("Passed");
        }
        if serial.contains("Failed") {
            return Some("Failed");
        }

        // Blargg memory protocol fallback:
        // A001..A003 == DE B0 61, A000 == status (0 pass, non-zero fail, 0x80 running).
        let sig_ok = gb.bus.read_byte(0xA001) == 0xDE
            && gb.bus.read_byte(0xA002) == 0xB0
            && gb.bus.read_byte(0xA003) == 0x61;
        if sig_ok {
            let status = gb.bus.read_byte(0xA000);
            if status == 0x00 {
                return Some("Passed");
            }
            if status != 0x80 {
                return Some("Failed");
            }
        }
    }
    None
}

fn run_mooneye(gb: &mut GameBoy, max_steps: usize, trace: bool) -> Option<&'static str> {
    println!("ROM: {}", gb.rom_title());
    let mut pc_window = [0u16; MOONEYE_LOOP_WINDOW];
    let mut pc_window_len = 0usize;
    let mut pc_window_pos = 0usize;

    for _ in 0..max_steps {
        let cycles = gb.step();
        if trace {
            print_mooneye_trace(gb, cycles);
        }

        let pc = gb.cpu.registers.pc;
        pc_window[pc_window_pos] = pc;
        pc_window_pos = (pc_window_pos + 1) % MOONEYE_LOOP_WINDOW;
        if pc_window_len < MOONEYE_LOOP_WINDOW {
            pc_window_len += 1;
        }

        // Mooneye acceptance convention:
        // - Success signature in B,C,D,E,H,L: 3,5,8,13,21,34
        // - Failure signature in B,C,D,E,H,L: 0x42,0x42,0x42,0x42,0x42,0x42
        //
        // Final signatures are expected to be observed in a tight loop.
        // This avoids false negatives in tests where intermediate values
        // can temporarily match the failure signature.
        let regs = (
            gb.cpu.registers.b,
            gb.cpu.registers.c,
            gb.cpu.registers.d,
            gb.cpu.registers.e,
            gb.cpu.registers.h,
            gb.cpu.registers.l,
        );
        let in_tight_loop =
            pc_window_len == MOONEYE_LOOP_WINDOW && looks_like_tight_loop(&pc_window);
        if regs == (3, 5, 8, 13, 21, 34) && in_tight_loop {
            return Some("Passed");
        }
        if regs == (0x42, 0x42, 0x42, 0x42, 0x42, 0x42) && in_tight_loop {
            return Some("Failed");
        }
    }
    None
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_args(env::args().skip(1))?;

    let cartridge = Cartridge::from_file(&options.rom_path)?;
    if options.cart_info {
        println!("{}", cartridge.metadata().debug_report());
        return Ok(());
    }

    let mut gb = GameBoy::new_with_model(cartridge, options.model);

    if options.blargg {
        match run_blargg(&mut gb, options.max_steps, options.trace) {
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
    } else if options.mooneye {
        match run_mooneye(&mut gb, options.max_steps, options.trace) {
            Some("Passed") => {
                println!("\nMooneye result: Passed");
            }
            Some("Failed") => {
                println!("\nMooneye result: Failed");
                return Err(io::Error::other("Mooneye test reported Failed").into());
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Mooneye test did not finish within max steps",
                )
                .into());
            }
        }
    } else {
        run_forever(&mut gb, options.trace);
    }

    gb.flush_battery_save()?;
    Ok(())
}

fn parse_args<I>(args: I) -> Result<CliOptions, io::Error>
where
    I: IntoIterator<Item = String>,
{
    let mut rom_path: Option<String> = None;
    let mut trace = false;
    let mut blargg = false;
    let mut mooneye = false;
    let mut cart_info = false;
    let mut model = HardwareModel::default();
    let mut max_steps: usize = 20_000_000;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            "--blargg" => blargg = true,
            "--mooneye" => mooneye = true,
            "--cart-info" => cart_info = true,
            "--model" => {
                let Some(value) = args.next() else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--model requires a value: dmg0|dmg|mgb|sgb|sgb2",
                    ));
                };
                model = value
                    .parse::<HardwareModel>()
                    .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
            }
            "--max-steps" => {
                let Some(value) = args.next() else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--max-steps requires a numeric value",
                    ));
                };
                max_steps = value.parse::<usize>().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Invalid --max-steps value")
                })?;
            }
            _ if arg.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown option: {arg}"),
                ));
            }
            _ => {
                if rom_path.is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Only one ROM file can be provided",
                    ));
                }
                rom_path = Some(arg);
            }
        }
    }

    let Some(rom_path) = rom_path else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: cargo run -p frontend-cli --bin gb-emu -- [--trace] [--blargg|--mooneye] [--cart-info] [--model dmg0|dmg|mgb|sgb|sgb2] [--max-steps N] <rom_file>",
        ));
    };

    if blargg && mooneye {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use either --blargg or --mooneye",
        ));
    }

    Ok(CliOptions {
        rom_path,
        trace,
        blargg,
        mooneye,
        cart_info,
        model,
        max_steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<CliOptions, io::Error> {
        parse_args(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn parses_expected_options_and_rom_path() {
        let options = parse(&[
            "--trace",
            "--blargg",
            "--model",
            "mgb",
            "--max-steps",
            "123",
            "test.gb",
        ])
        .expect("args should parse");

        assert_eq!(
            options,
            CliOptions {
                rom_path: "test.gb".to_string(),
                trace: true,
                blargg: true,
                mooneye: false,
                cart_info: false,
                model: HardwareModel::Mgb,
                max_steps: 123,
            }
        );
    }

    #[test]
    fn parses_cart_info_flag() {
        let options = parse(&["--cart-info", "test.gb"]).expect("args should parse");
        assert_eq!(
            options,
            CliOptions {
                rom_path: "test.gb".to_string(),
                trace: false,
                blargg: false,
                mooneye: false,
                cart_info: true,
                model: HardwareModel::default(),
                max_steps: 20_000_000,
            }
        );
    }

    #[test]
    fn rejects_unknown_flags_instead_of_treating_them_as_rom() {
        let error = parse(&["--unknown", "rom.gb"]).expect_err("unknown flag should fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), "Unknown option: --unknown");
    }

    #[test]
    fn rejects_multiple_rom_paths() {
        let error = parse(&["first.gb", "second.gb"]).expect_err("multiple ROM paths should fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), "Only one ROM file can be provided");
    }

    #[test]
    fn tight_loop_detector_accepts_small_repeating_pc_sets() {
        let one_pc = [0x1234; MOONEYE_LOOP_WINDOW];
        let two_pc = [
            0x2000, 0x2001, 0x2000, 0x2001, 0x2000, 0x2001, 0x2000, 0x2001,
        ];
        assert!(looks_like_tight_loop(&one_pc));
        assert!(looks_like_tight_loop(&two_pc));
    }

    #[test]
    fn tight_loop_detector_rejects_wide_pc_ranges() {
        let wide = [
            0x1000, 0x1001, 0x1002, 0x1003, 0x1004, 0x1005, 0x1006, 0x1007,
        ];
        assert!(!looks_like_tight_loop(&wide));
    }
}
