use crate::hardware::HardwareModel;
use crate::palette_db::{CGB_BASE_PALETTES_RGB, CGB_GAME_OVERRIDES};
use crate::palette_override::{PaletteOverrideDb, PaletteOverrideTripletRgb};
use crate::sgb::{bgr555_to_rgb888, sgb_built_in_boot_palette_rgb888};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub const DMG_CANONICAL_LUMA_LEVELS: [u8; 4] = [0xFF, 0xAA, 0x55, 0x00];

const PALETTE_SELECTOR_BG: u8 = 1;
const PALETTE_SELECTOR_OBJ0: u8 = 2;
const PALETTE_SELECTOR_OBJ1: u8 = 3;
const CGB_AUTO_FALLBACK_PALETTE_IDS: [u8; 3] = [0, 0, 0];

const GB_COLOR_PRESET_KEYS: [&str; 15] = [
    "grayscale",
    "gb-pocket",
    "gb-light",
    "cgb-brown",
    "cgb-red-a",
    "cgb-dark-brown-b",
    "cgb-pale-yellow",
    "cgb-orange-a",
    "cgb-yellow-b",
    "cgb-blue",
    "cgb-dark-blue-a",
    "cgb-gray-b",
    "cgb-green",
    "cgb-dark-green-a",
    "cgb-reverse-b",
];

const GB_COLOR_PRESET_LABELS: [&str; 15] = [
    "grayscale",
    "gb pocket",
    "gb light",
    "cgb brown",
    "cgb red A",
    "cgb dark brown B",
    "cgb pale yellow",
    "cgb orange A",
    "cgb yellow B",
    "cgb blue",
    "cgb dark blue A",
    "cgb gray B",
    "cgb green",
    "cgb dark green A",
    "cgb reverse B",
];

const SGB_BUILT_IN_PALETTE_KEYS: [&str; 32] = [
    "sgb-1a", "sgb-1b", "sgb-1c", "sgb-1d", "sgb-1e", "sgb-1f", "sgb-1g", "sgb-1h", "sgb-2a",
    "sgb-2b", "sgb-2c", "sgb-2d", "sgb-2e", "sgb-2f", "sgb-2g", "sgb-2h", "sgb-3a", "sgb-3b",
    "sgb-3c", "sgb-3d", "sgb-3e", "sgb-3f", "sgb-3g", "sgb-3h", "sgb-4a", "sgb-4b", "sgb-4c",
    "sgb-4d", "sgb-4e", "sgb-4f", "sgb-4g", "sgb-4h",
];

const SGB_BUILT_IN_PALETTE_LABELS: [&str; 32] = [
    "sgb 1-A", "sgb 1-B", "sgb 1-C", "sgb 1-D", "sgb 1-E", "sgb 1-F", "sgb 1-G", "sgb 1-H",
    "sgb 2-A", "sgb 2-B", "sgb 2-C", "sgb 2-D", "sgb 2-E", "sgb 2-F", "sgb 2-G", "sgb 2-H",
    "sgb 3-A", "sgb 3-B", "sgb 3-C", "sgb 3-D", "sgb 3-E", "sgb 3-F", "sgb 3-G", "sgb 3-H",
    "sgb 4-A", "sgb 4-B", "sgb 4-C", "sgb 4-D", "sgb 4-E", "sgb 4-F", "sgb 4-G", "sgb 4-H",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPalettePipeline {
    Dmg4Shade,
    SgbRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GbColorPresetId(u8);

impl GbColorPresetId {
    pub const COUNT: usize = GB_COLOR_PRESET_KEYS.len();

    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT {
            Some(Self(index as u8))
        } else {
            None
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub fn key(self) -> &'static str {
        GB_COLOR_PRESET_KEYS[self.index()]
    }

    pub fn label(self) -> &'static str {
        GB_COLOR_PRESET_LABELS[self.index()]
    }

    fn from_key(value: &str) -> Option<Self> {
        GB_COLOR_PRESET_KEYS
            .iter()
            .position(|key| *key == value)
            .and_then(Self::from_index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SgbBuiltInPaletteId(u8);

impl SgbBuiltInPaletteId {
    pub const COUNT: usize = SGB_BUILT_IN_PALETTE_KEYS.len();

    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT {
            Some(Self(index as u8))
        } else {
            None
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub fn key(self) -> &'static str {
        SGB_BUILT_IN_PALETTE_KEYS[self.index()]
    }

    pub fn label(self) -> &'static str {
        SGB_BUILT_IN_PALETTE_LABELS[self.index()]
    }

    fn from_key(value: &str) -> Option<Self> {
        SGB_BUILT_IN_PALETTE_KEYS
            .iter()
            .position(|key| *key == value)
            .and_then(Self::from_index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPalette {
    Dmg,
    Mgb,
    Cgb,
    Preset(GbColorPresetId),
    Sgb,
    SgbBuiltIn(SgbBuiltInPaletteId),
}

impl VideoPalette {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dmg => "dmg",
            Self::Mgb => "mgb",
            Self::Cgb => "cgb",
            Self::Preset(preset) => preset.key(),
            Self::Sgb => "sgb",
            Self::SgbBuiltIn(palette) => palette.key(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dmg => "dmg green",
            Self::Mgb => "mgb gray",
            Self::Cgb => "cgb auto",
            Self::Preset(preset) => preset.label(),
            Self::Sgb => "sgb runtime",
            Self::SgbBuiltIn(palette) => palette.label(),
        }
    }

    pub const fn for_model(model: HardwareModel) -> Self {
        match model {
            HardwareModel::Dmg0 | HardwareModel::Dmg => Self::Dmg,
            HardwareModel::Mgb => Self::Mgb,
            HardwareModel::Sgb | HardwareModel::Sgb2 => Self::Sgb,
        }
    }

    pub const fn auto_base_for_model(model: HardwareModel) -> Self {
        match model {
            HardwareModel::Sgb | HardwareModel::Sgb2 => Self::Dmg,
            _ => Self::for_model(model),
        }
    }

    pub const fn pipeline(self) -> VideoPalettePipeline {
        match self {
            Self::Sgb => VideoPalettePipeline::SgbRuntime,
            _ => VideoPalettePipeline::Dmg4Shade,
        }
    }

    pub fn rgb_for_canonical_luma(self, luma: u8) -> [u8; 3] {
        self.rgb_for_framebuffer_pixel(luma, PALETTE_SELECTOR_BG, 0)
    }

    pub fn rgb_for_framebuffer_pixel(
        self,
        luma: u8,
        palette_selector_code: u8,
        header_crc32: u32,
    ) -> [u8; 3] {
        self.rgb_for_framebuffer_pixel_with_overrides(
            luma,
            palette_selector_code,
            header_crc32,
            None,
        )
    }

    pub fn rgb_for_framebuffer_pixel_with_overrides(
        self,
        luma: u8,
        palette_selector_code: u8,
        header_crc32: u32,
        overrides: Option<&PaletteOverrideDb>,
    ) -> [u8; 3] {
        self.rgb_for_dmg_shade_id_with_context(
            canonical_dmg_shade_id_for_luma(luma),
            palette_selector_code,
            header_crc32,
            overrides,
        )
    }

    pub fn rgb_for_dmg_shade_id(self, shade_id: u8) -> [u8; 3] {
        self.rgb_for_dmg_shade_id_with_context(shade_id, PALETTE_SELECTOR_BG, 0, None)
    }

    pub fn rgb_for_dmg_shade_id_with_context(
        self,
        shade_id: u8,
        palette_selector_code: u8,
        header_crc32: u32,
        overrides: Option<&PaletteOverrideDb>,
    ) -> [u8; 3] {
        let shade_index = shade_id.min(3) as usize;
        let palette_triplet = self.palette_triplet_rgb(header_crc32, overrides);
        palette_triplet[palette_triplet_index_for_selector(palette_selector_code)][shade_index]
    }

    fn palette_triplet_rgb(
        self,
        header_crc32: u32,
        overrides: Option<&PaletteOverrideDb>,
    ) -> PaletteOverrideTripletRgb {
        match self {
            Self::Dmg => uniform_palette_triplet(DMG_PALETTE_RGB),
            Self::Mgb => uniform_palette_triplet(MGB_PALETTE_RGB),
            Self::Cgb => {
                let base = cgb_palette_triplet_from_ids(
                    cgb_game_override_palette_ids(header_crc32)
                        .unwrap_or(CGB_AUTO_FALLBACK_PALETTE_IDS),
                );
                overrides
                    .and_then(|db| db.merged_cgb_palette_triplet_rgb(header_crc32, base))
                    .unwrap_or(base)
            }
            Self::Preset(preset) => gb_color_preset_triplet_rgb(preset),
            Self::Sgb => uniform_palette_triplet(
                sgb_built_in_boot_palette_rgb888(0)
                    .expect("default SGB built-in palette should exist"),
            ),
            Self::SgbBuiltIn(palette) => uniform_palette_triplet(
                sgb_built_in_boot_palette_rgb888(palette.index())
                    .expect("valid SGB built-in palette index should resolve"),
            ),
        }
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
        let normalized = value.to_ascii_lowercase();
        match normalized.as_str() {
            "dmg" | "dmg-green" | "green" => Ok(Self::Dmg),
            "mgb" | "mgb-gray" | "mgb-grey" | "gray" | "grey" => Ok(Self::Mgb),
            "cgb" | "cgb-auto" | "cgb-scaffold" => Ok(Self::Cgb),
            "sgb" | "sgb2" | "sgb-runtime" | "sgb-scaffold" => Ok(Self::Sgb),
            _ => GbColorPresetId::from_key(normalized.as_str())
                .map(Self::Preset)
                .or_else(|| SgbBuiltInPaletteId::from_key(normalized.as_str()).map(Self::SgbBuiltIn))
                .ok_or_else(|| {
                    format!(
                        "Unsupported palette '{value}'. Supported palettes: dmg, mgb, cgb, sgb, grayscale, gb-pocket, gb-light, cgb-brown..cgb-reverse-b, sgb-1a..sgb-4h"
                    )
                }),
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

fn palette_triplet_index_for_selector(selector_code: u8) -> usize {
    match selector_code {
        PALETTE_SELECTOR_OBJ0 => 1,
        PALETTE_SELECTOR_OBJ1 => 2,
        _ => 0,
    }
}

fn uniform_palette_triplet(colors: [[u8; 3]; 4]) -> PaletteOverrideTripletRgb {
    [colors, colors, colors]
}

fn rgb888_palette_from_bgr555(colors: [u16; 4]) -> [[u8; 3]; 4] {
    colors.map(bgr555_to_rgb888)
}

fn cgb_base_palette_rgb(index: u8) -> [[u8; 3]; 4] {
    CGB_BASE_PALETTES_RGB
        .get(index as usize)
        .copied()
        .unwrap_or(CGB_BASE_PALETTES_RGB[0])
}

fn cgb_palette_triplet_from_ids(ids: [u8; 3]) -> PaletteOverrideTripletRgb {
    [
        cgb_base_palette_rgb(ids[0]),
        cgb_base_palette_rgb(ids[1]),
        cgb_base_palette_rgb(ids[2]),
    ]
}

fn cgb_game_override_palette_ids(header_crc32: u32) -> Option<[u8; 3]> {
    CGB_GAME_OVERRIDES
        .iter()
        .find_map(|(candidate_crc32, palette_ids)| {
            (*candidate_crc32 == header_crc32).then_some(*palette_ids)
        })
}

fn gb_color_preset_triplet_rgb(preset: GbColorPresetId) -> PaletteOverrideTripletRgb {
    match preset.index() {
        0 => uniform_palette_triplet(rgb888_palette_from_bgr555([0x7FFF, 0x56B5, 0x294A, 0x0000])),
        1 => uniform_palette_triplet(rgb888_palette_from_bgr555([0x52D4, 0x4270, 0x2989, 0x10A3])),
        2 => uniform_palette_triplet(rgb888_palette_from_bgr555([0x7FCF, 0x738B, 0x56C3, 0x39E0])),
        3 => cgb_palette_triplet_from_ids([0, 0, 0]),
        4 => cgb_palette_triplet_from_ids([4, 3, 28]),
        5 => cgb_palette_triplet_from_ids([1, 0, 0]),
        6 => cgb_palette_triplet_from_ids([12, 12, 12]),
        7 => cgb_palette_triplet_from_ids([24, 24, 24]),
        8 => cgb_palette_triplet_from_ids([6, 28, 3]),
        9 => cgb_palette_triplet_from_ids([28, 4, 3]),
        10 => cgb_palette_triplet_from_ids([2, 4, 0]),
        11 => cgb_palette_triplet_from_ids([5, 5, 5]),
        12 => cgb_palette_triplet_from_ids([18, 18, 18]),
        13 => cgb_palette_triplet_from_ids([29, 4, 4]),
        14 => cgb_palette_triplet_from_ids([27, 27, 27]),
        _ => cgb_palette_triplet_from_ids(CGB_AUTO_FALLBACK_PALETTE_IDS),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette_override::PaletteOverrideDb;

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
    fn auto_base_palette_uses_dmg_until_sgb_runtime_is_active() {
        assert_eq!(
            VideoPalette::auto_base_for_model(HardwareModel::Dmg),
            VideoPalette::Dmg
        );
        assert_eq!(
            VideoPalette::auto_base_for_model(HardwareModel::Mgb),
            VideoPalette::Mgb
        );
        assert_eq!(
            VideoPalette::auto_base_for_model(HardwareModel::Sgb),
            VideoPalette::Dmg
        );
        assert_eq!(
            VideoPalette::auto_base_for_model(HardwareModel::Sgb2),
            VideoPalette::Dmg
        );
    }

    #[test]
    fn parser_accepts_common_palette_aliases() {
        assert_eq!("dmg".parse::<VideoPalette>().ok(), Some(VideoPalette::Dmg));
        assert_eq!("gray".parse::<VideoPalette>().ok(), Some(VideoPalette::Mgb));
        assert_eq!("cgb".parse::<VideoPalette>().ok(), Some(VideoPalette::Cgb));
        assert_eq!("sgb2".parse::<VideoPalette>().ok(), Some(VideoPalette::Sgb));
        assert_eq!(
            "cgb-blue".parse::<VideoPalette>().ok(),
            Some(VideoPalette::Preset(
                GbColorPresetId::from_index(9).expect("preset id should exist"),
            ))
        );
        assert_eq!(
            "sgb-2c".parse::<VideoPalette>().ok(),
            Some(VideoPalette::SgbBuiltIn(
                SgbBuiltInPaletteId::from_index(10).expect("palette id should exist"),
            ))
        );
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
            VideoPalette::Preset(GbColorPresetId::from_index(0).expect("preset id should exist"),)
                .rgb_for_dmg_shade_id(1),
            [0xAD, 0xAD, 0xAD]
        );
        assert_eq!(
            VideoPalette::SgbBuiltIn(
                SgbBuiltInPaletteId::from_index(10).expect("palette id should exist")
            )
            .rgb_for_dmg_shade_id(0),
            [0xFF, 0xC6, 0xFF]
        );
    }

    #[test]
    fn cgb_auto_uses_header_crc_override_palette_triplet() {
        let kirby_crc32 = 0x3020_17CC;
        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id_with_context(
                0,
                PALETTE_SELECTOR_BG,
                kirby_crc32,
                None,
            ),
            [0xA5, 0x9C, 0xFF]
        );
        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id_with_context(
                0,
                PALETTE_SELECTOR_OBJ0,
                kirby_crc32,
                None,
            ),
            [0xFF, 0x63, 0x52]
        );
        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id_with_context(
                0,
                PALETTE_SELECTOR_OBJ1,
                kirby_crc32,
                None,
            ),
            [0x00, 0x00, 0xFF]
        );
    }

    #[test]
    fn cgb_auto_falls_back_to_default_brown_triplet_without_override() {
        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id_with_context(1, PALETTE_SELECTOR_BG, 0, None,),
            CGB_BASE_PALETTES_RGB[0][1]
        );
    }

    #[test]
    fn cgb_auto_prefers_external_palette_override_db_over_embedded_table() {
        let overrides = PaletteOverrideDb::parse_ini(
            "[gb.override.302017CC]\npal[0]=0x112233\npal[4]=0x445566\npal[8]=0x778899\n",
        )
        .expect("override INI should parse");

        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id_with_context(
                0,
                PALETTE_SELECTOR_BG,
                0x3020_17CC,
                Some(&overrides),
            ),
            [0x11, 0x22, 0x33]
        );
        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id_with_context(
                0,
                PALETTE_SELECTOR_OBJ0,
                0x3020_17CC,
                Some(&overrides),
            ),
            [0x44, 0x55, 0x66]
        );
        assert_eq!(
            VideoPalette::Cgb.rgb_for_dmg_shade_id_with_context(
                0,
                PALETTE_SELECTOR_OBJ1,
                0x3020_17CC,
                Some(&overrides),
            ),
            [0x77, 0x88, 0x99]
        );
    }

    #[test]
    fn built_in_palette_variants_stay_on_dmg_pipeline() {
        assert_eq!(
            VideoPalette::SgbBuiltIn(
                SgbBuiltInPaletteId::from_index(0).expect("palette id should exist")
            )
            .pipeline(),
            VideoPalettePipeline::Dmg4Shade
        );
        assert_eq!(
            VideoPalette::Preset(GbColorPresetId::from_index(9).expect("preset id should exist"),)
                .pipeline(),
            VideoPalettePipeline::Dmg4Shade
        );
    }

    #[test]
    fn pipeline_metadata_marks_runtime_only_sgb_profile() {
        assert_eq!(
            VideoPalette::Dmg.pipeline(),
            VideoPalettePipeline::Dmg4Shade
        );
        assert_eq!(
            VideoPalette::Cgb.pipeline(),
            VideoPalettePipeline::Dmg4Shade
        );
        assert_eq!(
            VideoPalette::Sgb.pipeline(),
            VideoPalettePipeline::SgbRuntime
        );
    }
}
