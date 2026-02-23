use gb_emu::hardware::HardwareModel;
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
}
