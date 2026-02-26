use super::*;

#[test]
fn framebuffer_renders_bg_tile_colors_with_identity_palette() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off to allow deterministic VRAM setup
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF47, 0xE4); // BGP identity: 0->0, 1->1, 2->2, 3->3

    // Tile map first entry points to tile 0.
    bus.write_byte(0x9800, 0x00);
    // Tile 0, row 0 encodes color ids: 0,1,2,3,0,1,2,3.
    bus.write_byte(0x8000, 0x55); // low plane
    bus.write_byte(0x8001, 0x33); // high plane

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on + unsigned tile data at 0x8000
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let expected = [0xFF, 0xAA, 0x55, 0x00, 0xFF, 0xAA, 0x55, 0x00];
    assert_eq!(&frame[..8], &expected);
}

#[test]
fn framebuffer_applies_scx_scroll_to_bg_sampling() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off to allow deterministic VRAM setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x08); // SCX: shift view by one tile
    bus.write_byte(0xFF47, 0xE4); // BGP identity

    // First tile is white, second tile is black.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x9801, 0x01);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0x00);
}

#[test]
fn framebuffer_scx_write_mid_frame_affects_following_lines_only() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX (initial)
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // Tile 0 white, tile 1 black.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x9801, 0x01);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);
    bus.write_byte(0x8014, 0xFF);
    bus.write_byte(0x8015, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on

    let mut reached_ly2_mode2 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 2 {
            reached_ly2_mode2 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode2);

    // Change SCX during frame after lines 0 and 1 are already rendered.
    bus.write_byte(0xFF43, 0x08);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // Line 1 keeps old SCX=0 (white at x=0).
    assert_eq!(frame[160], 0xFF);
    // Line 2 uses new SCX=8 (black at x=0).
    assert_eq!(frame[320], 0x00);
}

#[test]
fn framebuffer_scx_write_during_mode3_affects_remaining_pixels_same_line() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // Fill first BG map row with alternating white/black tiles.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, if (i & 1) == 0 { 0x00 } else { 0x01 });
    }

    // Tile 0 white.
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8004, 0x00);
    bus.write_byte(0x8005, 0x00);

    // Tile 1 black.
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);
    bus.write_byte(0x8014, 0xFF);
    bus.write_byte(0x8015, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on

    // Reach LY=2 mode 3.
    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // Let some pixels of LY=2 render with SCX=0, then shift by one tile.
    bus.tick(20);
    bus.write_byte(0xFF43, 0x08);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    // Early pixel rendered before SCX write: tile 0 (white).
    assert_eq!(frame[line2], 0xFF);
    // Later pixel rendered after SCX write: shifted one tile (white instead of black).
    assert_eq!(frame[line2 + 40], 0xFF);
}

#[test]
fn framebuffer_scx_low_bits_write_during_mode3_does_not_truncate_line_output() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x07); // SCX low-bits penalty latched at mode3 start
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // Fill first BG map row with black tile so rendered pixels are visibly non-white.
    for i in 0..32u16 {
        bus.write_byte(0x9800 + i, 0x01);
    }
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);
    bus.write_byte(0x8014, 0xFF);
    bus.write_byte(0x8015, 0xFF);

    bus.write_byte(0xFF40, 0x91); // LCD on + BG on

    // Reach LY=2 mode 3 and change SCX low bits to reduce live penalty.
    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    bus.tick(20);
    bus.write_byte(0xFF43, 0x00);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    assert_eq!(frame[line2 + 159], 0x00);
}

#[test]
fn framebuffer_obp_write_during_mode3_affects_later_obj_pixels_only() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);
    bus.write_byte(0x8004, 0x00);
    bus.write_byte(0x8005, 0x00);

    // Sprite tile 2 with color id=2 across the row used by LY=2.
    bus.write_byte(0x8024, 0x00);
    bus.write_byte(0x8025, 0xFF);

    // Two sprites on LY=2: one at x=0..7 and one at x=16..23.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 2); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    bus.write_byte(0xFE04, 16); // Y
    bus.write_byte(0xFE05, 24); // X
    bus.write_byte(0xFE06, 2); // tile
    bus.write_byte(0xFE07, 0x00); // attrs

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ

    let mut reached_ly2_mode3 = false;
    for _ in 0..(154 * 456 * 2) {
        let ly = bus.read_byte(0xFF44);
        let mode = bus.read_byte(0xFF41) & 0x03;
        if ly == 2 && mode == 3 {
            reached_ly2_mode3 = true;
            break;
        }
        bus.tick(1);
    }
    assert!(reached_ly2_mode3);

    // Render first sprite with original OBP0, then switch palette before second sprite.
    bus.tick(34);
    bus.write_byte(0xFF48, 0x00); // color 2 -> shade 0 (white)
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    let line2 = 2 * 160;
    assert_eq!(frame[line2], 0x55); // first sprite kept old OBP0 mapping
    assert_eq!(frame[line2 + 16], 0xFF); // second sprite used updated OBP0 mapping
}

#[test]
fn framebuffer_bg_disabled_forces_white_backdrop_ignoring_bgp() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xFF); // map all BG color IDs to shade 3 (black)

    // LCD on with BG/window disabled (LCDC.0=0).
    bus.write_byte(0xFF40, 0x90);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0xFF);
}

#[test]
fn framebuffer_window_overrides_bg_where_visible() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // BG tile map (9C00) uses tile 0 (white).
    bus.write_byte(0x9C00, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window tile map (9800) uses tile 1 (black).
    bus.write_byte(0x9800, 0x01);
    bus.write_byte(0x8010, 0xFF);
    bus.write_byte(0x8011, 0xFF);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0x07); // WX so window starts at x=0

    // LCD on + window enable + BG map 9C00 + BG on + tile data 8000.
    bus.write_byte(0xFF40, 0xB9);
    // First LCD-on frame contains startup quirks; validate steady-state frame.
    wait_for_next_frame(&mut bus);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0x00);
    assert_eq!(frame[8], 0xFF);
}

#[test]
fn framebuffer_sprite_renders_over_bg() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 2 with color id=2 across full row.
    bus.write_byte(0x8020, 0x00);
    bus.write_byte(0x8021, 0xFF);

    // Sprite at top-left visible corner.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 2); // tile
    bus.write_byte(0xFE03, 0x00); // attrs

    // LCD on + OBJ enable + BG enable + tile data 8000.
    bus.write_byte(0xFF40, 0x93);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0x55); // OBJ color id=2
}

#[test]
fn framebuffer_sprite_priority_bit_defers_to_non_zero_bg() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // BG tile 0 row starts with color id=1, then zeros.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x80);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 2 with color id=3 at first pixel.
    bus.write_byte(0x8020, 0x80);
    bus.write_byte(0x8021, 0x80);

    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 2); // tile
    bus.write_byte(0xFE03, 0x80); // priority: behind BG

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0xAA); // BG color id=1 should win
}

#[test]
fn framebuffer_sprite_obeys_palette_and_flip_attributes() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0
    bus.write_byte(0xFF49, 0x1B); // inverted mapping for OBP1

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 3:
    // row0 first pixel color id=1, last pixel color id=2.
    bus.write_byte(0x8030, 0x80);
    bus.write_byte(0x8031, 0x01);

    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 3); // tile
    // bit4=palette1, bit5=xflip.
    bus.write_byte(0xFE03, 0x30);

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // xflip makes leftmost pixel use original rightmost color id=2.
    // OBP1=0x1B maps color id=2 to shade 1 => luma 0xAA.
    assert_eq!(frame[0], 0xAA);
}

#[test]
fn framebuffer_limits_visible_sprites_to_ten_per_line() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Sprite tile 4 fully black (color id=3).
    bus.write_byte(0x8040, 0xFF);
    bus.write_byte(0x8041, 0xFF);

    // Place 11 sprites on the same scanline; first 10 should be considered.
    for i in 0..11u16 {
        let base = 0xFE00 + i * 4;
        bus.write_byte(base, 16); // Y
        bus.write_byte(base + 1, 8 + (i as u8) * 8); // X
        bus.write_byte(base + 2, 4); // tile
        bus.write_byte(base + 3, 0); // attrs
    }

    bus.write_byte(0xFF40, 0x93); // LCD on + BG + OBJ
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // Pixel x=80 belongs only to the 11th sprite, which should be dropped.
    assert_eq!(frame[80], 0xFF);
}

#[test]
fn framebuffer_sprite_8x16_uses_sequential_tiles() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // 8x16 sprite uses tiles 6 (top) and 7 (bottom).
    // Tile 6 row0 -> color id 0 (white), tile 7 row0 -> color id 3 (black).
    bus.write_byte(0x8060, 0x00);
    bus.write_byte(0x8061, 0x00);
    bus.write_byte(0x8070, 0xFF);
    bus.write_byte(0x8071, 0xFF);

    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 8); // X
    bus.write_byte(0xFE02, 6); // tile (LSB ignored in 8x16 mode)
    bus.write_byte(0xFE03, 0); // attrs

    // LCD on + BG + OBJ + OBJ size 8x16.
    bus.write_byte(0xFF40, 0x97);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[0], 0xFF); // top half from tile 6
    assert_eq!(frame[8 * 160], 0x00); // bottom half from tile 7
}

#[test]
fn framebuffer_sprite_priority_prefers_leftmost_x_then_oam_order() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Tile 8 => color id 3 across full row (black).
    bus.write_byte(0x8080, 0xFF);
    bus.write_byte(0x8081, 0xFF);
    // Tile 9 => color id 1 across full row (light gray).
    bus.write_byte(0x8090, 0xFF);
    bus.write_byte(0x8091, 0x00);

    // OAM index 0: right sprite (higher X), black.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 12); // X -> left=4
    bus.write_byte(0xFE02, 8); // tile
    bus.write_byte(0xFE03, 0);

    // OAM index 1: left sprite (lower X), light gray.
    bus.write_byte(0xFE04, 16); // Y
    bus.write_byte(0xFE05, 10); // X -> left=2
    bus.write_byte(0xFE06, 9); // tile
    bus.write_byte(0xFE07, 0);

    // LCD on + BG + OBJ.
    bus.write_byte(0xFF40, 0x93);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // Pixel x=4 is covered by both; lower X sprite should win.
    assert_eq!(frame[4], 0xAA);
}

#[test]
fn framebuffer_sprite_priority_uses_oam_order_when_x_matches() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF47, 0xE4); // identity BGP
    bus.write_byte(0xFF48, 0xE4); // identity OBP0

    // White BG tile.
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Tile 10 => color id 3 across full row (black).
    bus.write_byte(0x80A0, 0xFF);
    bus.write_byte(0x80A1, 0xFF);
    // Tile 11 => color id 1 across full row (light gray).
    bus.write_byte(0x80B0, 0xFF);
    bus.write_byte(0x80B1, 0x00);

    // OAM index 0 should have higher priority for equal X.
    bus.write_byte(0xFE00, 16); // Y
    bus.write_byte(0xFE01, 12); // X -> left=4
    bus.write_byte(0xFE02, 10); // tile (black)
    bus.write_byte(0xFE03, 0);

    bus.write_byte(0xFE04, 16); // Y
    bus.write_byte(0xFE05, 12); // X -> left=4
    bus.write_byte(0xFE06, 11); // tile (light gray)
    bus.write_byte(0xFE07, 0);

    // LCD on + BG + OBJ.
    bus.write_byte(0xFF40, 0x93);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    assert_eq!(frame[4], 0x00);
}

#[test]
fn framebuffer_left_edge_sprite_does_not_shift_with_bg_scx_fine_scroll() {
    fn render_top_line_with_scx(scx: u8) -> [u8; 160] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, scx); // SCX varies
        bus.write_byte(0xFF47, 0xE4); // identity BGP
        bus.write_byte(0xFF48, 0xE4); // identity OBP0

        // White BG tile so any visible difference comes from OBJ placement.
        bus.write_byte(0x9800, 0x00);
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);

        // Sprite tile 12 with non-zero per-column pattern across the row.
        // Pattern (left->right): 3,1,3,1,3,1,3,1
        bus.write_byte(0x80C0, 0xFF);
        bus.write_byte(0x80C1, 0xAA);

        // Place sprite partially off-screen on the left: x_left = -1.
        bus.write_byte(0xFE00, 16); // Y => row 0
        bus.write_byte(0xFE01, 7); // X => left = -1
        bus.write_byte(0xFE02, 12); // tile
        bus.write_byte(0xFE03, 0x00); // attrs

        // LCD on + BG + OBJ.
        bus.write_byte(0xFF40, 0x93);
        // Skip LCD-on startup frame quirks; compare steady-state.
        wait_for_next_frame(&mut bus);
        wait_for_next_frame(&mut bus);

        let mut line = [0u8; 160];
        line.copy_from_slice(&bus.framebuffer()[..160]);
        line
    }

    let line_scx0 = render_top_line_with_scx(0);
    let line_scx3 = render_top_line_with_scx(3);

    for x in 0..12usize {
        assert_eq!(
            line_scx0[x], line_scx3[x],
            "left-edge OBJ pixels must not shift with BG fine scroll discard (x={x})"
        );
    }
}

#[test]
fn framebuffer_window_wx_zero_applies_minus_seven_offset() {
    let mut bus = make_test_bus();

    bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
    bus.write_byte(0xFF42, 0x00); // SCY
    bus.write_byte(0xFF43, 0x00); // SCX
    bus.write_byte(0xFF47, 0xE4); // identity palette

    // BG tile map uses tile 0 (white).
    bus.write_byte(0x9800, 0x00);
    bus.write_byte(0x8000, 0x00);
    bus.write_byte(0x8001, 0x00);

    // Window map first tile is tile 1, second tile is tile 0.
    // Tile 1 row0 has color id 3 only at pixel 7 (rightmost pixel in tile).
    bus.write_byte(0x9800, 0x01);
    bus.write_byte(0x9801, 0x00);
    bus.write_byte(0x8010, 0x01);
    bus.write_byte(0x8011, 0x01);

    bus.write_byte(0xFF4A, 0x00); // WY
    bus.write_byte(0xFF4B, 0x00); // WX=0 => window starts at x=-7

    // LCD on + window enable + BG enable + tile data 8000.
    bus.write_byte(0xFF40, 0xB1);
    // First LCD-on frame contains startup quirks; validate steady-state frame.
    wait_for_next_frame(&mut bus);
    wait_for_next_frame(&mut bus);

    let frame = bus.framebuffer();
    // At x=0 we sample window pixel x=7 from first tile (black),
    // then x=1 samples next tile's pixel x=0 (white).
    assert_eq!(frame[0], 0x00);
    assert_eq!(frame[1], 0xFF);
}

#[test]
fn framebuffer_window_at_x0_does_not_inherit_bg_scx_discard() {
    fn render_top_line_with_scx(scx: u8) -> [u8; 160] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, scx); // SCX varies
        bus.write_byte(0xFF47, 0xE4); // identity palette

        // BG tiles vary so any visible coupling to SCX is easy to detect.
        for i in 0..32u16 {
            bus.write_byte(0x9800 + i, (i & 1) as u8);
        }
        // tile 0 => white
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);
        // tile 1 => black
        bus.write_byte(0x8010, 0xFF);
        bus.write_byte(0x8011, 0xFF);

        // Window map: constant distinct pattern (tile 2 then 3 repeating), independent of BG.
        for i in 0..32u16 {
            bus.write_byte(0x9C00 + i, if (i & 1) == 0 { 2 } else { 3 });
        }
        // tile 2 row0 => color id 1 across row
        bus.write_byte(0x8020, 0xFF);
        bus.write_byte(0x8021, 0x00);
        // tile 3 row0 => color id 2 across row
        bus.write_byte(0x8030, 0x00);
        bus.write_byte(0x8031, 0xFF);

        bus.write_byte(0xFF4A, 0x00); // WY=0
        bus.write_byte(0xFF4B, 0x07); // WX=7 => window starts at x=0

        // LCD on + window + BG, use window map 9C00 and tile data 8000.
        bus.write_byte(0xFF40, 0xF1);
        wait_for_next_frame(&mut bus);
        wait_for_next_frame(&mut bus);

        let mut line = [0u8; 160];
        line.copy_from_slice(&bus.framebuffer()[..160]);
        line
    }

    let line_scx0 = render_top_line_with_scx(0);
    let line_scx3 = render_top_line_with_scx(3);

    for x in 0..32usize {
        assert_eq!(
            line_scx0[x], line_scx3[x],
            "window pixels at x=0 (WX=7) must not inherit BG SCX discard (x={x})"
        );
    }
}

#[test]
fn framebuffer_window_restart_mid_sprite_does_not_corrupt_obj_pixels_when_bg_matches_window() {
    fn render_line_with_window_enabled(window_enabled: bool) -> [u8; 160] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, 0x00); // SCX
        bus.write_byte(0xFF47, 0xE4); // identity BGP
        bus.write_byte(0xFF48, 0xE4); // identity OBP0

        // BG and window both use the same white tiles so any visible difference
        // around WX restart must come from OBJ corruption, not background content.
        for i in 0..64u16 {
            bus.write_byte(0x9800 + i, 0x00);
            bus.write_byte(0x9C00 + i, 0x00);
        }
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);

        // Sprite tile with distinct per-column pattern (non-zero colors across row0).
        // low = 1010_1010, high = 1100_1100
        bus.write_byte(0x8020, 0xAA);
        bus.write_byte(0x8021, 0xCC);

        // Sprite visible at y=0, x=20..27, so WX=31 (x=24) crosses the sprite.
        bus.write_byte(0xFE00, 16); // Y => top at row 0
        bus.write_byte(0xFE01, 28); // X => left at x=20
        bus.write_byte(0xFE02, 2); // tile
        bus.write_byte(0xFE03, 0x00); // attrs

        bus.write_byte(0xFF4A, 0x00); // WY
        bus.write_byte(0xFF4B, 31); // WX => window starts at x=24

        let mut lcdc = 0x93; // LCD on + BG + OBJ
        if window_enabled {
            lcdc |= 0x20; // window enable
        }
        // Use BG map 9C00 + tile data 8000 for consistency with earlier window tests.
        lcdc |= 0x08 | 0x10;
        bus.write_byte(0xFF40, lcdc);

        // Render a stable frame (skip LCD-on startup quirks frame).
        wait_for_next_frame(&mut bus);
        wait_for_next_frame(&mut bus);

        let mut line = [0u8; 160];
        let frame = bus.framebuffer();
        line.copy_from_slice(&frame[..160]);
        line
    }

    let line_no_window = render_line_with_window_enabled(false);
    let line_with_window = render_line_with_window_enabled(true);

    // Compare a small span around the sprite/window overlap.
    for x in 16..32usize {
        assert_eq!(
            line_with_window[x], line_no_window[x],
            "window restart at WX boundary should not corrupt overlapping OBJ pixels when BG/window content matches (x={x})"
        );
    }
}

#[test]
fn framebuffer_window_restart_mid_multisprite_obj_does_not_split_columns_when_bg_matches_window() {
    fn render_line_with_window_enabled(window_enabled: bool) -> [u8; 160] {
        let mut bus = make_test_bus();

        bus.write_byte(0xFF40, 0x00); // LCD off for deterministic setup
        bus.write_byte(0xFF42, 0x00); // SCY
        bus.write_byte(0xFF43, 0x00); // SCX
        bus.write_byte(0xFF47, 0xE4); // identity BGP
        bus.write_byte(0xFF48, 0xE4); // identity OBP0

        for i in 0..64u16 {
            bus.write_byte(0x9800 + i, 0x00);
            bus.write_byte(0x9C00 + i, 0x00);
        }
        bus.write_byte(0x8000, 0x00);
        bus.write_byte(0x8001, 0x00);

        // Two sprite tiles with different column patterns to catch column splits.
        // Tile 2 row0 pattern: 1,2,1,2,1,2,1,2
        bus.write_byte(0x8020, 0xAA); // low
        bus.write_byte(0x8021, 0x00); // high
        // Tile 3 row0 pattern: 3,1,3,1,3,1,3,1
        bus.write_byte(0x8030, 0xFF); // low
        bus.write_byte(0x8031, 0xAA); // high

        // 16px-wide object composed from 2 sprites spanning x=20..35.
        // WX=31 => window starts at x=24, crossing inside sprite 0 and before sprite 1.
        bus.write_byte(0xFE00, 16); // Y
        bus.write_byte(0xFE01, 28); // X => left=20
        bus.write_byte(0xFE02, 2); // tile 2
        bus.write_byte(0xFE03, 0x00);

        bus.write_byte(0xFE04, 16); // Y
        bus.write_byte(0xFE05, 36); // X => left=28
        bus.write_byte(0xFE06, 3); // tile 3
        bus.write_byte(0xFE07, 0x00);

        bus.write_byte(0xFF4A, 0x00); // WY
        bus.write_byte(0xFF4B, 31); // WX => x=24

        let mut lcdc = 0x93 | 0x08 | 0x10; // LCD+BG+OBJ + 9C00 map + 8000 tile data
        if window_enabled {
            lcdc |= 0x20;
        }
        bus.write_byte(0xFF40, lcdc);

        wait_for_next_frame(&mut bus);
        wait_for_next_frame(&mut bus);

        let mut line = [0u8; 160];
        let frame = bus.framebuffer();
        line.copy_from_slice(&frame[..160]);
        line
    }

    let line_no_window = render_line_with_window_enabled(false);
    let line_with_window = render_line_with_window_enabled(true);

    for x in 18..38usize {
        assert_eq!(
            line_with_window[x], line_no_window[x],
            "window restart should not split multi-sprite OBJ columns when BG/window content matches (x={x})"
        );
    }
}
