use gb_emu::hardware::HardwareModel;
use gb_emu::video::VideoPalette;
use gb_runtime::audio::AudioResamplerQuality;
use std::env;
use std::io;

pub(super) fn parse_args<I>(args: I) -> Result<(String, HardwareModel), io::Error>
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

pub(super) fn audio_resampler_quality_name(quality: AudioResamplerQuality) -> &'static str {
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

fn parse_sdl_vsync(value: &str) -> Result<bool, io::Error> {
    match value {
        "1" | "true" | "on" => Ok(true),
        "0" | "false" | "off" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid GB_SDL2_VSYNC='{value}' (expected 1/0, true/false, on/off)"),
        )),
    }
}

pub(super) fn parse_sdl_vsync_from_env() -> Result<bool, io::Error> {
    match env::var("GB_SDL2_VSYNC") {
        Ok(value) => parse_sdl_vsync(value.trim()),
        Err(env::VarError::NotPresent) => Ok(true),
        Err(err) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to read GB_SDL2_VSYNC: {err}"),
        )),
    }
}

pub(super) fn parse_audio_resampler_quality_from_env() -> Result<AudioResamplerQuality, io::Error> {
    match env::var("GB_AUDIO_RESAMPLER") {
        Ok(value) => parse_audio_resampler_quality(value.trim()),
        Err(env::VarError::NotPresent) => Ok(AudioResamplerQuality::Cubic),
        Err(err) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to read GB_AUDIO_RESAMPLER: {err}"),
        )),
    }
}

fn parse_video_palette(value: &str, model: HardwareModel) -> Result<VideoPalette, io::Error> {
    if value.is_empty() || value.eq_ignore_ascii_case("auto") {
        return Ok(VideoPalette::for_model(model));
    }

    value
        .parse::<VideoPalette>()
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))
}

pub(super) fn parse_video_palette_from_env(
    model: HardwareModel,
) -> Result<VideoPalette, io::Error> {
    match env::var("GB_VIDEO_PALETTE") {
        Ok(value) => parse_video_palette(value.trim(), model),
        Err(env::VarError::NotPresent) => Ok(VideoPalette::for_model(model)),
        Err(err) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to read GB_VIDEO_PALETTE: {err}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_accepts_rom_only_and_defaults_model() {
        let (rom_path, model) =
            parse_args(["roms/tetris.gb".to_string()]).expect("args should parse");
        assert_eq!(rom_path, "roms/tetris.gb");
        assert_eq!(model, HardwareModel::Dmg);
    }

    #[test]
    fn parse_args_accepts_explicit_model() {
        let (rom_path, model) = parse_args(["roms/tetris.gb".to_string(), "mgb".to_string()])
            .expect("args should parse");
        assert_eq!(rom_path, "roms/tetris.gb");
        assert_eq!(model, HardwareModel::Mgb);
    }

    #[test]
    fn parse_args_rejects_invalid_model() {
        let err = parse_args(["roms/tetris.gb".to_string(), "cgb".to_string()])
            .expect_err("invalid model should fail");
        assert!(err.to_string().contains("Unsupported model"));
    }

    #[test]
    fn parse_args_rejects_missing_rom_path() {
        let err = parse_args(Vec::<String>::new()).expect_err("missing rom path should fail");
        assert!(err.to_string().contains("Usage:"));
    }

    #[test]
    fn parse_args_rejects_extra_arguments() {
        let err = parse_args([
            "roms/tetris.gb".to_string(),
            "dmg".to_string(),
            "extra".to_string(),
        ])
        .expect_err("extra args should fail");
        assert!(err.to_string().contains("Expected one ROM path"));
    }

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

    #[test]
    fn parse_sdl_vsync_accepts_supported_values() {
        for value in ["1", "true", "on"] {
            assert!(parse_sdl_vsync(value).expect("truthy value should parse"));
        }
        for value in ["0", "false", "off"] {
            assert!(!parse_sdl_vsync(value).expect("falsy value should parse"));
        }
    }

    #[test]
    fn parse_sdl_vsync_rejects_invalid_values() {
        let err = parse_sdl_vsync("maybe").expect_err("invalid value should fail");
        assert!(err.to_string().contains("GB_SDL2_VSYNC"));
    }

    #[test]
    fn parse_video_palette_defaults_to_model_mapped_palette() {
        assert_eq!(
            parse_video_palette("auto", HardwareModel::Mgb).expect("auto should parse"),
            VideoPalette::Mgb
        );
        assert_eq!(
            parse_video_palette("", HardwareModel::Dmg).expect("empty should parse"),
            VideoPalette::Dmg
        );
    }

    #[test]
    fn parse_video_palette_accepts_named_profiles() {
        assert_eq!(
            parse_video_palette("dmg", HardwareModel::Dmg).expect("dmg should parse"),
            VideoPalette::Dmg
        );
        assert_eq!(
            parse_video_palette("sgb", HardwareModel::Dmg).expect("sgb should parse"),
            VideoPalette::Sgb
        );
    }

    #[test]
    fn parse_video_palette_rejects_invalid_values() {
        let err = parse_video_palette("unknown", HardwareModel::Dmg)
            .expect_err("invalid palette should fail");
        assert!(err.to_string().contains("Unsupported palette"));
    }
}
