use super::WebEmulator;
use gb_emu::gameboy::{SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::palette_override::PaletteOverrideDb;
use gb_emu::video::{VideoPalette, VideoPalettePipeline};
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
    fn effective_video_palette(&mut self) -> VideoPalette {
        match self.video_palette_selection {
            VideoPaletteSelection::Auto
                if self.session.sgb_active()
                    && self
                        .session
                        .gameboy()
                        .hardware_model()
                        .supports_sgb_features() =>
            {
                VideoPalette::Sgb
            }
            VideoPaletteSelection::Auto => self.default_video_palette,
            VideoPaletteSelection::Palette(palette) => palette,
        }
    }

    fn active_screen_dimensions(&mut self) -> (u32, u32) {
        if self.effective_video_palette().pipeline() == VideoPalettePipeline::SgbRuntime
            && let Some((width, height)) = self.session.sgb_presented_frame_size()
        {
            return (width as u32, height as u32);
        }

        (SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
    }

    pub fn screen_width(&mut self) -> u32 {
        let (width, _) = self.active_screen_dimensions();
        width
    }

    pub fn screen_height(&mut self) -> u32 {
        let (_, height) = self.active_screen_dimensions();
        height
    }

    pub fn grayscale_frame(&self) -> Vec<u8> {
        self.session.gameboy().framebuffer().to_vec()
    }

    pub fn video_palette(&mut self) -> String {
        self.effective_video_palette().as_str().to_string()
    }

    pub fn set_video_palette(&mut self, palette: &str) -> Result<(), JsValue> {
        self.video_palette_selection = parse_video_palette_selection(palette)
            .map_err(|message| JsValue::from_str(&message))?;
        Ok(())
    }

    #[wasm_bindgen(js_name = setPaletteOverridesIni)]
    pub fn set_palette_overrides_ini(&mut self, ini: &str) -> Result<(), JsValue> {
        let overrides =
            PaletteOverrideDb::parse_ini(ini).map_err(|err| JsValue::from_str(&err.to_string()))?;
        self.palette_overrides = Some(overrides);
        self.session
            .apply_palette_overrides(self.palette_overrides.as_ref());
        Ok(())
    }

    #[wasm_bindgen(js_name = clearPaletteOverrides)]
    pub fn clear_palette_overrides(&mut self) {
        self.palette_overrides = None;
        self.session.apply_palette_overrides(None);
    }

    #[wasm_bindgen(js_name = paletteOverrideCount)]
    pub fn palette_override_count(&self) -> u32 {
        self.palette_overrides
            .as_ref()
            .map_or(0, |overrides| overrides.entry_count() as u32)
    }

    pub fn rgba_frame(&mut self) -> Vec<u8> {
        let video_palette = self.effective_video_palette();
        if video_palette.pipeline() == VideoPalettePipeline::SgbRuntime
            && let Some(rgb24_frame) = self.session.sgb_presented_rgb_frame()
        {
            let mut rgba = vec![0; rgb24_frame.len() / 3 * 4];
            for (pixel_index, rgb) in rgb24_frame.chunks_exact(3).enumerate() {
                let rgba_index = pixel_index * 4;
                rgba[rgba_index] = rgb[0];
                rgba[rgba_index + 1] = rgb[1];
                rgba[rgba_index + 2] = rgb[2];
                rgba[rgba_index + 3] = 0xFF;
            }
            return rgba;
        }

        let frame = self.session.gameboy().framebuffer();
        let palette_selectors = self.session.gameboy().framebuffer_palette_selectors();
        let header_crc32 = self.session.gameboy().rom_header_crc32();
        let palette_overrides = self.palette_overrides.as_ref();
        let mut rgba = vec![0; frame.len() * 4];
        for (i, luma) in frame.iter().copied().enumerate() {
            let selector = palette_selectors.get(i).copied().unwrap_or(1);
            let rgb = video_palette.rgb_for_framebuffer_pixel_with_overrides(
                luma,
                selector,
                header_crc32,
                palette_overrides,
            );
            let rgba_index = i * 4;
            rgba[rgba_index] = rgb[0];
            rgba[rgba_index + 1] = rgb[1];
            rgba[rgba_index + 2] = rgb[2];
            rgba[rgba_index + 3] = 0xFF;
        }
        rgba
    }
}
