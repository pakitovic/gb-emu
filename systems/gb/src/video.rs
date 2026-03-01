use crate::hardware::HardwareModel;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub const DMG_CANONICAL_LUMA_LEVELS: [u8; 4] = [0xFF, 0xAA, 0x55, 0x00];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPalettePipeline {
    Dmg4Shade,
    CgbScaffold,
    SgbScaffold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPalette {
    Dmg,
    Mgb,
    Cgb,
    Sgb,
}

impl VideoPalette {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dmg => "dmg",
            Self::Mgb => "mgb",
            Self::Cgb => "cgb",
            Self::Sgb => "sgb",
        }
    }

    pub const fn for_model(model: HardwareModel) -> Self {
        match model {
            HardwareModel::Dmg0 | HardwareModel::Dmg => Self::Dmg,
            HardwareModel::Mgb => Self::Mgb,
            HardwareModel::Sgb | HardwareModel::Sgb2 => Self::Sgb,
        }
    }

    pub const fn pipeline(self) -> VideoPalettePipeline {
        match self {
            Self::Dmg | Self::Mgb => VideoPalettePipeline::Dmg4Shade,
            Self::Cgb => VideoPalettePipeline::CgbScaffold,
            Self::Sgb => VideoPalettePipeline::SgbScaffold,
        }
    }

    pub fn rgb_for_canonical_luma(self, luma: u8) -> [u8; 3] {
        self.rgb_for_dmg_shade_id(canonical_dmg_shade_id_for_luma(luma))
    }

    pub const fn rgb_for_dmg_shade_id(self, shade_id: u8) -> [u8; 3] {
        let index = if shade_id <= 3 { shade_id as usize } else { 3 };
        let shades = match self {
            Self::Dmg => DMG_PALETTE_RGB,
            Self::Mgb => MGB_PALETTE_RGB,
            Self::Cgb => CGB_SCAFFOLD_PALETTE_RGB,
            Self::Sgb => SGB_SCAFFOLD_PALETTE_RGB,
        };
        shades[index]
    }
}

impl Display for VideoPalette {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for VideoPalette {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "dmg" | "dmg-green" | "green" => Ok(Self::Dmg),
            "mgb" | "mgb-gray" | "mgb-grey" | "gray" | "grey" => Ok(Self::Mgb),
            "cgb" | "cgb-scaffold" => Ok(Self::Cgb),
            "sgb" | "sgb2" | "sgb-scaffold" => Ok(Self::Sgb),
            _ => Err(format!(
                "Unsupported palette '{value}'. Supported palettes: dmg, mgb, cgb, sgb"
            )),
        }
    }
}

pub fn canonical_dmg_shade_id_for_luma(luma: u8) -> u8 {
    match luma {
        0xFF => 0,
        0xAA => 1,
        0x55 => 2,
        0x00 => 3,
        _ if luma >= 0xD5 => 0,
        _ if luma >= 0x80 => 1,
        _ if luma >= 0x2B => 2,
        _ => 3,
    }
}

const DMG_PALETTE_RGB: [[u8; 3]; 4] = [
    [0xE0, 0xF8, 0xD0],
    [0x88, 0xC0, 0x70],
    [0x34, 0x68, 0x56],
    [0x08, 0x18, 0x20],
];

const MGB_PALETTE_RGB: [[u8; 3]; 4] = [
    [0xE0, 0xE0, 0xE0],
    [0xA8, 0xA8, 0xA8],
    [0x58, 0x58, 0x58],
    [0x20, 0x20, 0x20],
];

// Placeholder CGB/SGB color profiles to keep wiring stable while DMG-only
// rendering semantics remain active.
const CGB_SCAFFOLD_PALETTE_RGB: [[u8; 3]; 4] = [
    [0xE8, 0xF8, 0xE0],
    [0xB0, 0xC8, 0x80],
    [0x58, 0x70, 0x48],
    [0x20, 0x28, 0x18],
];

const SGB_SCAFFOLD_PALETTE_RGB: [[u8; 3]; 4] = [
    [0xF0, 0xF0, 0xF0],
    [0xB8, 0xB8, 0xB8],
    [0x70, 0x70, 0x70],
    [0x20, 0x20, 0x20],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_follows_hardware_model_family() {
        assert_eq!(
            VideoPalette::for_model(HardwareModel::Dmg0),
            VideoPalette::Dmg
        );
        assert_eq!(
            VideoPalette::for_model(HardwareModel::Dmg),
            VideoPalette::Dmg
        );
        assert_eq!(
            VideoPalette::for_model(HardwareModel::Mgb),
            VideoPalette::Mgb
        );
        assert_eq!(
            VideoPalette::for_model(HardwareModel::Sgb),
            VideoPalette::Sgb
        );
        assert_eq!(
            VideoPalette::for_model(HardwareModel::Sgb2),
            VideoPalette::Sgb
        );
    }

    #[test]
    fn parser_accepts_common_palette_aliases() {
        assert_eq!("dmg".parse::<VideoPalette>().ok(), Some(VideoPalette::Dmg));
        assert_eq!("gray".parse::<VideoPalette>().ok(), Some(VideoPalette::Mgb));
        assert_eq!("cgb".parse::<VideoPalette>().ok(), Some(VideoPalette::Cgb));
        assert_eq!("sgb2".parse::<VideoPalette>().ok(), Some(VideoPalette::Sgb));
    }

    #[test]
    fn parser_rejects_unsupported_palette_name() {
        assert!("sepia".parse::<VideoPalette>().is_err());
    }

    #[test]
    fn canonical_luma_levels_map_to_expected_dmg_shades() {
        assert_eq!(canonical_dmg_shade_id_for_luma(0xFF), 0);
        assert_eq!(canonical_dmg_shade_id_for_luma(0xAA), 1);
        assert_eq!(canonical_dmg_shade_id_for_luma(0x55), 2);
        assert_eq!(canonical_dmg_shade_id_for_luma(0x00), 3);
    }

    #[test]
    fn rgb_mapping_returns_palette_specific_colors() {
        assert_eq!(
            VideoPalette::Dmg.rgb_for_dmg_shade_id(0),
            [0xE0, 0xF8, 0xD0]
        );
        assert_eq!(
            VideoPalette::Mgb.rgb_for_dmg_shade_id(0),
            [0xE0, 0xE0, 0xE0]
        );
        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id(0),
            [0xE8, 0xF8, 0xE0]
        );
    }

    #[test]
    fn pipeline_metadata_marks_scaffold_profiles() {
        assert_eq!(
            VideoPalette::Dmg.pipeline(),
            VideoPalettePipeline::Dmg4Shade
        );
        assert_eq!(
            VideoPalette::Cgb.pipeline(),
            VideoPalettePipeline::CgbScaffold
        );
        assert_eq!(
            VideoPalette::Sgb.pipeline(),
            VideoPalettePipeline::SgbScaffold
        );
    }
}
