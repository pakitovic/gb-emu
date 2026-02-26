use super::support::*;

#[test]
fn ppu_cgb_scaffold_runtime_is_model_gated_off_for_current_dmg_family_models() {
    for model in [
        HardwareModel::Dmg0,
        HardwareModel::Dmg,
        HardwareModel::Mgb,
        HardwareModel::Sgb,
        HardwareModel::Sgb2,
    ] {
        let bus = make_test_bus_with_model(model);
        assert!(
            !bus.debug_ppu_cgb_scaffold_runtime_enabled(),
            "PPU CGB scaffold runtime must stay gated off for current DMG-family model {model:?}"
        );
    }
}

#[test]
fn mode3_pixel_metadata_mixer_keeps_dmg_priority_and_palette_selection() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF47, 0b11_10_01_00); // BGP: c0->0, c1->1, c2->2, c3->3
    bus.write_byte(0xFF48, 0b00_11_10_01); // OBP0 distinct mapping
    bus.write_byte(0xFF49, 0b01_00_11_10); // OBP1 distinct mapping

    let (source_obj, color_id, palette_code, obj_behind_bg, bg_nonzero, shade_id) =
        bus.debug_compose_mode3_pixel_metadata_and_shade(0x93, 2, 1, 0x00);
    assert_eq!(
        source_obj, 1,
        "OBJ should win when not hidden by BG priority"
    );
    assert_eq!(color_id, 1);
    assert_eq!(palette_code, 2, "OBJ palette select should choose OBP0");
    assert!(!obj_behind_bg);
    assert!(bg_nonzero);
    assert_eq!(
        shade_id, 2,
        "OBP0 color1 should map through final DMG color step"
    );

    let (source_obj, color_id, palette_code, obj_behind_bg, bg_nonzero, shade_id) =
        bus.debug_compose_mode3_pixel_metadata_and_shade(0x93, 2, 1, 0x80);
    assert_eq!(
        source_obj, 0,
        "BG should win when OBJ is behind non-zero BG"
    );
    assert_eq!(color_id, 2);
    assert_eq!(palette_code, 1, "BG path should select BGP");
    assert!(obj_behind_bg);
    assert!(bg_nonzero);
    assert_eq!(
        shade_id, 2,
        "BGP color2 should map through final DMG color step"
    );
}

#[test]
fn mode3_pixel_metadata_forces_white_backdrop_when_bg_is_disabled() {
    let mut bus = make_test_bus();
    bus.write_byte(0xFF47, 0b00_00_00_11); // Would make color0 non-white if BGP were used.

    let (source_obj, color_id, palette_code, obj_behind_bg, bg_nonzero, shade_id) =
        bus.debug_compose_mode3_pixel_metadata_and_shade(0x92, 3, 0, 0x00);

    assert_eq!(
        source_obj, 0,
        "Transparent OBJ should leave BG/backdrop path selected"
    );
    assert_eq!(
        color_id, 0,
        "DMG BG-disabled path should force backdrop color id 0"
    );
    assert_eq!(
        palette_code, 0,
        "BG-disabled path should bypass BGP and use forced-white DMG mapping"
    );
    assert!(!obj_behind_bg);
    assert!(!bg_nonzero);
    assert_eq!(
        shade_id, 0,
        "BG-disabled backdrop should remain white regardless of BGP"
    );
}

#[test]
fn mode3_bg_tile_attr_scaffold_reads_vram_bank1_tilemap_metadata_without_changing_dmg_path() {
    let mut bus = make_test_bus();
    bus.debug_force_enable_ppu_cgb_scaffold_runtime(true);
    let lcdc = 0x91; // BG on, BG map 0x9800

    // BG tilemap entry for screen (0,0) lives at VRAM map offset 0x1800.
    bus.write_vram_bank_index_internal(0, 0x1800, 0x2A);
    bus.write_vram_bank_index_internal(1, 0x1800, 0b1110_1010);

    let (palette_index, vram_bank, x_flip, y_flip, bg_priority) =
        bus.debug_mode3_bg_tile_attrs_scaffold_for_screen_x(lcdc, 0, 0);
    assert_eq!(palette_index, 0b010);
    assert_eq!(vram_bank, 1);
    assert!(x_flip);
    assert!(y_flip);
    assert!(bg_priority);

    // DMG fetch path still uses bank 0 tile index and DMG color rules.
    let (palette_index, vram_bank, x_flip, bg_priority, obj_palette, obj_vram_bank, shade_id) =
        bus.debug_compose_mode3_pixel_cgb_scaffold_and_shade(lcdc, 1, 0b1110_1010, 0, 0);
    assert_eq!(palette_index, 0b010);
    assert_eq!(vram_bank, 1);
    assert!(x_flip);
    assert!(bg_priority);
    assert_eq!(obj_palette, 0);
    assert_eq!(obj_vram_bank, 0);
    assert_eq!(
        shade_id, 3,
        "DMG final shade should still come from BGP mapping only"
    );
}

#[test]
fn mode3_pixel_metadata_carries_cgb_obj_palette_scaffold_without_affecting_dmg_obj_palette_choice()
{
    let mut bus = make_test_bus();
    bus.write_byte(0xFF48, 0b00_11_10_01); // OBP0
    let lcdc = 0x93;

    let (
        bg_palette_index,
        bg_vram_bank,
        _x_flip,
        _bg_priority,
        obj_palette,
        obj_vram_bank,
        shade_id,
    ) = bus.debug_compose_mode3_pixel_cgb_scaffold_and_shade(
        lcdc,
        2,
        0b1000_0101, // BG attrs scaffold only
        1,
        0b0000_1011, // OBJ attrs: cgb palette=3, vram_bank=1, DMG OBP0
    );

    assert_eq!(bg_palette_index, 0b101);
    assert_eq!(bg_vram_bank, 0);
    assert_eq!(obj_palette, 0b011);
    assert_eq!(obj_vram_bank, 1);
    assert_eq!(
        shade_id, 2,
        "DMG final shade must remain driven by OBP0/OBP1 selection, not CGB scaffold palette bits"
    );
}
