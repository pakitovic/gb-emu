use super::Bus;

const STAT_MODE_HBLANK: u8 = 0;
const STAT_MODE_VBLANK: u8 = 1;
const STAT_MODE_OAM: u8 = 2;
const STAT_MODE_TRANSFER: u8 = 3;
const STARTUP_MODE0_DOTS: u16 = 80;
const STARTUP_LINE_DOTS: u16 = 452;
const DMG_SHADE_TO_LUMA: [u8; 4] = [0xFF, 0xAA, 0x55, 0x00];
const MAX_SPRITES_PER_LINE: usize = 10;
const MODE3_BG_WARMUP_DOTS: u8 = 12;
const BG_FIFO_CAPACITY: usize = 16;

#[derive(Clone, Copy)]
struct ObjCandidate {
    x_raw: u8,
    y_raw: u8,
    tile: u8,
    attr: u8,
    oam_index: u8,
}

impl ObjCandidate {
    const EMPTY: Self = Self {
        x_raw: 0,
        y_raw: 0,
        tile: 0,
        attr: 0,
        oam_index: 0,
    };
}

#[derive(Default)]
struct Mode3FifoState {
    active: bool,
    warmup_dots: u8,
    discard_pixels: u8,
    output_x: u8,
    fetch_screen_x: i16,
    head: usize,
    len: usize,
    pixels: [u8; BG_FIFO_CAPACITY],
}

impl Mode3FifoState {
    fn start(&mut self, discard_pixels: u8) {
        self.active = true;
        self.warmup_dots = MODE3_BG_WARMUP_DOTS;
        self.discard_pixels = discard_pixels;
        self.output_x = 0;
        self.fetch_screen_x = -(discard_pixels as i16);
        self.head = 0;
        self.len = 0;
    }

    fn reset(&mut self) {
        self.active = false;
        self.warmup_dots = 0;
        self.discard_pixels = 0;
        self.output_x = 0;
        self.fetch_screen_x = 0;
        self.head = 0;
        self.len = 0;
    }

    fn can_push_8(&self) -> bool {
        self.len <= 8
    }

    fn push(&mut self, color_id: u8) {
        if self.len == self.pixels.len() {
            return;
        }
        let tail = (self.head + self.len) % self.pixels.len();
        self.pixels[tail] = color_id;
        self.len += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let color_id = self.pixels[self.head];
        self.head = (self.head + 1) % self.pixels.len();
        self.len -= 1;
        Some(color_id)
    }
}

pub(super) struct PpuState {
    pub(super) ly_counter: u16,
    pub(super) startup_line: bool,
    pub(super) post_enable_phase: u8,
    pub(super) enable_delay: u8,
    pub(super) stat_irq_line: bool,
    pub(super) stat_mode0_enabled_this_line: bool,
    pub(super) frame_counter: u64,
    mode3_dots_latched: u16,
    mode3_fifo: Mode3FifoState,
    bg_color_ids_line: [u8; super::LCD_WIDTH],
}

impl Default for PpuState {
    fn default() -> Self {
        Self {
            ly_counter: 0,
            startup_line: false,
            post_enable_phase: 0,
            enable_delay: 0,
            stat_irq_line: false,
            stat_mode0_enabled_this_line: false,
            frame_counter: 0,
            mode3_dots_latched: 0,
            mode3_fifo: Mode3FifoState::default(),
            bg_color_ids_line: [0; super::LCD_WIDTH],
        }
    }
}

impl PpuState {
    pub(super) fn write_lcdc(bus: &mut Bus, value: u8) {
        let was_enabled = Self::lcd_enabled(bus);
        let now_enabled = (value & 0x80) != 0;
        bus.io[0x40] = value;

        match (was_enabled, now_enabled) {
            (true, false) => {
                bus.io[0x44] = 0;
                bus.ppu.ly_counter = 0;
                bus.ppu.startup_line = false;
                bus.ppu.post_enable_phase = 0;
                bus.ppu.enable_delay = 0;
                bus.ppu.stat_mode0_enabled_this_line = false;
                bus.ppu.mode3_dots_latched = 0;
                bus.ppu.mode3_fifo.reset();
                bus.ppu.bg_color_ids_line.fill(0);
                Self::set_stat_mode(bus, STAT_MODE_HBLANK);
                // LY=LYC flag is retained while LCD is disabled.
                Self::update_stat_irq_line(bus);
            }
            (false, true) => {
                bus.io[0x44] = 0;
                bus.ppu.ly_counter = 0;
                bus.ppu.startup_line = true;
                bus.ppu.post_enable_phase = 0;
                bus.ppu.enable_delay = 0;
                bus.ppu.stat_mode0_enabled_this_line = false;
                bus.ppu.mode3_dots_latched = 0;
                bus.ppu.mode3_fifo.reset();
                bus.ppu.bg_color_ids_line.fill(0);
                Self::set_stat_mode(bus, STAT_MODE_HBLANK);
                Self::update_lyc_flag(bus);
                Self::update_stat_irq_line(bus);
            }
            _ => {}
        }
    }

    pub(super) fn write_stat(bus: &mut Bus, value: u8) {
        // Bits 3..6 are writable. Bits 0..2 are PPU-generated.
        let old_mode0_source = (bus.io[0x41] & 0x08) != 0;
        bus.io[0x41] = (bus.io[0x41] & 0x07) | (value & 0x78);
        let new_mode0_source = (bus.io[0x41] & 0x08) != 0;
        if !old_mode0_source && new_mode0_source && Self::lcd_enabled(bus) && bus.io[0x44] < 144 {
            bus.ppu.stat_mode0_enabled_this_line = true;
        } else if !new_mode0_source {
            bus.ppu.stat_mode0_enabled_this_line = false;
        }
        Self::update_stat_irq_line(bus);
    }

    pub(super) fn write_lyc(bus: &mut Bus, value: u8) {
        bus.io[0x45] = value;
        if Self::lcd_enabled(bus) {
            Self::update_lyc_flag(bus);
            Self::update_stat_irq_line(bus);
        }
    }

    pub(super) fn write_ly(bus: &mut Bus, value: u8) {
        let _ = value;
        bus.io[0x44] = 0;
        bus.ppu.ly_counter = 0;
        bus.ppu.startup_line = false;
        bus.ppu.post_enable_phase = 0;
        bus.ppu.enable_delay = 0;
        bus.ppu.stat_mode0_enabled_this_line = false;
        bus.ppu.mode3_dots_latched = 0;
        bus.ppu.mode3_fifo.reset();
        bus.ppu.bg_color_ids_line.fill(0);
        Self::set_stat_mode(bus, STAT_MODE_HBLANK);
        if Self::lcd_enabled(bus) {
            Self::update_lyc_flag(bus);
        }
        Self::update_stat_irq_line(bus);
    }

    pub(super) fn step(bus: &mut Bus) {
        if !Self::lcd_enabled(bus) {
            return;
        }

        if bus.ppu.enable_delay > 0 {
            bus.ppu.enable_delay -= 1;
            Self::update_lyc_flag(bus);
            Self::update_stat_irq_line(bus);
            return;
        }

        let ly = bus.io[0x44];
        if ly < 144 && bus.ppu.ly_counter == 0 {
            let startup_line = bus.ppu.startup_line && ly == 0;
            bus.ppu.mode3_dots_latched = Self::mode3_length_tcycles(bus, ly, startup_line);
        }

        if ly < 144 {
            let startup_line = bus.ppu.startup_line && ly == 0;
            Self::render_mode3_dot(bus, ly, bus.ppu.ly_counter, startup_line);
        }

        let line_length = Self::line_length_tcycles(bus, ly);
        bus.ppu.ly_counter = bus.ppu.ly_counter.wrapping_add(1);
        if bus.ppu.ly_counter >= line_length {
            bus.ppu.ly_counter = 0;
            if ly < 144 {
                let lcdc = bus.io[0x40];
                let bg_color_ids_line = bus.ppu.bg_color_ids_line;
                Self::render_objects_scanline(bus, lcdc, ly as usize, &bg_color_ids_line);
                bus.ppu.mode3_fifo.reset();
            }
            let next_ly = if ly >= 153 { 0 } else { ly.wrapping_add(1) };
            bus.io[0x44] = next_ly;
            bus.ppu.stat_mode0_enabled_this_line = false;

            if bus.ppu.startup_line && ly == 0 {
                bus.ppu.startup_line = false;
                bus.ppu.post_enable_phase = 2;
            } else if bus.ppu.post_enable_phase > 0 {
                bus.ppu.post_enable_phase -= 1;
            }
            if next_ly == 144 {
                let iflags = bus.interrupt_flags() | (1 << 0);
                bus.set_interrupt_flags(iflags);
                bus.ppu.frame_counter = bus.ppu.frame_counter.wrapping_add(1);
            }
        }

        let ly = bus.io[0x44];
        let mode = if ly >= 144 {
            STAT_MODE_VBLANK
        } else {
            Self::mode_for_visible_line(
                bus,
                ly,
                bus.ppu.ly_counter,
                bus.ppu.startup_line && ly == 0,
            )
        };
        Self::set_stat_mode(bus, mode);
        Self::update_lyc_flag(bus);
        Self::update_stat_irq_line(bus);
    }

    fn lcd_enabled(bus: &Bus) -> bool {
        (bus.io[0x40] & 0x80) != 0
    }

    fn ppu_mode(bus: &Bus) -> u8 {
        bus.io[0x41] & 0x03
    }

    fn ppu_startup_mode0_slice_active(bus: &Bus) -> bool {
        bus.ppu.post_enable_phase > 0
            && bus.io[0x44] > 0
            && bus.io[0x44] < 144
            && Self::ppu_mode(bus) == STAT_MODE_HBLANK
            && bus.ppu.ly_counter < 4
    }

    fn ppu_startup_mode2_tail_active(bus: &Bus) -> bool {
        bus.ppu.post_enable_phase > 0
            && bus.io[0x44] > 0
            && bus.io[0x44] < 144
            && Self::ppu_mode(bus) == STAT_MODE_OAM
            && (80..84).contains(&bus.ppu.ly_counter)
    }

    pub(super) fn ppu_blocks_oam_read(bus: &Bus) -> bool {
        bus.dma.active
            || Self::ppu_startup_mode0_slice_active(bus)
            || (Self::lcd_enabled(bus)
                && matches!(Self::ppu_mode(bus), STAT_MODE_OAM | STAT_MODE_TRANSFER))
    }

    pub(super) fn ppu_blocks_oam_write(bus: &Bus) -> bool {
        bus.dma.active || !Self::ppu_allows_oam_access(bus)
    }

    pub(super) fn ppu_blocks_vram_read(bus: &Bus) -> bool {
        Self::ppu_startup_mode2_tail_active(bus) || !Self::ppu_allows_vram_access(bus)
    }

    pub(super) fn ppu_blocks_vram_write(bus: &Bus) -> bool {
        !Self::ppu_allows_vram_access(bus)
    }

    pub(super) fn stat_read_value(bus: &Bus) -> u8 {
        let mut value = bus.io[0x41];
        if Self::ppu_startup_mode0_slice_active(bus) {
            value &= !0x04;
        }
        value
    }

    fn ppu_allows_oam_access(bus: &Bus) -> bool {
        if !Self::lcd_enabled(bus) {
            return true;
        }
        // DMG LCD-on startup quirk: OAM remains briefly accessible in mode 2
        // around dot 80 on the first lines after enabling LCD.
        if Self::ppu_startup_mode2_tail_active(bus) {
            return true;
        }
        !matches!(Self::ppu_mode(bus), STAT_MODE_OAM | STAT_MODE_TRANSFER)
    }

    fn ppu_allows_vram_access(bus: &Bus) -> bool {
        if !Self::lcd_enabled(bus) {
            return true;
        }
        (bus.io[0x41] & 0x03) != STAT_MODE_TRANSFER
    }

    fn line_length_tcycles(bus: &Bus, ly: u8) -> u16 {
        if bus.ppu.startup_line && ly == 0 {
            STARTUP_LINE_DOTS
        } else {
            456
        }
    }

    fn mode_for_visible_line(bus: &Bus, ly: u8, line_cycle: u16, startup_line: bool) -> u8 {
        let mode3_dots = if line_cycle == 0 {
            Self::mode3_length_tcycles(bus, ly, startup_line)
        } else {
            bus.ppu.mode3_dots_latched
        };
        if startup_line {
            if line_cycle < STARTUP_MODE0_DOTS {
                STAT_MODE_HBLANK
            } else if line_cycle < STARTUP_MODE0_DOTS.saturating_add(mode3_dots) {
                STAT_MODE_TRANSFER
            } else {
                STAT_MODE_HBLANK
            }
        } else {
            let mode2_end = match bus.ppu.post_enable_phase {
                2 => 84u16,
                1 => 84u16,
                _ => 80u16,
            };

            if bus.ppu.post_enable_phase == 0 {
                if line_cycle < 80 {
                    STAT_MODE_OAM
                } else if line_cycle < 80u16.saturating_add(mode3_dots) {
                    STAT_MODE_TRANSFER
                } else {
                    STAT_MODE_HBLANK
                }
            } else if line_cycle < 4 {
                STAT_MODE_HBLANK
            } else if line_cycle < mode2_end {
                STAT_MODE_OAM
            } else if line_cycle < mode2_end.saturating_add(mode3_dots) {
                STAT_MODE_TRANSFER
            } else {
                STAT_MODE_HBLANK
            }
        }
    }

    fn mode3_length_tcycles(bus: &Bus, ly: u8, startup_line: bool) -> u16 {
        let extra = Self::mode3_extra_tcycles(bus, ly);
        let base = 172u16.saturating_add(extra);
        let line_len = Self::line_length_tcycles(bus, ly);
        if startup_line {
            // Startup line starts in mode 0 and skips mode 2.
            base.min(line_len.saturating_sub(STARTUP_MODE0_DOTS))
        } else {
            // Visible lines always spend 80 dots in mode 2.
            base.min(line_len.saturating_sub(80))
        }
    }

    fn mode3_extra_tcycles(bus: &Bus, ly: u8) -> u16 {
        let scx_penalty = (bus.io[0x43] & 0x07) as u16;
        scx_penalty.saturating_add(Self::mode3_obj_penalty_tcycles(bus, ly))
    }

    fn mode3_obj_penalty_tcycles(bus: &Bus, ly: u8) -> u16 {
        // Objects disabled.
        if (bus.io[0x40] & 0x02) == 0 {
            return 0;
        }

        let sprite_height = if (bus.io[0x40] & 0x04) != 0 {
            16i16
        } else {
            8i16
        };

        // DMG pipeline considers up to 10 sprites per scanline.
        let mut sprites: [(u8, u8); 10] = [(0, 0); 10]; // (x, oam_index)
        let mut sprite_count = 0usize;
        for oam_index in 0u8..40u8 {
            let base = (oam_index as usize) * 4;
            let y = bus.oam[base] as i16 - 16;
            let x = bus.oam[base + 1];

            // Drawn sprites are X=0 special case or X in 1..=167.
            if x != 0 && x >= 168 {
                continue;
            }

            let ly_i = ly as i16;
            if ly_i < y || ly_i >= y + sprite_height {
                continue;
            }

            if sprite_count < sprites.len() {
                sprites[sprite_count] = (x, oam_index);
                sprite_count += 1;
            }
        }

        // Penalty order is left-to-right; ties broken by OAM index.
        sprites[..sprite_count].sort_unstable();

        // DMG sprite timing heuristic compatible with mooneye acceptance:
        // - sprites are evaluated in sessions
        // - first sprite in a session has startup adjustment based on X mod 8
        // - every additional sprite in the same session costs 6 dots
        // - every ended (non-final) session incurs a shutdown adjustment
        //
        // A new session starts when sprite X has a gap of 8+ pixels compared
        // to the previous sprite.
        const SHUTDOWN_PENALTY: [u16; 8] = [3, 2, 3, 2, 3, 2, 2, 2];

        let mut penalty = 0u16;
        let mut i = 0usize;
        while i < sprite_count {
            let mut j = i;
            while j + 1 < sprite_count {
                let x = sprites[j].0;
                let next_x = sprites[j + 1].0;
                if next_x.wrapping_sub(x) < 8 {
                    j += 1;
                } else {
                    break;
                }
            }

            let first_x_mod = (sprites[i].0 & 0x07) as i16;
            let startup_adjust = match first_x_mod {
                0 | 1 => 2,
                4..=7 => -2,
                _ => 0,
            };
            let first_penalty = (6i16 + startup_adjust) as u16;
            penalty = penalty.saturating_add(first_penalty);

            let additional_sprites = j - i;
            penalty = penalty.saturating_add((additional_sprites as u16).saturating_mul(6));

            if j + 1 < sprite_count {
                let last_x_mod = (sprites[j].0 & 0x07) as usize;
                penalty = penalty.saturating_add(SHUTDOWN_PENALTY[last_x_mod]);
            }

            i = j + 1;
        }

        penalty
    }

    fn mode3_start_tcycle(bus: &Bus, startup_line: bool) -> u16 {
        if startup_line {
            STARTUP_MODE0_DOTS
        } else {
            match bus.ppu.post_enable_phase {
                2 | 1 => 84u16,
                _ => 80u16,
            }
        }
    }

    fn render_mode3_dot(bus: &mut Bus, ly: u8, line_cycle: u16, startup_line: bool) {
        let mode3_start = Self::mode3_start_tcycle(bus, startup_line);
        let mode3_dots = bus.ppu.mode3_dots_latched;
        let mode3_end = mode3_start.saturating_add(mode3_dots);
        if line_cycle < mode3_start || line_cycle >= mode3_end {
            return;
        }

        if line_cycle == mode3_start {
            let row_start = (ly as usize) * super::LCD_WIDTH;
            bus.framebuffer[row_start..row_start + super::LCD_WIDTH].fill(DMG_SHADE_TO_LUMA[0]);
            bus.ppu.bg_color_ids_line.fill(0);
            let discard_pixels = bus.io[0x43] & 0x07;
            bus.ppu.mode3_fifo.start(discard_pixels);
        }

        if !bus.ppu.mode3_fifo.active {
            return;
        }

        let lcdc = bus.io[0x40];
        let y = ly as usize;
        let should_fetch = bus.ppu.mode3_fifo.can_push_8();
        if should_fetch {
            let fetch_screen_x = bus.ppu.mode3_fifo.fetch_screen_x;
            let mut fetched_pixels = [0u8; 8];
            for (lane, pixel) in fetched_pixels.iter_mut().enumerate() {
                *pixel = Self::bg_window_color_id_for_screen_x(
                    bus,
                    lcdc,
                    y,
                    fetch_screen_x + lane as i16,
                );
            }
            for color_id in fetched_pixels {
                bus.ppu.mode3_fifo.push(color_id);
            }
            bus.ppu.mode3_fifo.fetch_screen_x += 8;
        }

        let color_id = if bus.ppu.mode3_fifo.warmup_dots > 0 {
            bus.ppu.mode3_fifo.warmup_dots -= 1;
            None
        } else {
            bus.ppu.mode3_fifo.pop()
        };

        let Some(color_id) = color_id else {
            return;
        };

        if bus.ppu.mode3_fifo.discard_pixels > 0 {
            bus.ppu.mode3_fifo.discard_pixels -= 1;
            return;
        }

        if (bus.ppu.mode3_fifo.output_x as usize) < super::LCD_WIDTH {
            let x = bus.ppu.mode3_fifo.output_x as usize;
            bus.ppu.mode3_fifo.output_x = bus.ppu.mode3_fifo.output_x.saturating_add(1);
            let shade_id = if (lcdc & 0x01) == 0 {
                0
            } else {
                (bus.io[0x47] >> (color_id * 2)) & 0x03
            };
            let row_start = y * super::LCD_WIDTH;
            bus.ppu.bg_color_ids_line[x] = color_id;
            bus.framebuffer[row_start + x] = DMG_SHADE_TO_LUMA[shade_id as usize];
        }
    }

    fn bg_window_color_id_for_screen_x(bus: &Bus, lcdc: u8, y: usize, screen_x: i16) -> u8 {
        if (lcdc & 0x01) == 0 {
            return 0;
        }

        let scx = bus.io[0x43];
        let scy = bus.io[0x42];
        let wy = bus.io[0x4A];
        let wx = bus.io[0x4B];
        let wx_start = wx as i16 - 7;

        let bg_map_base = if (lcdc & 0x08) != 0 {
            0x1C00usize
        } else {
            0x1800usize
        };
        let window_map_base = if (lcdc & 0x40) != 0 {
            0x1C00usize
        } else {
            0x1800usize
        };
        let window_enabled = (lcdc & 0x20) != 0 && wy < 144 && wx <= 166;
        let window_active_line = window_enabled && y >= wy as usize;
        let use_window = window_active_line && screen_x >= wx_start;

        let (tile_map_base, tile_col, tile_row, line_in_tile, bit_x) = if use_window {
            let window_x = (screen_x - wx_start) as usize;
            let window_y = y - wy as usize;
            (
                window_map_base,
                window_x / 8,
                window_y / 8,
                window_y & 0x07,
                (window_x & 0x07) as u8,
            )
        } else {
            let bg_y = (y as u8).wrapping_add(scy);
            let bg_x = (screen_x as i32 + scx as i32).rem_euclid(256) as u8;
            (
                bg_map_base,
                (bg_x / 8) as usize,
                (bg_y / 8) as usize,
                (bg_y & 0x07) as usize,
                bg_x & 0x07,
            )
        };

        let tile_map_index = tile_row * 32 + tile_col;
        let tile_index = bus.vram[tile_map_base + tile_map_index];
        let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, line_in_tile);
        let low = bus.vram[tile_line_addr];
        let high = bus.vram[tile_line_addr + 1];
        let bit = 7u8.wrapping_sub(bit_x);
        (((high >> bit) & 1) << 1) | ((low >> bit) & 1)
    }

    fn render_objects_scanline(
        bus: &mut Bus,
        lcdc: u8,
        y: usize,
        bg_color_ids: &[u8; super::LCD_WIDTH],
    ) {
        if (lcdc & 0x02) == 0 {
            return;
        }

        let sprite_height: usize = if (lcdc & 0x04) != 0 { 16 } else { 8 };
        let obp0 = bus.io[0x48];
        let obp1 = bus.io[0x49];
        let row_start = y * super::LCD_WIDTH;
        let y_i = y as i16;

        let mut sprites = [ObjCandidate::EMPTY; MAX_SPRITES_PER_LINE];
        let mut sprite_count = 0usize;

        for oam_index in 0u8..40 {
            let base = (oam_index as usize) * 4;
            let y_raw = bus.oam[base];
            let x_raw = bus.oam[base + 1];
            let tile = bus.oam[base + 2];
            let attr = bus.oam[base + 3];

            // Hidden objects on DMG.
            if x_raw == 0 || x_raw >= 168 {
                continue;
            }

            let top = y_raw as i16 - 16;
            if y_i < top || y_i >= top + sprite_height as i16 {
                continue;
            }

            sprites[sprite_count] = ObjCandidate {
                x_raw,
                y_raw,
                tile,
                attr,
                oam_index,
            };
            sprite_count += 1;
            if sprite_count == MAX_SPRITES_PER_LINE {
                break;
            }
        }

        sprites[..sprite_count].sort_unstable_by_key(|sprite| (sprite.x_raw, sprite.oam_index));

        // Draw lowest-priority first, highest-priority last.
        for sprite in sprites[..sprite_count].iter().rev() {
            let x_left = sprite.x_raw as i16 - 8;
            let y_top = sprite.y_raw as i16 - 16;

            let mut y_in_sprite = (y_i - y_top) as usize;
            if (sprite.attr & 0x40) != 0 {
                y_in_sprite = sprite_height - 1 - y_in_sprite;
            }

            let tile_line = if sprite_height == 16 {
                let base_tile = sprite.tile & 0xFE;
                base_tile.wrapping_add((y_in_sprite / 8) as u8)
            } else {
                sprite.tile
            };
            let line_in_tile = y_in_sprite & 0x07;
            let line_addr = (tile_line as usize) * 16 + line_in_tile * 2;
            let low = bus.vram[line_addr];
            let high = bus.vram[line_addr + 1];

            for x_in_sprite in 0..8usize {
                let screen_x = x_left + x_in_sprite as i16;
                if !(0..super::LCD_WIDTH as i16).contains(&screen_x) {
                    continue;
                }

                let bit = if (sprite.attr & 0x20) != 0 {
                    x_in_sprite as u8
                } else {
                    7u8.wrapping_sub(x_in_sprite as u8)
                };
                let color_id = (((high >> bit) & 1) << 1) | ((low >> bit) & 1);
                if color_id == 0 {
                    continue;
                }

                let x_index = screen_x as usize;
                let obj_behind_bg = (sprite.attr & 0x80) != 0;
                if obj_behind_bg && bg_color_ids[x_index] != 0 {
                    continue;
                }

                let palette = if (sprite.attr & 0x10) != 0 {
                    obp1
                } else {
                    obp0
                };
                let shade_id = (palette >> (color_id * 2)) & 0x03;
                bus.framebuffer[row_start + x_index] = DMG_SHADE_TO_LUMA[shade_id as usize];
            }
        }
    }

    fn bg_tile_line_addr(lcdc: u8, tile_index: u8, line_in_tile: usize) -> usize {
        if (lcdc & 0x10) != 0 {
            (tile_index as usize) * 16 + line_in_tile * 2
        } else {
            let signed_index = tile_index as i8 as i16;
            let tile_base = 0x1000i16 + signed_index * 16;
            (tile_base as usize) + line_in_tile * 2
        }
    }

    fn set_stat_mode(bus: &mut Bus, mode: u8) {
        bus.io[0x41] = (bus.io[0x41] & !0x03) | (mode & 0x03);
    }

    fn update_lyc_flag(bus: &mut Bus) {
        let lyc_match = bus.io[0x44] == bus.io[0x45];
        if lyc_match {
            bus.io[0x41] |= 0x04;
        } else {
            bus.io[0x41] &= !0x04;
        }
    }

    pub(super) fn stat_irq_source_active(bus: &Bus) -> bool {
        let stat = bus.io[0x41];
        let mode = stat & 0x03;
        let lyc = (stat & 0x04) != 0;
        // DMG quirk: if mode 2 interrupt is enabled, entering LY=144 also
        // raises STAT alongside VBlank.
        let mode2_or_vblank_start = mode == STAT_MODE_OAM
            || (mode == STAT_MODE_VBLANK && bus.io[0x44] == 144 && bus.ppu.ly_counter == 0);
        let mode0_active = mode == STAT_MODE_HBLANK && Self::mode0_stat_source_active_now(bus);
        ((stat & 0x40) != 0 && lyc)
            || ((stat & 0x20) != 0 && mode2_or_vblank_start)
            || ((stat & 0x10) != 0 && mode == STAT_MODE_VBLANK)
            || ((stat & 0x08) != 0 && mode0_active)
    }

    fn mode0_stat_source_active_now(bus: &Bus) -> bool {
        let ly = bus.io[0x44];
        if ly >= 144 {
            return false;
        }

        let startup_line = bus.ppu.startup_line && ly == 0;
        let mode3_end = if startup_line {
            STARTUP_MODE0_DOTS.saturating_add(bus.ppu.mode3_dots_latched)
        } else {
            80u16.saturating_add(bus.ppu.mode3_dots_latched)
        };

        let delay_tcycles = if bus.ppu.stat_mode0_enabled_this_line {
            0
        } else {
            4
        };
        bus.ppu.ly_counter >= mode3_end.saturating_add(delay_tcycles)
    }

    fn update_stat_irq_line(bus: &mut Bus) {
        let high = Self::stat_irq_source_active(bus);
        if high && !bus.ppu.stat_irq_line {
            let iflags = bus.interrupt_flags() | (1 << 1);
            bus.set_interrupt_flags(iflags);
        }
        bus.ppu.stat_irq_line = high;
    }
}

impl Bus {
    pub(super) fn ppu_blocks_oam_read(&self) -> bool {
        PpuState::ppu_blocks_oam_read(self)
    }

    pub(super) fn ppu_blocks_oam_write(&self) -> bool {
        PpuState::ppu_blocks_oam_write(self)
    }

    pub(super) fn ppu_blocks_vram_read(&self) -> bool {
        PpuState::ppu_blocks_vram_read(self)
    }

    pub(super) fn ppu_blocks_vram_write(&self) -> bool {
        PpuState::ppu_blocks_vram_write(self)
    }

    pub(super) fn stat_read_value(&self) -> u8 {
        PpuState::stat_read_value(self)
    }

    pub(super) fn write_lcdc(&mut self, value: u8) {
        PpuState::write_lcdc(self, value);
    }

    pub(super) fn write_stat(&mut self, value: u8) {
        PpuState::write_stat(self, value);
    }

    pub(super) fn write_lyc(&mut self, value: u8) {
        PpuState::write_lyc(self, value);
    }

    pub(super) fn write_ly(&mut self, value: u8) {
        PpuState::write_ly(self, value);
    }

    pub(super) fn step_ppu(&mut self) {
        PpuState::step(self);
    }

    pub(super) fn stat_irq_source_active(&self) -> bool {
        PpuState::stat_irq_source_active(self)
    }
}
