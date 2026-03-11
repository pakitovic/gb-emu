use gb_emu::hardware::HardwareModel;
use gb_emu::palette_override::PaletteOverrideDb;
use gb_emu::video::VideoPalette;
use gb_runtime::audio::AudioResamplerQuality;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SdlOptions {
    pub(super) rom_path: String,
    pub(super) model: HardwareModel,
    pub(super) no_bootrom: bool,
    pub(super) bootrom_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoPaletteSelection {
    Auto,
    Palette(VideoPalette),
}

impl VideoPaletteSelection {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Palette(palette) => palette.as_str(),
        }
    }

    pub(super) fn resolve(self, model: HardwareModel, sgb_active: bool) -> VideoPalette {
        match self {
            Self::Auto if sgb_active && model.supports_sgb_features() => VideoPalette::Sgb,
            Self::Auto => VideoPalette::auto_base_for_model(model),
            Self::Palette(palette) => palette,
        }
    }
}

pub(super) fn parse_args<I>(args: I) -> Result<SdlOptions, io::Error>
where
    I: IntoIterator<Item = String>,
{
    let mut rom_path: Option<String> = None;
    let mut model = HardwareModel::default();
    let mut no_bootrom = false;
    let mut bootrom_dir: Option<PathBuf> = None;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
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
            _ if arg.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown option: {arg}"),
                ));
            }
            _ => {
                if rom_path.is_none() {
                    rom_path = Some(arg);
                    continue;
                }

                if model == HardwareModel::default() {
                    model = arg
                        .parse::<HardwareModel>()
                        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
                    continue;
                }

                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Expected one ROM path, optional hardware model, and optional boot ROM flags",
                ));
            }
        }
    }

    let Some(rom_path) = rom_path else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: cargo run -p frontend-sdl2 --bin frontend-sdl2 -- [--no-bootrom] [--bootrom-dir <path>] <rom_file> [dmg0|dmg|mgb|sgb|sgb2]",
        ));
    };

    Ok(SdlOptions {
        rom_path,
        model,
        no_bootrom,
        bootrom_dir,
    })
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

fn parse_video_palette(value: &str) -> Result<VideoPaletteSelection, io::Error> {
    if value.is_empty() || value.eq_ignore_ascii_case("auto") {
        return Ok(VideoPaletteSelection::Auto);
    }

    value
        .parse::<VideoPalette>()
        .map(VideoPaletteSelection::Palette)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))
}

pub(super) fn parse_video_palette_from_env() -> Result<VideoPaletteSelection, io::Error> {
    match env::var("GB_VIDEO_PALETTE") {
        Ok(value) => parse_video_palette(value.trim()),
        Err(env::VarError::NotPresent) => Ok(VideoPaletteSelection::Auto),
        Err(err) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to read GB_VIDEO_PALETTE: {err}"),
        )),
    }
}

pub(super) fn parse_palette_overrides_from_env() -> Result<Option<PaletteOverrideDb>, io::Error> {
    match env::var("GB_VIDEO_PALETTE_OVERRIDES") {
        Ok(value) => {
            let path = value.trim();
            if path.is_empty() {
                return Ok(None);
            }
            let ini = fs::read_to_string(path).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("Failed to read GB_VIDEO_PALETTE_OVERRIDES='{path}': {err}"),
                )
            })?;
            let overrides = PaletteOverrideDb::parse_ini(&ini).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid GB_VIDEO_PALETTE_OVERRIDES='{path}': {err}"),
                )
            })?;
            Ok(Some(overrides))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to read GB_VIDEO_PALETTE_OVERRIDES: {err}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_accepts_rom_only_and_defaults_model() {
        let options = parse_args(["roms/tetris.gb".to_string()]).expect("args should parse");
        assert_eq!(
            options,
            SdlOptions {
                rom_path: "roms/tetris.gb".to_string(),
                model: HardwareModel::Dmg,
                no_bootrom: false,
                bootrom_dir: None,
            }
        );
    }

    #[test]
    fn parse_args_accepts_explicit_model() {
        let options = parse_args(["roms/tetris.gb".to_string(), "mgb".to_string()])
            .expect("args should parse");
        assert_eq!(
            options,
            SdlOptions {
                rom_path: "roms/tetris.gb".to_string(),
                model: HardwareModel::Mgb,
                no_bootrom: false,
                bootrom_dir: None,
            }
        );
    }

    #[test]
    fn parse_args_accepts_boot_rom_flags() {
        let options = parse_args([
            "--no-bootrom".to_string(),
            "--bootrom-dir".to_string(),
            "roms/bootrom".to_string(),
            "roms/tetris.gb".to_string(),
            "sgb".to_string(),
        ])
        .expect("args should parse");
        assert_eq!(
            options,
            SdlOptions {
                rom_path: "roms/tetris.gb".to_string(),
                model: HardwareModel::Sgb,
                no_bootrom: true,
                bootrom_dir: Some(PathBuf::from("roms/bootrom")),
            }
        );
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
        assert!(err.to_string().contains("Unsupported model"));
    }

    #[test]
    fn parse_args_rejects_unknown_option() {
        let err = parse_args(["--unknown".to_string(), "roms/tetris.gb".to_string()])
            .expect_err("unknown option should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), "Unknown option: --unknown");
    }

    #[test]
    fn parse_args_rejects_missing_bootrom_dir_value() {
        let err =
            parse_args(["--bootrom-dir".to_string()]).expect_err("missing bootrom dir should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), "--bootrom-dir requires a directory path");
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
            parse_video_palette("auto").expect("auto should parse"),
            VideoPaletteSelection::Auto
        );
        assert_eq!(
            parse_video_palette("").expect("empty should parse"),
            VideoPaletteSelection::Auto
        );
    }

    #[test]
    fn parse_video_palette_accepts_named_profiles() {
        assert_eq!(
            parse_video_palette("dmg").expect("dmg should parse"),
            VideoPaletteSelection::Palette(VideoPalette::Dmg)
        );
        assert_eq!(
            parse_video_palette("cgb-blue").expect("manual CGB preset should parse"),
            VideoPaletteSelection::Palette(
                "cgb-blue"
                    .parse::<VideoPalette>()
                    .expect("manual CGB preset should parse"),
            )
        );
        assert_eq!(
            parse_video_palette("sgb").expect("sgb should parse"),
            VideoPaletteSelection::Palette(VideoPalette::Sgb)
        );
        assert_eq!(
            parse_video_palette("sgb-2c").expect("built-in SGB palette should parse"),
            VideoPaletteSelection::Palette(
                "sgb-2c"
                    .parse::<VideoPalette>()
                    .expect("built-in SGB palette should parse"),
            )
        );
    }

    #[test]
    fn parse_video_palette_rejects_invalid_values() {
        let err = parse_video_palette("unknown").expect_err("invalid palette should fail");
        assert!(err.to_string().contains("Unsupported palette"));
    }

    #[test]
    fn parse_palette_overrides_from_env_returns_none_when_unset() {
        unsafe {
            env::remove_var("GB_VIDEO_PALETTE_OVERRIDES");
        }
        assert!(
            parse_palette_overrides_from_env()
                .expect("unset env should not fail")
                .is_none()
        );
    }

    #[test]
    fn parse_palette_overrides_from_env_loads_mgba_style_ini_file() {
        let path = env::temp_dir().join("gb-emu-test-palette-overrides.ini");
        std::fs::write(&path, "[gb.override.302017CC]\npal[0]=0x112233\n")
            .expect("temporary override INI should be writable");
        unsafe {
            env::set_var("GB_VIDEO_PALETTE_OVERRIDES", &path);
        }

        let overrides = parse_palette_overrides_from_env()
            .expect("override env should parse")
            .expect("override file should load");
        assert_eq!(overrides.entry_count(), 1);

        unsafe {
            env::remove_var("GB_VIDEO_PALETTE_OVERRIDES");
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn video_palette_selection_auto_promotes_to_sgb_only_after_detected_traffic() {
        assert_eq!(
            VideoPaletteSelection::Auto.resolve(HardwareModel::Dmg, false),
            VideoPalette::Dmg
        );
        assert_eq!(
            VideoPaletteSelection::Auto.resolve(HardwareModel::Mgb, false),
            VideoPalette::Mgb
        );
        assert_eq!(
            VideoPaletteSelection::Auto.resolve(HardwareModel::Sgb, false),
            VideoPalette::Dmg
        );
        assert_eq!(
            VideoPaletteSelection::Auto.resolve(HardwareModel::Dmg, true),
            VideoPalette::Dmg
        );
        assert_eq!(
            VideoPaletteSelection::Auto.resolve(HardwareModel::Sgb, true),
            VideoPalette::Sgb
        );
    }
}
