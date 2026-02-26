use gb_emu::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_runtime::audio::{AudioMixer, MixerSource};
use gb_runtime::cartridge_debug::format_cartridge_debug_report;
use gb_runtime::cartridge_persistence::load_cartridge_from_file;
use gb_runtime::timing::FramePacer;
use sdl2::audio::AudioSpecDesired;
use sdl2::pixels::PixelFormatEnum;
use std::env;
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

mod args;
mod audio_queue;
mod input;
mod save_flush;
mod ui;

use args::{
    audio_resampler_quality_name, parse_args, parse_audio_resampler_quality_from_env,
    parse_sdl_vsync_from_env,
};
use audio_queue::{SdlAudioQueueState, refill_audio_queue};
use input::{EventAction, process_event};
use save_flush::SaveAutosaveDebouncer;
use ui::{build_window_title, render_grayscale_frame, show_cartridge_info_dialog};

const SCALE: u32 = 4;
const FRAME_STEP_LIMIT: usize = 250_000;
const AUDIO_QUEUE_TARGET_INITIAL_SAMPLES: usize = 4_096;
const AUDIO_QUEUE_TARGET_MIN_SAMPLES: usize = 2_048;
const AUDIO_QUEUE_TARGET_MAX_SAMPLES: usize = 16_384;
const AUDIO_QUEUE_HARD_MAX_SAMPLES: usize = 32_768;
const AUDIO_REFILL_BLOCK_SAMPLES: usize = 512;
const AUDIO_REFILL_MAX_BLOCKS: usize = 32;
const AUDIO_CHANNELS: usize = 2;
const SAVE_AUTOSAVE_DEBOUNCE: Duration = Duration::from_secs(2);

pub(crate) fn main_entry() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let (rom_path, model) = parse_args(env::args().skip(1))?;

    let (cartridge, persistence) = load_cartridge_from_file(&rom_path)?;
    let mut gb = GameBoy::new_with_model(cartridge, model);
    gb.set_audio_tcycle_stream_enabled(true);
    let cartridge_metadata = gb.cartridge_metadata();
    let cartridge_debug_report = format_cartridge_debug_report(&cartridge_metadata);
    println!("{cartridge_debug_report}");

    let sdl = sdl2::init().map_err(io::Error::other)?;
    let video = sdl.video().map_err(io::Error::other)?;
    let audio = sdl.audio().map_err(io::Error::other)?;

    let window = video
        .window(
            &build_window_title(&gb, &cartridge_metadata),
            (SCREEN_WIDTH as u32) * SCALE,
            (SCREEN_HEIGHT as u32) * SCALE,
        )
        .position_centered()
        .resizable()
        .build()
        .map_err(io::Error::other)?;

    let sdl_vsync = parse_sdl_vsync_from_env()?;
    let canvas_builder = window.into_canvas().accelerated();
    let mut canvas = if sdl_vsync {
        canvas_builder.present_vsync().build()
    } else {
        canvas_builder.build()
    }
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
        channels: Some(AUDIO_CHANNELS as u8),
        samples: Some(1024),
    };
    let audio_queue = audio
        .open_queue::<f32, _>(None, &desired_audio)
        .map_err(io::Error::other)?;
    audio_queue.resume();
    let mut audio_mixer = AudioMixer::new(audio_queue.spec().freq.max(1) as u32);
    audio_mixer.set_source(MixerSource::CoreApu);
    let resampler_quality = parse_audio_resampler_quality_from_env()?;
    audio_mixer.set_core_resampler_quality(resampler_quality);
    if env::var("GB_AUDIO_TEST_TONE")
        .map(|value| value == "1")
        .unwrap_or(false)
    {
        audio_mixer.set_source(MixerSource::TestTone);
    }
    println!(
        "Audio config: source={} | sample_rate={} Hz | resampler={}",
        match audio_mixer.source() {
            MixerSource::Silence => "silence",
            MixerSource::TestTone => "test-tone",
            MixerSource::CoreApu => "core-apu",
        },
        audio_mixer.sample_rate_hz(),
        audio_resampler_quality_name(audio_mixer.core_resampler_quality())
    );
    println!(
        "Video config: renderer=accelerated | vsync={}",
        if sdl_vsync { "on" } else { "off" }
    );

    let mut event_pump = sdl.event_pump().map_err(io::Error::other)?;
    let mut pacer = FramePacer::default();
    let mut last_host_tick = Instant::now();
    let mut audio_queue_state =
        SdlAudioQueueState::new(audio_mixer.sample_rate_hz(), last_host_tick);
    let mut save_autosave = SaveAutosaveDebouncer::new(SAVE_AUTOSAVE_DEBOUNCE);

    'main_loop: loop {
        let now = Instant::now();
        pacer.push_host_time(now.saturating_duration_since(last_host_tick));
        last_host_tick = now;

        for event in event_pump.poll_iter() {
            match process_event(&mut gb, event) {
                EventAction::Continue => {}
                EventAction::Quit => break 'main_loop,
                EventAction::FlushPersistence => {
                    flush_persistence(&persistence, &mut gb)?;
                    save_autosave.mark_flushed();
                }
                EventAction::ShowCartInfo => {
                    show_cartridge_info_dialog(&cartridge_debug_report);
                }
            }
        }

        let mut produced_frame = false;
        while pacer.has_frame_budget() {
            let Some(cycles) = gb.run_frame_with_limit(FRAME_STEP_LIMIT) else {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "PPU frame was not produced within the SDL frame step budget",
                )
                .into());
            };
            pacer.consume_emulated_cycles(cycles);
            let tcycle_samples = gb.drain_audio_tcycle_samples();
            audio_mixer.push_core_tcycle_samples(&tcycle_samples);
            produced_frame = true;
        }

        refill_audio_queue(
            &audio_queue,
            &mut audio_mixer,
            &mut audio_queue_state,
            pacer.drain_audio_tcycles(),
            now,
        );

        if save_autosave.update_and_should_flush(gb.cartridge_battery_save_dirty(), now) {
            flush_persistence(&persistence, &mut gb)?;
            save_autosave.mark_flushed();
        }

        if !produced_frame {
            let sleep_for = pacer.duration_until_next_frame();
            if sleep_for > Duration::from_micros(200) {
                std::thread::sleep(sleep_for.min(Duration::from_millis(2)));
            }
            continue;
        }

        render_grayscale_frame(&mut texture, &mut canvas, gb.framebuffer())?;
    }

    flush_persistence(&persistence, &mut gb)?;

    Ok(())
}

fn flush_persistence(
    persistence: &gb_runtime::cartridge_persistence::FileBackedCartridgePersistence,
    gb: &mut GameBoy,
) -> Result<(), io::Error> {
    persistence.flush_gameboy(gb).map_err(io::Error::other)
}
