use super::CliOptions;
use super::runners::{run_blargg, run_forever, run_mooneye};
use gb_emu::gameboy::GameBoy;
use std::io;

pub(super) fn execute_cli_mode(gb: &mut GameBoy, options: &CliOptions) -> Result<(), io::Error> {
    if options.blargg {
        match run_blargg(gb, options.max_steps, options.trace) {
            Some("Passed") => {
                println!("\nBlargg result: Passed");
                Ok(())
            }
            Some("Failed") => {
                println!("\nBlargg result: Failed");
                Err(io::Error::other("Blargg test reported Failed"))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Blargg test did not finish within max steps",
            )),
        }
    } else if options.mooneye {
        match run_mooneye(gb, options.max_steps, options.trace) {
            Some("Passed") => {
                println!("\nMooneye result: Passed");
                Ok(())
            }
            Some("Failed") => {
                println!("\nMooneye result: Failed");
                Err(io::Error::other("Mooneye test reported Failed"))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Mooneye test did not finish within max steps",
            )),
        }
    } else {
        run_forever(gb, options.trace)
    }
}
