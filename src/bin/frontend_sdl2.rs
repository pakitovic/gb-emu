use gb_emu::audio::{AudioMixer, MixerSource};
use gb_emu::cartridge::Cartridge;
use gb_emu::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::hardware::HardwareModel;
use gb_emu::input::Button;
use gb_emu::timing::FramePacer;
use sdl2::audio::AudioSpecDesired;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::env;
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

const SCALE: u32 = 4;
const FRAME_STEP_LIMIT: usize = 250_000;
const AUDIO_QUEUE_LOW_WATER_SAMPLES: usize = 2048;
const AUDIO_QUEUE_TARGET_SAMPLES: usize = 4096;
const AUDIO_QUEUE_MAX_SAMPLES: usize = 16384;

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
    let audio = sdl.audio().map_err(io::Error::other)?;

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

    let desired_audio = AudioSpecDesired {
        freq: Some(48_000),
        channels: Some(1),
        samples: Some(1024),
    };
    let audio_queue = audio
        .open_queue::<f32, _>(None, &desired_audio)
        .map_err(io::Error::other)?;
    audio_queue.resume();
    let mut audio_mixer = AudioMixer::new(audio_queue.spec().freq.max(1) as u32);
    if env::var("GB_AUDIO_TEST_TONE")
        .map(|value| value == "1")
        .unwrap_or(false)
    {
        audio_mixer.set_source(MixerSource::TestTone);
    }

    let mut event_pump = sdl.event_pump().map_err(io::Error::other)?;
    let mut pacer = FramePacer::default();
    let mut last_host_tick = Instant::now();

    'main_loop: loop {
        let now = Instant::now();
        pacer.push_host_time(now.saturating_duration_since(last_host_tick));
        last_host_tick = now;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main_loop,
                Event::KeyDown {
                    keycode: Some(code),
                    repeat: false,
                    ..
                } => {
                    if let Some(button) = map_key_to_button(code) {
                        gb.set_button_pressed(button, true);
                    }
                }
                Event::KeyUp {
                    keycode: Some(code),
                    repeat: false,
                    ..
                } => {
                    if let Some(button) = map_key_to_button(code) {
                        gb.set_button_pressed(button, false);
                    }
                }
                _ => {}
            }
        }

        let mut produced_frame = false;
        while pacer.has_frame_budget() {
            let Some(cycles) = gb.run_frame_with_limit(false, FRAME_STEP_LIMIT) else {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "PPU frame was not produced within the SDL frame step budget",
                )
                .into());
            };
            pacer.consume_emulated_cycles(cycles);
            produced_frame = true;
        }

        refill_audio_queue(&audio_queue, &mut audio_mixer, pacer.drain_audio_tcycles());

        if !produced_frame {
            let sleep_for = pacer.duration_until_next_frame();
            if sleep_for > Duration::from_micros(200) {
                std::thread::sleep(sleep_for.min(Duration::from_millis(2)));
            }
            continue;
        }

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
    }

    Ok(())
}

fn map_key_to_button(code: Keycode) -> Option<Button> {
    match code {
        Keycode::Right => Some(Button::Right),
        Keycode::Left => Some(Button::Left),
        Keycode::Up => Some(Button::Up),
        Keycode::Down => Some(Button::Down),
        Keycode::Z => Some(Button::A),
        Keycode::X => Some(Button::B),
        Keycode::Backspace => Some(Button::Select),
        Keycode::Return => Some(Button::Start),
        _ => None,
    }
}

fn refill_audio_queue(
    audio_queue: &sdl2::audio::AudioQueue<f32>,
    mixer: &mut AudioMixer,
    pending_tcycles: u64,
) {
    mixer.push_tcycles(pending_tcycles);

    let sample_size_bytes = std::mem::size_of::<f32>();
    let mut queued_samples = (audio_queue.size() as usize) / sample_size_bytes;

    if queued_samples > AUDIO_QUEUE_MAX_SAMPLES {
        audio_queue.clear();
        queued_samples = 0;
    }

    if queued_samples >= AUDIO_QUEUE_LOW_WATER_SAMPLES {
        return;
    }

    let wanted = AUDIO_QUEUE_TARGET_SAMPLES.saturating_sub(queued_samples);
    let samples = mixer.drain_realtime_block(0, wanted);
    if wanted > 0 {
        let _ = audio_queue.queue_audio(&samples);
    }
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
