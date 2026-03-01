use super::WebEmulator;
use gb_emu::gameboy::{SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::video::VideoPalette;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoPaletteSelection {
    Auto,
    Palette(VideoPalette),
}

pub(super) fn parse_video_palette_selection(value: &str) -> Result<VideoPaletteSelection, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return Ok(VideoPaletteSelection::Auto);
    }

    trimmed
        .parse::<VideoPalette>()
        .map(VideoPaletteSelection::Palette)
}

#[wasm_bindgen]
impl WebEmulator {
    pub fn screen_width(&self) -> u32 {
        SCREEN_WIDTH as u32
    }

    pub fn screen_height(&self) -> u32 {
        SCREEN_HEIGHT as u32
    }

    pub fn grayscale_frame(&self) -> Vec<u8> {
        self.session.gameboy().framebuffer().to_vec()
    }

    pub fn video_palette(&self) -> String {
        self.active_video_palette.as_str().to_string()
    }

    pub fn set_video_palette(&mut self, palette: &str) -> Result<(), JsValue> {
        let selection = parse_video_palette_selection(palette)
            .map_err(|message| JsValue::from_str(&message))?;
        self.active_video_palette = match selection {
            VideoPaletteSelection::Auto => self.default_video_palette,
            VideoPaletteSelection::Palette(palette) => palette,
        };
        Ok(())
    }

    pub fn rgba_frame(&self) -> Vec<u8> {
        let frame = self.session.gameboy().framebuffer();
        let mut rgba = vec![0; frame.len() * 4];
        for (i, luma) in frame.iter().copied().enumerate() {
            let rgb = self.active_video_palette.rgb_for_canonical_luma(luma);
            let rgba_index = i * 4;
            rgba[rgba_index] = rgb[0];
            rgba[rgba_index + 1] = rgb[1];
            rgba[rgba_index + 2] = rgb[2];
            rgba[rgba_index + 3] = 0xFF;
        }
        rgba
    }
}
