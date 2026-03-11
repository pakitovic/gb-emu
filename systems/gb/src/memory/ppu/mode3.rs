use super::bus::PpuStateAdapter;
use super::*;

impl PpuState {
    pub(super) fn obj_session_startup_adjust(x_mod: u8) -> i16 {
        match x_mod {
            0 | 1 => 2,
            4..=7 => -2,
            _ => 0,
        }
    }

    pub(super) fn obj_session_shutdown_penalty(x_mod: u8) -> u16 {
        OBJ_SESSION_SHUTDOWN_PENALTY[x_mod as usize] as u16
    }

    pub(super) fn mode3_start_tcycle(bus: &Bus, startup_line: bool) -> u16 {
        if startup_line {
            STARTUP_MODE0_DOTS
        } else {
            match bus.ppu_state().post_enable_phase {
                2 | 1 => 84u16,
                _ => 80u16,
            }
        }
    }

    pub(super) fn render_mode3_dot(bus: &mut Bus, ly: u8, line_cycle: u16, startup_line: bool) {
        let mode3_start = Self::mode3_start_tcycle(bus, startup_line);
        let mode3_dots = bus.ppu_state().mode3_dots_latched;
        let mode3_end = mode3_start.saturating_add(mode3_dots);
        if line_cycle < mode3_start || line_cycle >= mode3_end {
            return;
        }

        if line_cycle == mode3_start {
            bus.ppu_clear_framebuffer_line(ly, DMG_SHADE_TO_LUMA[0]);
            bus.ppu_state_mut().bg_color_ids_line.fill(0);
            let discard_pixels = bus.ppu_scx() & 0x07;
            bus.ppu_state_mut().mode3_fifo.start(discard_pixels);
            Self::prepare_mode3_obj_line(bus, ly as usize);
        }

        if !bus.ppu_state().mode3_fifo.active {
            return;
        }

        let screen_x = Self::mode3_current_screen_x(bus);
        Self::mode3_maybe_trigger_window(bus, ly, startup_line, screen_x);
        if Self::mode3_step_obj_fetch(bus, screen_x) {
            Self::extend_mode3_for_obj_contention(bus, ly, startup_line);
            return;
        }

        let lcdc = bus.ppu_lcdc();
        let y = ly as usize;
        Self::mode3_step_bg_fetch(bus, lcdc, y);

        let bg_pixel = if bus.ppu_state().mode3_fifo.warmup_dots > 0 {
            bus.ppu_state_mut().mode3_fifo.warmup_dots -= 1;
            None
        } else {
            bus.ppu_state_mut().mode3_fifo.pop()
        };

        let Some(bg_pixel) = bg_pixel else {
            return;
        };
        Self::mode3_latch_bg_push_recovery_sleep_after_pop(bus);

        if bus.ppu_state().mode3_fifo.discard_pixels > 0 {
            // Fine-scroll discard consumes the pixel stream before the first visible
            // dot, so OBJ FIFO must advance in lockstep with BG to keep sprite columns
            // aligned at the left edge when SCX has a sub-tile offset.
            let _ = Self::mode3_pop_obj_pixel(bus);
            bus.ppu_state_mut().mode3_fifo.discard_pixels -= 1;
            return;
        }

        if (bus.ppu_state().mode3_fifo.output_x as usize) < crate::memory::LCD_WIDTH {
            let x = bus.ppu_state().mode3_fifo.output_x as usize;
            let next_output_x = bus.ppu_state().mode3_fifo.output_x.saturating_add(1);
            bus.ppu_state_mut().mode3_fifo.output_x = next_output_x;
            let obj_pixel = Self::mode3_pop_obj_pixel(bus);
            let pixel_meta = Self::compose_mode3_pixel_meta(lcdc, bg_pixel, obj_pixel);
            let shade_id = Self::map_mode3_dmg_shade_id(bus, pixel_meta);
            let palette_selector_code = match pixel_meta.dmg_palette {
                DmgPaletteSelector::ForcedWhite => 1,
                DmgPaletteSelector::Bg => 1,
                DmgPaletteSelector::Obj0 => 2,
                DmgPaletteSelector::Obj1 => 3,
            };
            bus.ppu_state_mut().bg_color_ids_line[x] = bg_pixel.color_id;
            bus.ppu_write_framebuffer_pixel(
                y,
                x,
                DMG_SHADE_TO_LUMA[shade_id as usize],
                palette_selector_code,
            );
        }
    }

    pub(super) fn mode3_current_screen_x(bus: &Bus) -> i16 {
        bus.ppu_state().mode3_fifo.output_x as i16
            - bus.ppu_state().mode3_fifo.discard_pixels as i16
    }

    pub(super) fn mode3_latch_bg_push_recovery_sleep_after_pop(bus: &mut Bus) {
        if bus.ppu_state().mode3_fifo.bg_fetch_phase != BgFetchPhase::Push {
            return;
        }
        if bus.ppu_state().mode3_fifo.bg_push_substate != BgPushSubstate::Stalled {
            return;
        }
        if bus.ppu_state().mode3_fifo.can_push_8() {
            bus.ppu_state_mut().mode3_fifo.bg_push_substate = BgPushSubstate::RecoverySleep;
        }
    }

    pub(super) fn mode3_window_enabled_on_line(bus: &Bus, ly: u8) -> bool {
        let lcdc = bus.ppu_lcdc();
        let wy = bus.ppu_wy();
        let wx = bus.ppu_wx();
        (lcdc & 0x20) != 0 && wy < 144 && wx <= 166 && ly >= wy
    }

    pub(super) fn mode3_window_trigger_screen_x(bus: &Bus) -> i16 {
        bus.ppu_wx() as i16 - 7
    }

    pub(super) fn mode3_bg_takeover_boundary(bus: &Bus) -> bool {
        let push_stalled_boundary = bus.ppu_state().mode3_fifo.bg_fetch_phase == BgFetchPhase::Push
            && bus.ppu_state().mode3_fifo.bg_push_substate == BgPushSubstate::Stalled
            && !bus.ppu_state().mode3_fifo.can_push_8();
        let push_ready_boundary = Self::mode3_bg_push_ready_takeover_boundary(bus);
        (bus.ppu_state().mode3_fifo.bg_fetch_phase == BgFetchPhase::TileIndex
            && bus.ppu_state().mode3_fifo.bg_fetch_dots_remaining == 0)
            || push_stalled_boundary
            || push_ready_boundary
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_push_recovery_sleep_pending(bus: &Bus) -> bool {
        bus.ppu_state().mode3_fifo.bg_fetch_phase == BgFetchPhase::Push
            && bus.ppu_state().mode3_fifo.bg_push_substate == BgPushSubstate::RecoverySleep
    }

    pub(super) fn mode3_bg_push_ready_takeover_boundary(bus: &Bus) -> bool {
        bus.ppu_state().mode3_fifo.bg_fetch_phase == BgFetchPhase::Push
            && bus.ppu_state().mode3_fifo.bg_push_substate == BgPushSubstate::ReadyAfterRecovery
            && bus.ppu_state().mode3_fifo.bg_fetch_dots_remaining == 0
    }

    pub(super) fn mode3_obj_takeover_boundary(bus: &Bus) -> bool {
        Self::mode3_bg_takeover_boundary(bus)
            && bus.ppu_state().mode3_fifo.obj_fetch_dots_remaining == 0
            && bus.ppu_state().mode3_fifo.obj_shutdown_dots_remaining == 0
    }

    pub(super) fn mode3_window_takeover_boundary(bus: &Bus) -> bool {
        Self::mode3_obj_takeover_boundary(bus)
    }

    pub(super) fn mode3_window_restart_now(
        bus: &mut Bus,
        ly: u8,
        startup_line: bool,
        trigger_x: i16,
    ) {
        bus.ppu_state_mut().window_trigger_pending = false;
        bus.ppu_state_mut().window_triggered_this_line = true;
        bus.ppu_state_mut().mode3_fifo.restart_for_window(trigger_x);
        Self::extend_mode3_dots(bus, ly, startup_line, MODE3_WINDOW_RESTART_DOTS);
    }

    pub(super) fn mode3_window_trigger_is_immediate(trigger_x: i16) -> bool {
        trigger_x <= 0
    }

    pub(super) fn mode3_obj_can_takeover_now(bus: &Bus, screen_x: i16) -> bool {
        if (bus.ppu_lcdc() & 0x02) == 0 || !Self::mode3_obj_takeover_boundary(bus) {
            return false;
        }
        if bus.ppu_state().mode3_fifo.obj_next_sprite >= bus.ppu_state().mode3_fifo.obj_sprite_count
        {
            return false;
        }
        let sprite =
            bus.ppu_state().mode3_fifo.obj_sprites[bus.ppu_state().mode3_fifo.obj_next_sprite];
        let obj_fetch_lookahead = sprite.fetch_dots as i16;
        sprite.x_left <= screen_x + obj_fetch_lookahead
    }

    pub(super) fn mode3_maybe_trigger_window(
        bus: &mut Bus,
        ly: u8,
        startup_line: bool,
        screen_x: i16,
    ) {
        if bus.ppu_state().window_triggered_this_line {
            return;
        }
        if !Self::mode3_window_enabled_on_line(bus, ly) {
            bus.ppu_state_mut().window_trigger_pending = false;
            return;
        }

        let trigger_x = Self::mode3_window_trigger_screen_x(bus);
        let output_x = bus.ppu_state().mode3_fifo.output_x as i16;
        let reached_trigger = output_x == trigger_x || (trigger_x <= 0 && output_x == 0);
        if !bus.ppu_state().window_trigger_pending {
            if !reached_trigger {
                return;
            }

            // WX<=7 can start at the beginning of the visible line without
            // waiting for a later BG takeover boundary.
            if Self::mode3_window_trigger_is_immediate(trigger_x) {
                Self::mode3_window_restart_now(bus, ly, startup_line, trigger_x);
                return;
            }
            bus.ppu_state_mut().window_trigger_pending = true;
        }

        if !Self::mode3_window_takeover_boundary(bus) {
            return;
        }
        if Self::mode3_obj_can_takeover_now(bus, screen_x) {
            return;
        }

        Self::mode3_window_restart_now(bus, ly, startup_line, trigger_x);
    }

    pub(super) fn mode3_step_bg_fetch(bus: &mut Bus, lcdc: u8, y: usize) {
        match bus.ppu_state().mode3_fifo.bg_fetch_phase {
            BgFetchPhase::TileIndex | BgFetchPhase::TileDataLow | BgFetchPhase::TileDataHigh => {
                if bus.ppu_state().mode3_fifo.bg_fetch_dots_remaining > 0 {
                    bus.ppu_state_mut().mode3_fifo.bg_fetch_dots_remaining -= 1;
                }
                if bus.ppu_state().mode3_fifo.bg_fetch_dots_remaining != 0 {
                    return;
                }

                match bus.ppu_state().mode3_fifo.bg_fetch_phase {
                    BgFetchPhase::TileIndex => {
                        let fetch_screen_x = bus.ppu_state().mode3_fifo.fetch_screen_x;
                        let (tile_index, cgb_bg_attrs, tile_line_addr) =
                            Self::mode3_fetch_tile_index_and_line_addr(
                                bus,
                                lcdc,
                                y,
                                fetch_screen_x,
                            );
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_tile_index = tile_index;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_cgb_attrs = cgb_bg_attrs;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_tile_line_addr = tile_line_addr;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_phase = BgFetchPhase::TileDataLow;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_dots_remaining =
                            BG_FETCH_PHASE_DOTS;
                        return;
                    }
                    BgFetchPhase::TileDataLow => {
                        let tile_line_addr = bus.ppu_state().mode3_fifo.bg_fetch_tile_line_addr;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_low =
                            bus.read_vram_index_internal(tile_line_addr);
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_phase = BgFetchPhase::TileDataHigh;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_dots_remaining =
                            BG_FETCH_PHASE_DOTS;
                        return;
                    }
                    BgFetchPhase::TileDataHigh => {
                        let tile_line_addr = bus.ppu_state().mode3_fifo.bg_fetch_tile_line_addr;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_high =
                            bus.read_vram_index_internal(tile_line_addr + 1);
                        bus.ppu_state_mut().mode3_fifo.bg_push_substate =
                            BgPushSubstate::ReadyNormal;
                        bus.ppu_state_mut().mode3_fifo.bg_fetch_phase = BgFetchPhase::Push;
                    }
                    BgFetchPhase::Push => {}
                }
            }
            BgFetchPhase::Push => {}
        }

        if bus.ppu_state().mode3_fifo.bg_fetch_phase != BgFetchPhase::Push {
            return;
        }
        if !bus.ppu_state().mode3_fifo.can_push_8() {
            bus.ppu_state_mut().mode3_fifo.bg_push_substate = BgPushSubstate::Stalled;
            return;
        }
        if bus.ppu_state().mode3_fifo.bg_push_substate == BgPushSubstate::RecoverySleep {
            // Explicit one-dot "sleep" micro-op after a FIFO-full push stall. This
            // keeps the fetcher in Push for one more dot before the actual push.
            bus.ppu_state_mut().mode3_fifo.bg_push_substate = BgPushSubstate::ReadyAfterRecovery;
            return;
        }
        if bus.ppu_state().mode3_fifo.bg_push_substate == BgPushSubstate::Stalled {
            // Fallback if the prior dot did not latch the explicit recovery-sleep
            // substate after FIFO drain.
            bus.ppu_state_mut().mode3_fifo.bg_push_substate = BgPushSubstate::RecoverySleep;
            return;
        }

        let fetch_screen_x = bus.ppu_state().mode3_fifo.fetch_screen_x;
        let bit_x_start = Self::mode3_fetch_bit_x_start(bus, fetch_screen_x);
        if bit_x_start == 0 {
            let bg_fetch_high = bus.ppu_state().mode3_fifo.bg_fetch_high;
            let bg_fetch_low = bus.ppu_state().mode3_fifo.bg_fetch_low;
            let bg_fetch_cgb_attrs = bus.ppu_state().mode3_fifo.bg_fetch_cgb_attrs;
            for lane in 0..8u8 {
                let bit = 7u8.wrapping_sub(lane);
                let color_id = (((bg_fetch_high >> bit) & 1) << 1) | ((bg_fetch_low >> bit) & 1);
                bus.ppu_state_mut().mode3_fifo.push(BgFifoPixel {
                    color_id,
                    cgb_bg_attrs: bg_fetch_cgb_attrs,
                });
            }
        } else {
            let mut fetched_pixels = [BgFifoPixel::default(); 8];
            for (lane, pixel) in fetched_pixels.iter_mut().enumerate() {
                *pixel = Self::mode3_bg_pixel_for_screen_x_scaffold(
                    bus,
                    lcdc,
                    y,
                    fetch_screen_x + lane as i16,
                );
            }
            for pixel in fetched_pixels {
                bus.ppu_state_mut().mode3_fifo.push(pixel);
            }
        }

        bus.ppu_state_mut().mode3_fifo.fetch_screen_x += 8;
        bus.ppu_state_mut().mode3_fifo.bg_fetch_phase = BgFetchPhase::TileIndex;
        bus.ppu_state_mut().mode3_fifo.bg_fetch_dots_remaining = 0;
        bus.ppu_state_mut().mode3_fifo.bg_push_substate = BgPushSubstate::ReadyNormal;
    }

    pub(super) fn mode3_bg_color_id_for_screen_x(
        bus: &Bus,
        lcdc: u8,
        y: usize,
        screen_x: i16,
    ) -> u8 {
        if (lcdc & 0x01) == 0 {
            return 0;
        }

        if bus.ppu_state().mode3_fifo.window_active {
            return Self::window_color_id_for_screen_x(bus, lcdc, screen_x);
        }

        Self::background_color_id_for_screen_x(bus, lcdc, y, screen_x)
    }

    pub(super) fn mode3_bg_pixel_for_screen_x_scaffold(
        bus: &Bus,
        lcdc: u8,
        y: usize,
        screen_x: i16,
    ) -> BgFifoPixel {
        let color_id = Self::mode3_bg_color_id_for_screen_x(bus, lcdc, y, screen_x);
        if !bus.ppu_state().cgb_scaffold_runtime_enabled {
            return BgFifoPixel {
                color_id,
                cgb_bg_attrs: CgbBgTileAttrsScaffold::default(),
            };
        }
        let cgb_bg_attrs = Self::mode3_bg_tile_attrs_for_screen_x_scaffold(bus, lcdc, y, screen_x);
        BgFifoPixel {
            color_id,
            cgb_bg_attrs,
        }
    }

    pub(super) fn mode3_fetch_tile_index_and_line_addr(
        bus: &Bus,
        lcdc: u8,
        y: usize,
        screen_x: i16,
    ) -> (u8, CgbBgTileAttrsScaffold, usize) {
        let cgb_scaffold_enabled = bus.ppu_state().cgb_scaffold_runtime_enabled;
        if bus.ppu_state().mode3_fifo.window_active {
            let window_map_base = if (lcdc & 0x40) != 0 {
                0x1C00usize
            } else {
                0x1800usize
            };
            let window_x = (screen_x - bus.ppu_state().mode3_fifo.window_start_x).max(0) as usize;
            let window_y = bus.ppu_state().window_line_counter as usize;
            let tile_map_index = (window_y / 8) * 32 + (window_x / 8);
            let tile_index = bus.read_vram_index_internal(window_map_base + tile_map_index);
            let cgb_bg_attrs = if cgb_scaffold_enabled {
                Self::decode_cgb_bg_tile_attrs_scaffold(
                    bus.read_vram_bank_index_internal(1, window_map_base + tile_map_index),
                )
            } else {
                CgbBgTileAttrsScaffold::default()
            };
            let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, window_y & 0x07);
            return (tile_index, cgb_bg_attrs, tile_line_addr);
        }

        let scx = bus.ppu_scx();
        let scy = bus.ppu_scy();
        let bg_map_base = if (lcdc & 0x08) != 0 {
            0x1C00usize
        } else {
            0x1800usize
        };
        let bg_y = (y as u8).wrapping_add(scy);
        let bg_x = (screen_x as i32 + scx as i32).rem_euclid(256) as u8;
        let tile_col = (bg_x / 8) as usize;
        let tile_row = (bg_y / 8) as usize;
        let tile_map_index = tile_row * 32 + tile_col;
        let tile_index = bus.read_vram_index_internal(bg_map_base + tile_map_index);
        let cgb_bg_attrs = if cgb_scaffold_enabled {
            Self::decode_cgb_bg_tile_attrs_scaffold(
                bus.read_vram_bank_index_internal(1, bg_map_base + tile_map_index),
            )
        } else {
            CgbBgTileAttrsScaffold::default()
        };
        let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, (bg_y & 0x07) as usize);
        (tile_index, cgb_bg_attrs, tile_line_addr)
    }

    pub(super) fn mode3_bg_tile_attrs_for_screen_x_scaffold(
        bus: &Bus,
        lcdc: u8,
        y: usize,
        screen_x: i16,
    ) -> CgbBgTileAttrsScaffold {
        let (_tile_index, attrs, _tile_line_addr) =
            Self::mode3_fetch_tile_index_and_line_addr(bus, lcdc, y, screen_x);
        attrs
    }

    pub(super) fn mode3_fetch_bit_x_start(bus: &Bus, screen_x: i16) -> u8 {
        if bus.ppu_state().mode3_fifo.window_active {
            let window_x = (screen_x - bus.ppu_state().mode3_fifo.window_start_x).max(0) as usize;
            return (window_x & 0x07) as u8;
        }

        let scx = bus.ppu_scx();
        let bg_x = (screen_x as i32 + scx as i32).rem_euclid(256) as usize;
        (bg_x & 0x07) as u8
    }

    pub(super) fn window_color_id_for_screen_x(bus: &Bus, lcdc: u8, screen_x: i16) -> u8 {
        let window_map_base = if (lcdc & 0x40) != 0 {
            0x1C00usize
        } else {
            0x1800usize
        };

        let window_x = screen_x - bus.ppu_state().mode3_fifo.window_start_x;
        if window_x < 0 {
            return 0;
        }
        let window_x = window_x as usize;
        let window_y = bus.ppu_state().window_line_counter as usize;

        let tile_map_index = (window_y / 8) * 32 + (window_x / 8);
        let tile_index = bus.read_vram_index_internal(window_map_base + tile_map_index);
        let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, window_y & 0x07);
        let low = bus.read_vram_index_internal(tile_line_addr);
        let high = bus.read_vram_index_internal(tile_line_addr + 1);
        let bit = 7u8.wrapping_sub((window_x & 0x07) as u8);
        (((high >> bit) & 1) << 1) | ((low >> bit) & 1)
    }

    pub(super) fn background_color_id_for_screen_x(
        bus: &Bus,
        lcdc: u8,
        y: usize,
        screen_x: i16,
    ) -> u8 {
        let scx = bus.ppu_scx();
        let scy = bus.ppu_scy();

        let bg_map_base = if (lcdc & 0x08) != 0 {
            0x1C00usize
        } else {
            0x1800usize
        };
        let bg_y = (y as u8).wrapping_add(scy);
        let bg_x = (screen_x as i32 + scx as i32).rem_euclid(256) as u8;
        let tile_col = (bg_x / 8) as usize;
        let tile_row = (bg_y / 8) as usize;
        let line_in_tile = (bg_y & 0x07) as usize;
        let bit_x = bg_x & 0x07;

        let tile_map_index = tile_row * 32 + tile_col;
        let tile_index = bus.read_vram_index_internal(bg_map_base + tile_map_index);
        let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, line_in_tile);
        let low = bus.read_vram_index_internal(tile_line_addr);
        let high = bus.read_vram_index_internal(tile_line_addr + 1);
        let bit = 7u8.wrapping_sub(bit_x);
        (((high >> bit) & 1) << 1) | ((low >> bit) & 1)
    }

    pub(super) fn prepare_mode3_obj_line(bus: &mut Bus, y: usize) {
        let lcdc = bus.ppu_lcdc();
        let sprite_height: usize = if (lcdc & 0x04) != 0 { 16 } else { 8 };
        let y_i = y as i16;

        let mut candidates = [ObjCandidate::EMPTY; MAX_SPRITES_PER_LINE];
        let mut candidate_count = 0usize;
        for oam_index in 0u8..40 {
            let base = (oam_index as usize) * 4;
            let y_raw = bus.read_oam_index_internal(base);
            let x_raw = bus.read_oam_index_internal(base + 1);
            let tile = bus.read_oam_index_internal(base + 2);
            let attr = bus.read_oam_index_internal(base + 3);

            if x_raw >= 168 {
                continue;
            }

            let top = y_raw as i16 - 16;
            if y_i < top || y_i >= top + sprite_height as i16 {
                continue;
            }

            candidates[candidate_count] = ObjCandidate {
                x_raw,
                y_raw,
                tile,
                attr,
                oam_index,
            };
            candidate_count += 1;
            if candidate_count == MAX_SPRITES_PER_LINE {
                break;
            }
        }

        candidates[..candidate_count]
            .sort_unstable_by_key(|sprite| (sprite.x_raw, sprite.oam_index));

        let mut fetch_dots = [OBJ_FETCH_BASE_DOTS; MAX_SPRITES_PER_LINE];
        let mut post_fetch_dots = [0u8; MAX_SPRITES_PER_LINE];
        let mut i = 0usize;
        while i < candidate_count {
            let mut j = i;
            while j + 1 < candidate_count {
                let x = candidates[j].x_raw;
                let next_x = candidates[j + 1].x_raw;
                if next_x.wrapping_sub(x) < 8 {
                    j += 1;
                } else {
                    break;
                }
            }

            let startup_adjust = Self::obj_session_startup_adjust(candidates[i].x_raw & 0x07);
            fetch_dots[i] = (OBJ_FETCH_BASE_DOTS as i16 + startup_adjust).max(1) as u8;
            for dots in &mut fetch_dots[(i + 1)..=j] {
                *dots = OBJ_FETCH_BASE_DOTS;
            }

            if j + 1 < candidate_count {
                post_fetch_dots[j] =
                    Self::obj_session_shutdown_penalty(candidates[j].x_raw & 0x07) as u8;
            }

            i = j + 1;
        }

        let mut sprites = [Mode3ObjSprite::EMPTY; MAX_SPRITES_PER_LINE];
        for (i, candidate) in candidates[..candidate_count].iter().enumerate() {
            let y_top = candidate.y_raw as i16 - 16;
            let mut y_in_sprite = (y_i - y_top) as usize;
            if (candidate.attr & 0x40) != 0 {
                y_in_sprite = sprite_height - 1 - y_in_sprite;
            }

            let tile_line = if sprite_height == 16 {
                let base_tile = candidate.tile & 0xFE;
                base_tile.wrapping_add((y_in_sprite / 8) as u8)
            } else {
                candidate.tile
            };
            let line_in_tile = y_in_sprite & 0x07;
            let line_addr = (tile_line as usize) * 16 + line_in_tile * 2;

            sprites[i] = Mode3ObjSprite {
                x_left: candidate.x_raw as i16 - 8,
                low: bus.read_vram_index_internal(line_addr),
                high: bus.read_vram_index_internal(line_addr + 1),
                attr: candidate.attr,
                fetch_dots: fetch_dots[i],
                post_fetch_dots: post_fetch_dots[i],
            };
        }

        bus.ppu_state_mut()
            .mode3_fifo
            .obj_set_sprites(sprites, candidate_count);
    }

    pub(super) fn mode3_step_obj_fetch(bus: &mut Bus, screen_x: i16) -> bool {
        if (bus.ppu_lcdc() & 0x02) == 0 {
            bus.ppu_state_mut().mode3_fifo.obj_clear_pending();
            return false;
        }

        if bus.ppu_state().mode3_fifo.obj_fetch_dots_remaining > 0 {
            bus.ppu_state_mut().mode3_fifo.obj_fetch_dots_remaining -= 1;
            if bus.ppu_state().mode3_fifo.obj_fetch_dots_remaining == 0
                && let Some(sprite) = bus.ppu_state_mut().mode3_fifo.obj_active_sprite.take()
            {
                Self::mode3_merge_sprite_into_obj_fifo(bus, sprite, screen_x);
                bus.ppu_state_mut().mode3_fifo.obj_shutdown_dots_remaining = sprite.post_fetch_dots;
            }
            return true;
        }

        if bus.ppu_state().mode3_fifo.obj_shutdown_dots_remaining > 0 {
            bus.ppu_state_mut().mode3_fifo.obj_shutdown_dots_remaining -= 1;
            return true;
        }

        if !Self::mode3_obj_takeover_boundary(bus) {
            return false;
        }

        if bus.ppu_state().mode3_fifo.obj_next_sprite < bus.ppu_state().mode3_fifo.obj_sprite_count
        {
            let sprite_index = bus.ppu_state().mode3_fifo.obj_next_sprite;
            let sprite = bus.ppu_state().mode3_fifo.obj_sprites[sprite_index];
            let obj_fetch_lookahead = sprite.fetch_dots as i16;
            if sprite.x_left <= screen_x + obj_fetch_lookahead {
                bus.ppu_state_mut().mode3_fifo.obj_next_sprite += 1;
                bus.ppu_state_mut().mode3_fifo.obj_active_sprite = Some(sprite);
                bus.ppu_state_mut().mode3_fifo.obj_fetch_dots_remaining = sprite.fetch_dots.max(1);
                bus.ppu_state_mut().mode3_fifo.obj_fetch_dots_remaining -= 1;
                if bus.ppu_state_mut().mode3_fifo.obj_fetch_dots_remaining == 0 {
                    bus.ppu_state_mut().mode3_fifo.obj_active_sprite = None;
                    Self::mode3_merge_sprite_into_obj_fifo(bus, sprite, screen_x);
                    bus.ppu_state_mut().mode3_fifo.obj_shutdown_dots_remaining =
                        sprite.post_fetch_dots;
                }
                return true;
            }
        }

        false
    }

    pub(super) fn mode3_pop_obj_pixel(bus: &mut Bus) -> ObjFifoPixel {
        if (bus.ppu_lcdc() & 0x02) == 0 {
            bus.ppu_state_mut().mode3_fifo.obj_clear_pending();
            return ObjFifoPixel::TRANSPARENT;
        }
        bus.ppu_state_mut().mode3_fifo.obj_ensure_len(1);
        bus.ppu_state_mut().mode3_fifo.obj_pop()
    }

    pub(super) fn mode3_merge_sprite_into_obj_fifo(
        bus: &mut Bus,
        sprite: Mode3ObjSprite,
        screen_x: i16,
    ) {
        let x_start = sprite.x_left - screen_x;
        let mut lane_start = 0usize;
        let mut rel_offset = x_start;
        if rel_offset < 0 {
            lane_start = (-rel_offset) as usize;
            rel_offset = 0;
        }

        for x_in_sprite in lane_start..8usize {
            let bit = if (sprite.attr & 0x20) != 0 {
                x_in_sprite as u8
            } else {
                7u8.wrapping_sub(x_in_sprite as u8)
            };
            let color_id = (((sprite.high >> bit) & 1) << 1) | ((sprite.low >> bit) & 1);
            if color_id == 0 {
                continue;
            }

            let rel = rel_offset as usize + (x_in_sprite - lane_start);
            let cgb_obj_attrs = if bus.ppu_state_mut().cgb_scaffold_runtime_enabled {
                Self::decode_cgb_obj_attrs_scaffold(sprite.attr)
            } else {
                CgbObjAttrsScaffold::default()
            };
            let pixel = ObjFifoPixel {
                color_id,
                attr: sprite.attr,
                cgb_obj_attrs,
            };
            bus.ppu_state_mut()
                .mode3_fifo
                .obj_set_if_transparent(rel, pixel);
        }
    }

    pub(super) fn decode_cgb_bg_tile_attrs_scaffold(attr: u8) -> CgbBgTileAttrsScaffold {
        CgbBgTileAttrsScaffold {
            palette_index: attr & 0x07,
            vram_bank: (attr >> 3) & 0x01,
            x_flip: (attr & 0x20) != 0,
            y_flip: (attr & 0x40) != 0,
            bg_priority: (attr & 0x80) != 0,
        }
    }

    pub(super) fn decode_cgb_obj_attrs_scaffold(attr: u8) -> CgbObjAttrsScaffold {
        CgbObjAttrsScaffold {
            palette_index: attr & 0x07,
            vram_bank: (attr >> 3) & 0x01,
        }
    }

    pub(super) fn model_supports_cgb_scaffold(_model: HardwareModel) -> bool {
        // Current project scope exposes only DMG-family models.
        false
    }

    pub(super) fn bg_tile_line_addr(lcdc: u8, tile_index: u8, line_in_tile: usize) -> usize {
        if (lcdc & 0x10) != 0 {
            (tile_index as usize) * 16 + line_in_tile * 2
        } else {
            let signed_index = tile_index as i8 as i16;
            let tile_base = 0x1000i16 + signed_index * 16;
            (tile_base as usize) + line_in_tile * 2
        }
    }
}
