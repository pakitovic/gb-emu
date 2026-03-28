use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum BgFetchPhase {
    #[default]
    TileIndex = 0,
    TileDataLow = 1,
    TileDataHigh = 2,
    Push = 3,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum BgPushSubstate {
    #[default]
    ReadyNormal,
    ReadyAfterRecovery,
    Stalled,
    RecoverySleep,
}

#[derive(Clone, Copy)]
pub(super) struct ObjCandidate {
    pub(super) x_raw: u8,
    pub(super) y_raw: u8,
    pub(super) oam_index: u8,
}

impl ObjCandidate {
    pub(super) const EMPTY: Self = Self {
        x_raw: 0,
        y_raw: 0,
        oam_index: 0,
    };
}

#[derive(Clone, Copy, Default)]
pub(super) struct BgFifoPixel {
    pub(super) color_id: u8,
    pub(super) cgb_bg_attrs: CgbBgTileAttrsScaffold,
}

#[derive(Clone, Copy, Default)]
pub(super) struct ObjFifoPixel {
    pub(super) color_id: u8,
    pub(super) attr: u8,
    pub(super) cgb_obj_attrs: CgbObjAttrsScaffold,
}

impl ObjFifoPixel {
    pub(super) const TRANSPARENT: Self = Self {
        color_id: 0,
        attr: 0,
        cgb_obj_attrs: CgbObjAttrsScaffold {
            palette_index: 0,
            vram_bank: 0,
        },
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CgbBgTileAttrsScaffold {
    pub(super) palette_index: u8,
    pub(super) vram_bank: u8,
    pub(super) x_flip: bool,
    pub(super) y_flip: bool,
    pub(super) bg_priority: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CgbObjAttrsScaffold {
    pub(super) palette_index: u8,
    pub(super) vram_bank: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Mode3CgbPixelMetaScaffold {
    pub(super) bg_attrs: CgbBgTileAttrsScaffold,
    pub(super) obj_attrs: CgbObjAttrsScaffold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode3PixelSource {
    Bg,
    Obj,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Mode3PixelPriorityFlags {
    pub(super) obj_behind_bg: bool,
    pub(super) bg_color_nonzero: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DmgPaletteSelector {
    ForcedWhite,
    Bg,
    Obj0,
    Obj1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Mode3PixelMeta {
    pub(super) color_id: u8,
    pub(super) source: Mode3PixelSource,
    pub(super) priority_flags: Mode3PixelPriorityFlags,
    pub(super) dmg_palette: DmgPaletteSelector,
    pub(super) cgb_scaffold: Mode3CgbPixelMetaScaffold,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Mode3ObjSprite {
    pub(super) x_left: i16,
    pub(super) oam_index: u8,
    pub(super) line_in_sprite: u8,
    pub(super) sprite_height: u8,
    pub(super) low: u8,
    pub(super) high: u8,
    pub(super) attr: u8,
    pub(super) fetch_dots: u8,
    pub(super) post_fetch_dots: u8,
}

impl Mode3ObjSprite {
    pub(super) const EMPTY: Self = Self {
        x_left: 0,
        oam_index: 0,
        line_in_sprite: 0,
        sprite_height: 8,
        low: 0,
        high: 0,
        attr: 0,
        fetch_dots: OBJ_FETCH_BASE_DOTS,
        post_fetch_dots: 0,
    };
}

#[derive(Default)]
pub(super) struct Mode3FifoState {
    pub(super) active: bool,
    pub(super) warmup_dots: u8,
    pub(super) discard_pixels: u8,
    pub(super) output_x: u8,
    pub(super) fetch_screen_x: i16,
    pub(super) window_active: bool,
    pub(super) window_start_x: i16,
    pub(super) head: usize,
    pub(super) len: usize,
    pub(super) pixels: [BgFifoPixel; BG_FIFO_CAPACITY],
    pub(super) bg_fetch_phase: BgFetchPhase,
    pub(super) bg_fetch_dots_remaining: u8,
    pub(super) bg_fetch_tile_index: u8,
    pub(super) bg_fetch_tile_line_addr: usize,
    pub(super) bg_fetch_low: u8,
    pub(super) bg_fetch_high: u8,
    pub(super) bg_fetch_cgb_attrs: CgbBgTileAttrsScaffold,
    pub(super) bg_push_substate: BgPushSubstate,
    pub(super) obj_head: usize,
    pub(super) obj_len: usize,
    pub(super) obj_pixels: [ObjFifoPixel; BG_FIFO_CAPACITY],
    pub(super) obj_sprites: [Mode3ObjSprite; MAX_SPRITES_PER_LINE],
    pub(super) obj_sprite_count: usize,
    pub(super) obj_next_sprite: usize,
    pub(super) obj_active_sprite: Option<Mode3ObjSprite>,
    pub(super) obj_fetch_dots_remaining: u8,
    pub(super) obj_shutdown_dots_remaining: u8,
}

impl Mode3FifoState {
    pub(super) fn start(&mut self, discard_pixels: u8) {
        self.active = true;
        self.warmup_dots = MODE3_BG_WARMUP_DOTS;
        self.discard_pixels = discard_pixels;
        self.output_x = 0;
        self.fetch_screen_x = -(discard_pixels as i16);
        self.window_active = false;
        self.window_start_x = 0;
        self.head = 0;
        self.len = 0;
        self.bg_fetch_phase = BgFetchPhase::TileIndex;
        self.bg_fetch_dots_remaining = BG_FETCH_PHASE_DOTS;
        self.bg_fetch_tile_index = 0;
        self.bg_fetch_tile_line_addr = 0;
        self.bg_fetch_low = 0;
        self.bg_fetch_high = 0;
        self.bg_fetch_cgb_attrs = CgbBgTileAttrsScaffold::default();
        self.bg_push_substate = BgPushSubstate::ReadyNormal;
        self.obj_head = 0;
        self.obj_len = 0;
        self.obj_pixels.fill(ObjFifoPixel::TRANSPARENT);
        self.obj_sprites.fill(Mode3ObjSprite::EMPTY);
        self.obj_sprite_count = 0;
        self.obj_next_sprite = 0;
        self.obj_active_sprite = None;
        self.obj_fetch_dots_remaining = 0;
        self.obj_shutdown_dots_remaining = 0;
    }

    pub(super) fn reset(&mut self) {
        self.active = false;
        self.warmup_dots = 0;
        self.discard_pixels = 0;
        self.output_x = 0;
        self.fetch_screen_x = 0;
        self.window_active = false;
        self.window_start_x = 0;
        self.head = 0;
        self.len = 0;
        self.bg_fetch_phase = BgFetchPhase::TileIndex;
        self.bg_fetch_dots_remaining = 0;
        self.bg_fetch_tile_index = 0;
        self.bg_fetch_tile_line_addr = 0;
        self.bg_fetch_low = 0;
        self.bg_fetch_high = 0;
        self.bg_fetch_cgb_attrs = CgbBgTileAttrsScaffold::default();
        self.bg_push_substate = BgPushSubstate::ReadyNormal;
        self.obj_head = 0;
        self.obj_len = 0;
        self.obj_pixels.fill(ObjFifoPixel::TRANSPARENT);
        self.obj_sprites.fill(Mode3ObjSprite::EMPTY);
        self.obj_sprite_count = 0;
        self.obj_next_sprite = 0;
        self.obj_active_sprite = None;
        self.obj_fetch_dots_remaining = 0;
        self.obj_shutdown_dots_remaining = 0;
    }

    pub(super) fn can_push_8(&self) -> bool {
        self.len <= 8
    }

    pub(super) fn restart_for_window(&mut self, trigger_x: i16) {
        self.window_active = true;
        self.window_start_x = trigger_x;
        // SCX fine-scroll discard applies only to the initial BG fetch path of the line.
        // Once the window restarts, pixels come from window coordinates and must not
        // inherit any remaining BG discard budget (Kirby HUD/window jitter case).
        self.discard_pixels = 0;
        self.fetch_screen_x = trigger_x.max(0);
        self.head = 0;
        self.len = 0;
        self.bg_fetch_phase = BgFetchPhase::TileIndex;
        self.bg_fetch_dots_remaining = BG_FETCH_PHASE_DOTS;
        self.bg_fetch_tile_index = 0;
        self.bg_fetch_tile_line_addr = 0;
        self.bg_fetch_low = 0;
        self.bg_fetch_high = 0;
        self.bg_fetch_cgb_attrs = CgbBgTileAttrsScaffold::default();
        self.bg_push_substate = BgPushSubstate::ReadyNormal;
    }

    pub(super) fn push(&mut self, pixel: BgFifoPixel) {
        if self.len == self.pixels.len() {
            return;
        }
        let tail = (self.head + self.len) % self.pixels.len();
        self.pixels[tail] = pixel;
        self.len += 1;
    }

    pub(super) fn pop(&mut self) -> Option<BgFifoPixel> {
        if self.len == 0 {
            return None;
        }
        let pixel = self.pixels[self.head];
        self.head = (self.head + 1) % self.pixels.len();
        self.len -= 1;
        Some(pixel)
    }

    pub(super) fn obj_set_sprites(
        &mut self,
        sprites: [Mode3ObjSprite; MAX_SPRITES_PER_LINE],
        count: usize,
    ) {
        self.obj_sprites = sprites;
        self.obj_sprite_count = count;
        self.obj_next_sprite = 0;
        self.obj_head = 0;
        self.obj_len = 0;
        self.obj_pixels.fill(ObjFifoPixel::TRANSPARENT);
        self.obj_active_sprite = None;
        self.obj_fetch_dots_remaining = 0;
        self.obj_shutdown_dots_remaining = 0;
    }

    pub(super) fn obj_ensure_len(&mut self, needed: usize) {
        while self.obj_len < needed && self.obj_len < self.obj_pixels.len() {
            let tail = (self.obj_head + self.obj_len) % self.obj_pixels.len();
            self.obj_pixels[tail] = ObjFifoPixel::TRANSPARENT;
            self.obj_len += 1;
        }
    }

    pub(super) fn obj_set_if_transparent(&mut self, offset: usize, pixel: ObjFifoPixel) {
        self.obj_ensure_len(offset.saturating_add(1));
        if offset >= self.obj_len {
            return;
        }
        let index = (self.obj_head + offset) % self.obj_pixels.len();
        if self.obj_pixels[index].color_id == 0 {
            self.obj_pixels[index] = pixel;
        }
    }

    pub(super) fn obj_pop(&mut self) -> ObjFifoPixel {
        if self.obj_len == 0 {
            return ObjFifoPixel::TRANSPARENT;
        }
        let pixel = self.obj_pixels[self.obj_head];
        self.obj_pixels[self.obj_head] = ObjFifoPixel::TRANSPARENT;
        self.obj_head = (self.obj_head + 1) % self.obj_pixels.len();
        self.obj_len -= 1;
        pixel
    }

    pub(super) fn obj_clear_pending(&mut self) {
        self.obj_head = 0;
        self.obj_len = 0;
        self.obj_pixels.fill(ObjFifoPixel::TRANSPARENT);
        self.obj_active_sprite = None;
        self.obj_fetch_dots_remaining = 0;
        self.obj_shutdown_dots_remaining = 0;
    }
}

pub(in crate::memory) struct PpuState {
    pub(in crate::memory) ly_counter: u16,
    pub(in crate::memory) startup_line: bool,
    pub(in crate::memory) post_enable_phase: u8,
    pub(in crate::memory) enable_delay: u8,
    pub(in crate::memory) mode: PpuMode,
    pub(in crate::memory) mode_edge_events: PpuModeEdgeEvents,
    pub(in crate::memory) stat_irq_line: bool,
    pub(in crate::memory) stat_mode0_enabled_this_line: bool,
    pub(in crate::memory) frame_counter: u64,
    pub(super) window_line_counter: u8,
    pub(super) window_triggered_this_line: bool,
    pub(super) window_trigger_pending: bool,
    pub(super) mode3_dots_latched: u16,
    pub(super) cgb_scaffold_runtime_enabled: bool,
    pub(super) mode3_fifo: Mode3FifoState,
    pub(super) bg_color_ids_line: [u8; crate::memory::LCD_WIDTH],
}

impl Default for PpuState {
    fn default() -> Self {
        Self {
            ly_counter: 0,
            startup_line: false,
            post_enable_phase: 0,
            enable_delay: 0,
            mode: PpuMode::HBlank,
            mode_edge_events: PpuModeEdgeEvents::default(),
            stat_irq_line: false,
            stat_mode0_enabled_this_line: false,
            frame_counter: 0,
            window_line_counter: 0,
            window_triggered_this_line: false,
            window_trigger_pending: false,
            mode3_dots_latched: 0,
            cgb_scaffold_runtime_enabled: false,
            mode3_fifo: Mode3FifoState::default(),
            bg_color_ids_line: [0; crate::memory::LCD_WIDTH],
        }
    }
}
