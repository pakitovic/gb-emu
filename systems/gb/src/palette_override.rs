use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

pub type PaletteOverrideTripletRgb = [[[u8; 3]; 4]; 3];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaletteOverrideDb {
    entries: BTreeMap<u32, [Option<[u8; 3]>; 12]>,
}

impl PaletteOverrideDb {
    pub fn parse_ini(ini: &str) -> Result<Self, PaletteOverrideParseError> {
        let mut entries = BTreeMap::new();
        let mut current_section_crc32 = None;
        let mut current_colors = [None; 12];

        for (line_index, raw_line) in ini.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if line.starts_with('[') {
                finalize_section(&mut entries, current_section_crc32, &current_colors);
                current_colors = [None; 12];
                current_section_crc32 = parse_section_crc32(line)
                    .map_err(|message| PaletteOverrideParseError::new(line_number, message))?;
                continue;
            }

            let Some(section_crc32) = current_section_crc32 else {
                continue;
            };
            let Some((raw_key, raw_value)) = line.split_once('=') else {
                return Err(PaletteOverrideParseError::new(
                    line_number,
                    "expected key=value entry".to_string(),
                ));
            };

            let key = raw_key.trim();
            let value = raw_value.trim();
            let Some(color_index) = parse_palette_color_index(key) else {
                let _ = section_crc32;
                continue;
            };
            let color = parse_rgb888_value(value)
                .map_err(|message| PaletteOverrideParseError::new(line_number, message))?;
            apply_mgba_color_override_write(&mut current_colors, color_index, color);
        }

        finalize_section(&mut entries, current_section_crc32, &current_colors);
        Ok(Self { entries })
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn merged_cgb_palette_triplet_rgb(
        &self,
        header_crc32: u32,
        base_triplet: PaletteOverrideTripletRgb,
    ) -> Option<PaletteOverrideTripletRgb> {
        let colors = self.entries.get(&header_crc32)?;
        let mut merged = base_triplet;
        for (index, color) in colors.iter().enumerate() {
            let Some(color) = color else {
                continue;
            };
            merged[index / 4][index % 4] = *color;
        }
        Some(merged)
    }

    pub fn merged_sgb_boot_palette_rgb(
        &self,
        header_crc32: u32,
        base_palette: [[u8; 3]; 4],
    ) -> Option<[[u8; 3]; 4]> {
        let colors = self.entries.get(&header_crc32)?;
        let mut merged = base_palette;
        for (index, color) in colors.iter().take(4).enumerate() {
            let Some(color) = color else {
                continue;
            };
            merged[index] = *color;
        }
        Some(merged)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteOverrideParseError {
    line_number: usize,
    message: String,
}

impl PaletteOverrideParseError {
    fn new(line_number: usize, message: String) -> Self {
        Self {
            line_number,
            message,
        }
    }
}

impl Display for PaletteOverrideParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line_number, self.message)
    }
}

impl std::error::Error for PaletteOverrideParseError {}

fn finalize_section(
    entries: &mut BTreeMap<u32, [Option<[u8; 3]>; 12]>,
    section_crc32: Option<u32>,
    colors: &[Option<[u8; 3]>; 12],
) {
    let Some(section_crc32) = section_crc32 else {
        return;
    };
    if colors.iter().any(Option::is_some) {
        entries.insert(section_crc32, *colors);
    }
}

fn parse_section_crc32(line: &str) -> Result<Option<u32>, String> {
    if !line.ends_with(']') {
        return Err("section header must end with ']'".to_string());
    }
    let name = line
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| "section header must be enclosed in '[' and ']'".to_string())?
        .trim();
    let normalized = name.to_ascii_lowercase();
    let Some(raw_crc32) = normalized.strip_prefix("gb.override.") else {
        return Ok(None);
    };
    if raw_crc32.is_empty() || raw_crc32.len() > 8 {
        return Err("gb.override section must contain 1 to 8 hex digits".to_string());
    }
    u32::from_str_radix(raw_crc32, 16)
        .map(Some)
        .map_err(|_| "gb.override section must contain a valid hex CRC32".to_string())
}

fn parse_palette_color_index(key: &str) -> Option<usize> {
    let normalized = key.trim().to_ascii_lowercase();
    let index = normalized
        .strip_prefix("pal[")
        .and_then(|value| value.strip_suffix(']'))?
        .parse::<usize>()
        .ok()?;
    (index < 12).then_some(index)
}

fn parse_rgb888_value(value: &str) -> Result<[u8; 3], String> {
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u32::from_str_radix(hex, 16).map_err(|_| format!("invalid hex RGB888 value '{trimmed}'"))?
    } else {
        trimmed
            .parse::<u32>()
            .map_err(|_| format!("invalid RGB888 value '{trimmed}'"))?
    };

    if parsed > 0x00FF_FFFF {
        return Err(format!("RGB888 value '{trimmed}' exceeds 24-bit range"));
    }

    Ok([
        ((parsed >> 16) & 0xFF) as u8,
        ((parsed >> 8) & 0xFF) as u8,
        (parsed & 0xFF) as u8,
    ])
}

fn apply_mgba_color_override_write(
    colors: &mut [Option<[u8; 3]>; 12],
    index: usize,
    color: [u8; 3],
) {
    colors[index] = Some(color);
    if index < 8 {
        colors[index + 4] = Some(color);
    }
    if index < 4 {
        colors[index + 8] = Some(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_accepts_mgba_style_override_sections_and_palette_entries() {
        let db = PaletteOverrideDb::parse_ini(
            r#"
            [gb.override.302017CC]
            pal[0]=0x112233
            pal[4]=0x445566
            pal[8]=0x778899
            "#,
        )
        .expect("override INI should parse");

        let merged = db
            .merged_cgb_palette_triplet_rgb(
                0x3020_17CC,
                [[[0, 0, 0]; 4], [[0, 0, 0]; 4], [[0, 0, 0]; 4]],
            )
            .expect("override should resolve");
        assert_eq!(merged[0][0], [0x11, 0x22, 0x33]);
        assert_eq!(merged[1][0], [0x44, 0x55, 0x66]);
        assert_eq!(merged[2][0], [0x77, 0x88, 0x99]);
    }

    #[test]
    fn parser_applies_mgba_partial_palette_inheritance_rules() {
        let db = PaletteOverrideDb::parse_ini(
            r#"
            [gb.override.302017CC]
            pal[1]=0x010203
            "#,
        )
        .expect("override INI should parse");

        let merged = db
            .merged_cgb_palette_triplet_rgb(
                0x3020_17CC,
                [[[0, 0, 0]; 4], [[0, 0, 0]; 4], [[0, 0, 0]; 4]],
            )
            .expect("override should resolve");
        assert_eq!(merged[0][1], [0x01, 0x02, 0x03]);
        assert_eq!(merged[1][1], [0x01, 0x02, 0x03]);
        assert_eq!(merged[2][1], [0x01, 0x02, 0x03]);
    }

    #[test]
    fn parser_can_merge_sgb_boot_palette_from_first_palette_quartet() {
        let db = PaletteOverrideDb::parse_ini(
            r#"
            [gb.override.302017CC]
            pal[1]=0x010203
            "#,
        )
        .expect("override INI should parse");

        let merged = db
            .merged_sgb_boot_palette_rgb(
                0x3020_17CC,
                [
                    [0x10, 0x20, 0x30],
                    [0x40, 0x50, 0x60],
                    [0x70, 0x80, 0x90],
                    [0xA0, 0xB0, 0xC0],
                ],
            )
            .expect("override should resolve");
        assert_eq!(merged[0], [0x10, 0x20, 0x30]);
        assert_eq!(merged[1], [0x01, 0x02, 0x03]);
        assert_eq!(merged[2], [0x70, 0x80, 0x90]);
        assert_eq!(merged[3], [0xA0, 0xB0, 0xC0]);
    }

    #[test]
    fn parser_ignores_unknown_sections_and_keys() {
        let db = PaletteOverrideDb::parse_ini(
            r#"
            [other.section]
            value=1
            [gb.override.1234]
            model=sgb
            pal[0]=0xAABBCC
            "#,
        )
        .expect("override INI should parse");

        assert_eq!(db.entry_count(), 1);
        assert!(
            db.merged_cgb_palette_triplet_rgb(
                0x0000_1234,
                [[[0, 0, 0]; 4], [[0, 0, 0]; 4], [[0, 0, 0]; 4]]
            )
            .is_some()
        );
    }

    #[test]
    fn parser_reports_invalid_values_with_line_number() {
        let err = PaletteOverrideDb::parse_ini(
            r#"
            [gb.override.302017CC]
            pal[0]=oops
            "#,
        )
        .expect_err("invalid RGB value should fail");

        assert_eq!(err.to_string(), "line 3: invalid RGB888 value 'oops'");
    }
}
