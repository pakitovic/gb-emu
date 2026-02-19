use gb_emu::cartridge::Cartridge;
use gb_emu::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::hardware::HardwareModel;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::env;
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

const SCALE: u32 = 4;
const FRAME_STEP_LIMIT: usize = 250_000;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let (rom_path, model) = parse_args(env::args().skip(1))?;

    let cartridge = Cartridge::from_file(&rom_path)?;
    let mut gb = GameBoy::new_with_model(cartridge, model);

    let sdl = sdl2::init().map_err(io::Error::other)?;
    let video = sdl.video().map_err(io::Error::other)?;

    let window = video
        .window(
            &format!("gb-emu SDL2 | {}", gb.rom_title()),
            (SCREEN_WIDTH as u32) * SCALE,
            (SCREEN_HEIGHT as u32) * SCALE,
        )
        .position_centered()
        .resizable()
        .build()
        .map_err(io::Error::other)?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(io::Error::other)?;

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            SCREEN_WIDTH as u32,
            SCREEN_HEIGHT as u32,
        )
        .map_err(io::Error::other)?;

    let mut event_pump = sdl.event_pump().map_err(io::Error::other)?;
    let target_frame_time = Duration::from_micros(16_667);

    'main_loop: loop {
        let frame_start = Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main_loop,
                _ => {}
            }
        }

        let Some(_) = gb.run_frame_with_limit(false, FRAME_STEP_LIMIT) else {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "PPU frame was not produced within the SDL frame step budget",
            )
            .into());
        };

        let frame = gb.framebuffer();
        texture
            .with_lock(None, |bytes, pitch| {
                for y in 0..SCREEN_HEIGHT {
                    for x in 0..SCREEN_WIDTH {
                        let shade = frame[y * SCREEN_WIDTH + x];
                        let offset = y * pitch + x * 3;
                        bytes[offset] = shade;
                        bytes[offset + 1] = shade;
                        bytes[offset + 2] = shade;
                    }
                }
            })
            .map_err(io::Error::other)?;

        canvas.clear();
        canvas
            .copy(&texture, None, None)
            .map_err(io::Error::other)?;
        canvas.present();

        let elapsed = frame_start.elapsed();
        if elapsed < target_frame_time {
            std::thread::sleep(target_frame_time - elapsed);
        }
    }

    Ok(())
}

fn parse_args<I>(args: I) -> Result<(String, HardwareModel), io::Error>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(rom_path) = args.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: cargo run --features frontend-sdl2 --bin frontend-sdl2 -- <rom_file> [dmg0|dmg|mgb|sgb|sgb2]",
        ));
    };

    let model = if let Some(model_str) = args.next() {
        model_str
            .parse::<HardwareModel>()
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?
    } else {
        HardwareModel::default()
    };

    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected one ROM path and optional hardware model",
        ));
    }

    Ok((rom_path, model))
}
