use super::CliOptions;
use gb_emu::hardware::HardwareModel;
use std::io;
use std::path::PathBuf;

pub(super) fn parse_args<I>(args: I) -> Result<CliOptions, io::Error>
where
    I: IntoIterator<Item = String>,
{
    let mut rom_path: Option<String> = None;
    let mut trace = false;
    let mut blargg = false;
    let mut mooneye = false;
    let mut sgb_report = false;
    let mut cart_info = false;
    let mut model = HardwareModel::default();
    let mut max_steps: usize = 20_000_000;
    let mut no_bootrom = false;
    let mut bootrom_dir: Option<PathBuf> = None;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            "--blargg" => blargg = true,
            "--mooneye" => mooneye = true,
            "--sgb-report" => sgb_report = true,
            "--cart-info" => cart_info = true,
            "--no-bootrom" => no_bootrom = true,
            "--bootrom-dir" => {
                let Some(value) = args.next() else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--bootrom-dir requires a directory path",
                    ));
                };
                bootrom_dir = Some(PathBuf::from(value));
            }
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
            "Usage: cargo run -p frontend-cli --bin gb-emu -- [--trace] [--blargg|--mooneye] [--cart-info] [--model dmg0|dmg|mgb|sgb|sgb2] [--no-bootrom] [--bootrom-dir <path>] [--max-steps N] <rom_file>",
        ));
    };

    if blargg && mooneye {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use either --blargg or --mooneye",
        ));
    }

    if sgb_report && (blargg || mooneye) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use --sgb-report by itself or with --trace, not with --blargg/--mooneye",
        ));
    }

    Ok(CliOptions {
        rom_path,
        trace,
        blargg,
        mooneye,
        sgb_report,
        cart_info,
        model,
        max_steps,
        no_bootrom,
        bootrom_dir,
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
                sgb_report: false,
                cart_info: false,
                model: HardwareModel::Mgb,
                max_steps: 123,
                no_bootrom: false,
                bootrom_dir: None,
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
                sgb_report: false,
                cart_info: true,
                model: HardwareModel::default(),
                max_steps: 20_000_000,
                no_bootrom: false,
                bootrom_dir: None,
            }
        );
    }

    #[test]
    fn parses_boot_rom_options() {
        let options = parse(&["--no-bootrom", "--bootrom-dir", "roms/bootrom", "test.gb"])
            .expect("args should parse");
        assert_eq!(
            options,
            CliOptions {
                rom_path: "test.gb".to_string(),
                trace: false,
                blargg: false,
                mooneye: false,
                sgb_report: false,
                cart_info: false,
                model: HardwareModel::default(),
                max_steps: 20_000_000,
                no_bootrom: true,
                bootrom_dir: Some(PathBuf::from("roms/bootrom")),
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
    fn parses_sgb_report_mode() {
        let options =
            parse(&["--sgb-report", "--model", "sgb", "test.gb"]).expect("args should parse");
        assert!(options.sgb_report);
        assert_eq!(options.model, HardwareModel::Sgb);
    }

    #[test]
    fn rejects_sgb_report_with_blargg_or_mooneye() {
        let error = parse(&["--sgb-report", "--blargg", "test.gb"]).expect_err("args should fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            error.to_string(),
            "Use --sgb-report by itself or with --trace, not with --blargg/--mooneye"
        );
    }
}
