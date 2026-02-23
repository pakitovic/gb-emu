use gb_emu::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::hardware::HardwareModel;
use gb_emu::input::Button;
use gb_runtime::audio::{
    AdaptiveQueueController, AdaptiveQueueOptions, AudioMixer, AudioResamplerQuality, MixerSource,
    estimate_playback_underrun_samples,
};
use gb_runtime::cartridge_persistence::load_cartridge_from_file;
use gb_runtime::timing::FramePacer;
use sdl2::audio::AudioSpecDesired;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl2::pixels::PixelFormatEnum;
use std::env;
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

const SCALE: u32 = 4;
const FRAME_STEP_LIMIT: usize = 250_000;
const AUDIO_QUEUE_TARGET_INITIAL_SAMPLES: usize = 4_096;
const AUDIO_QUEUE_TARGET_MIN_SAMPLES: usize = 2_048;
const AUDIO_QUEUE_TARGET_MAX_SAMPLES: usize = 16_384;
const AUDIO_QUEUE_HARD_MAX_SAMPLES: usize = 32_768;
const AUDIO_REFILL_BLOCK_SAMPLES: usize = 512;
const AUDIO_REFILL_MAX_BLOCKS: usize = 32;
const AUDIO_CHANNELS: usize = 2;

fn main() {
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
    let cartridge_debug_report = cartridge_metadata.debug_report();
    println!("{cartridge_debug_report}");

    let sdl = sdl2::init().map_err(io::Error::other)?;
    let video = sdl.video().map_err(io::Error::other)?;
    let audio = sdl.audio().map_err(io::Error::other)?;

    let window = video
        .window(
            &format!(
                "gb-emu SDL2 | {} | {} | warnings {} | F1 cart-info",
                gb.rom_title(),
                cartridge_metadata.mapper,
                cartridge_metadata.header_warnings.len()
            ),
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

    let mut event_pump = sdl.event_pump().map_err(io::Error::other)?;
    let mut pacer = FramePacer::default();
    let mut last_host_tick = Instant::now();
    let mut audio_queue_state =
        SdlAudioQueueState::new(audio_mixer.sample_rate_hz(), last_host_tick);

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
                    keycode: Some(Keycode::F1),
                    repeat: false,
                    ..
                } => {
                    if let Err(err) = show_simple_message_box(
                        MessageBoxFlag::INFORMATION,
                        "Cartridge metadata",
                        &cartridge_debug_report,
                        None,
                    ) {
                        eprintln!("SDL2 cart-info panel failed: {err}");
                    }
                }
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

    persistence
        .flush_gameboy(&mut gb)
        .map_err(io::Error::other)?;

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
    queue_state: &mut SdlAudioQueueState,
    pending_tcycles: u64,
    now: Instant,
) {
    mixer.push_tcycles(pending_tcycles);
    let channel_count = usize::from(audio_queue.spec().channels.max(1));

    let mut queued_samples = queued_audio_samples(audio_queue);

    if queued_samples > AUDIO_QUEUE_HARD_MAX_SAMPLES {
        audio_queue.clear();
        queued_samples = 0;
    }

    let target_samples = queue_state.observe_and_update_target(now, queued_samples);

    let mut guard = 0;
    while queued_samples < target_samples && guard < AUDIO_REFILL_MAX_BLOCKS {
        let wanted = target_samples
            .saturating_sub(queued_samples)
            .min(AUDIO_REFILL_BLOCK_SAMPLES);
        let samples = mixer.drain_realtime_block(0, wanted);
        if samples.is_empty() {
            break;
        }
        if audio_queue.queue_audio(&samples).is_err() {
            break;
        }
        let enqueued_frames = samples.len() / channel_count;
        queued_samples = queued_samples.saturating_add(enqueued_frames);
        guard += 1;
    }

    queue_state.commit_refill(now, queued_samples);
}

fn queued_audio_samples(audio_queue: &sdl2::audio::AudioQueue<f32>) -> usize {
    let sample_size_bytes = std::mem::size_of::<f32>();
    let channel_count = usize::from(audio_queue.spec().channels.max(1));
    (audio_queue.size() as usize) / sample_size_bytes / channel_count
}

struct SdlAudioQueueState {
    sample_rate_hz: u32,
    start_instant: Instant,
    last_refill_instant: Instant,
    last_queue_after_refill_samples: usize,
    total_underrun_samples: u64,
    adaptive_target: AdaptiveQueueController,
}

impl SdlAudioQueueState {
    fn new(sample_rate_hz: u32, now: Instant) -> Self {
        let options = AdaptiveQueueOptions {
            min_target_samples: AUDIO_QUEUE_TARGET_MIN_SAMPLES,
            max_target_samples: AUDIO_QUEUE_TARGET_MAX_SAMPLES,
            ..AdaptiveQueueOptions::default()
        };
        let adaptive_target =
            AdaptiveQueueController::new(AUDIO_QUEUE_TARGET_INITIAL_SAMPLES, 0, 0, options);
        Self {
            sample_rate_hz: sample_rate_hz.max(1),
            start_instant: now,
            last_refill_instant: now,
            last_queue_after_refill_samples: 0,
            total_underrun_samples: 0,
            adaptive_target,
        }
    }

    fn observe_and_update_target(
        &mut self,
        now: Instant,
        queued_samples_before_refill: usize,
    ) -> usize {
        let elapsed = now.saturating_duration_since(self.last_refill_instant);
        let underrun_delta = estimate_playback_underrun_samples(
            self.last_queue_after_refill_samples,
            elapsed,
            self.sample_rate_hz,
        );
        self.total_underrun_samples = self.total_underrun_samples.saturating_add(underrun_delta);
        let now_ms = u64::try_from(
            now.saturating_duration_since(self.start_instant)
                .as_millis(),
        )
        .unwrap_or(u64::MAX);
        let update = self.adaptive_target.update(
            now_ms,
            queued_samples_before_refill,
            self.total_underrun_samples,
            AUDIO_REFILL_BLOCK_SAMPLES,
        );
        update.target_samples
    }

    fn commit_refill(&mut self, now: Instant, queued_samples_after_refill: usize) {
        self.last_refill_instant = now;
        self.last_queue_after_refill_samples = queued_samples_after_refill;
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
            "Usage: cargo run -p frontend-sdl2 --bin frontend-sdl2 -- <rom_file> [dmg0|dmg|mgb|sgb|sgb2]",
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

fn audio_resampler_quality_name(quality: AudioResamplerQuality) -> &'static str {
    match quality {
        AudioResamplerQuality::Linear => "linear",
        AudioResamplerQuality::Cubic => "cubic",
    }
}

fn parse_audio_resampler_quality(value: &str) -> Result<AudioResamplerQuality, io::Error> {
    match value {
        "linear" => Ok(AudioResamplerQuality::Linear),
        "cubic" => Ok(AudioResamplerQuality::Cubic),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid GB_AUDIO_RESAMPLER='{value}' (expected 'linear' or 'cubic')"),
        )),
    }
}

fn parse_audio_resampler_quality_from_env() -> Result<AudioResamplerQuality, io::Error> {
    match env::var("GB_AUDIO_RESAMPLER") {
        Ok(value) => parse_audio_resampler_quality(value.trim()),
        Err(env::VarError::NotPresent) => Ok(AudioResamplerQuality::Cubic),
        Err(err) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to read GB_AUDIO_RESAMPLER: {err}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_audio_resampler_quality_accepts_supported_values() {
        assert_eq!(
            parse_audio_resampler_quality("linear").expect("linear should parse"),
            AudioResamplerQuality::Linear
        );
        assert_eq!(
            parse_audio_resampler_quality("cubic").expect("cubic should parse"),
            AudioResamplerQuality::Cubic
        );
    }

    #[test]
    fn parse_audio_resampler_quality_rejects_invalid_values() {
        let err = parse_audio_resampler_quality("nearest").expect_err("invalid value should fail");
        assert!(err.to_string().contains("GB_AUDIO_RESAMPLER"));
    }
}
