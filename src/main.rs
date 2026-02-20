use gb_emu::cartridge::Cartridge;
use gb_emu::gameboy::GameBoy;
use gb_emu::hardware::HardwareModel;
use std::env;
use std::error::Error;
use std::io;

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

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_args(env::args().skip(1))?;

    let cartridge = Cartridge::from_file(&options.rom_path)?;
    if options.cart_info {
        println!("{}", cartridge.metadata().debug_report());
        return Ok(());
    }

    let mut gb = GameBoy::new_with_model(cartridge, options.model);

    if options.blargg {
        match gb.run_blargg(options.max_steps, options.trace).as_deref() {
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
        match gb.run_mooneye(options.max_steps, options.trace).as_deref() {
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
        gb.run(options.trace);
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
            "Usage: cargo run -- [--trace] [--blargg|--mooneye] [--cart-info] [--model dmg0|dmg|mgb|sgb|sgb2] [--max-steps N] <rom_file>",
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
}
