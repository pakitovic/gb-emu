use gb_emu::cartridge::CartridgeMetadata;
use gb_emu::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::video::VideoPalette;
use sdl2::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
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

pub(super) fn render_grayscale_frame(
    texture: &mut Texture<'_>,
    canvas: &mut Canvas<Window>,
    frame: &[u8],
    palette: VideoPalette,
) -> Result<(), io::Error> {
    texture
        .with_lock(None, |bytes, pitch| {
            for y in 0..SCREEN_HEIGHT {
                for x in 0..SCREEN_WIDTH {
                    let shade = frame[y * SCREEN_WIDTH + x];
                    let rgb = palette.rgb_for_canonical_luma(shade);
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
