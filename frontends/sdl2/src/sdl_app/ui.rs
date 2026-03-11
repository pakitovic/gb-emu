use gb_emu::cartridge::CartridgeMetadata;
use gb_emu::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::palette_override::PaletteOverrideDb;
use gb_emu::video::VideoPalette;
use sdl2::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use std::io;

pub(super) fn build_window_title(gb: &GameBoy, cartridge_metadata: &CartridgeMetadata) -> String {
    format!(
        "gb-emu SDL2 | {} | {} | warnings {} | F1 cart-info",
        gb.rom_title(),
        cartridge_metadata.mapper,
        cartridge_metadata.header_warnings.len()
    )
}

pub(super) fn show_cartridge_info_dialog(cartridge_debug_report: &str) {
    if let Err(err) = show_simple_message_box(
        MessageBoxFlag::INFORMATION,
        "Cartridge metadata",
        cartridge_debug_report,
        None,
    ) {
        eprintln!("SDL2 cart-info panel failed: {err}");
    }
}

pub(super) fn create_rgb24_texture<'a>(
    texture_creator: &'a TextureCreator<WindowContext>,
    width: u32,
    height: u32,
) -> Result<Texture<'a>, io::Error> {
    texture_creator
        .create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, width, height)
        .map_err(io::Error::other)
}

pub(super) fn render_grayscale_frame(
    texture: &mut Texture<'_>,
    canvas: &mut Canvas<Window>,
    frame: &[u8],
    palette_selectors: &[u8],
    palette: VideoPalette,
    header_crc32: u32,
    palette_overrides: Option<&PaletteOverrideDb>,
) -> Result<(), io::Error> {
    texture
        .with_lock(None, |bytes, pitch| {
            for y in 0..SCREEN_HEIGHT {
                for x in 0..SCREEN_WIDTH {
                    let pixel_index = y * SCREEN_WIDTH + x;
                    let shade = frame[pixel_index];
                    let selector = palette_selectors.get(pixel_index).copied().unwrap_or(1);
                    let rgb = palette.rgb_for_framebuffer_pixel_with_overrides(
                        shade,
                        selector,
                        header_crc32,
                        palette_overrides,
                    );
                    let offset = y * pitch + x * 3;
                    bytes[offset] = rgb[0];
                    bytes[offset + 1] = rgb[1];
                    bytes[offset + 2] = rgb[2];
                }
            }
        })
        .map_err(io::Error::other)?;

    canvas.clear();
    canvas.copy(texture, None, None).map_err(io::Error::other)?;
    canvas.present();

    Ok(())
}

pub(super) fn render_rgb24_frame(
    texture: &mut Texture<'_>,
    canvas: &mut Canvas<Window>,
    frame_rgb24: &[u8],
) -> Result<(), io::Error> {
    let texture_query = texture.query();
    let row_bytes = texture_query.width as usize * 3;
    let expected_len = row_bytes * texture_query.height as usize;
    if frame_rgb24.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "RGB24 frame length mismatch: expected {expected_len} bytes for {}x{}, got {}",
                texture_query.width,
                texture_query.height,
                frame_rgb24.len()
            ),
        ));
    }

    texture
        .with_lock(None, |bytes, pitch| {
            for y in 0..texture_query.height as usize {
                let src_start = y * row_bytes;
                let src_end = src_start + row_bytes;
                let dst_start = y * pitch;
                let dst_end = dst_start + row_bytes;
                bytes[dst_start..dst_end].copy_from_slice(&frame_rgb24[src_start..src_end]);
            }
        })
        .map_err(io::Error::other)?;

    canvas.clear();
    canvas.copy(texture, None, None).map_err(io::Error::other)?;
    canvas.present();

    Ok(())
}
