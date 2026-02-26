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
const BG_FETCH_TILE_DOTS: u8 = 6;
const BG_FETCH_PHASE_DOTS: u8 = 2;
const MODE3_WINDOW_RESTART_DOTS: u16 = BG_FETCH_TILE_DOTS as u16;
const OBJ_FETCH_BASE_DOTS: u8 = 6;
const OBJ_SESSION_SHUTDOWN_PENALTY: [u8; 8] = [3, 2, 3, 2, 3, 2, 2, 2];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub(in crate::memory) enum PpuMode {
    #[default]
    HBlank = STAT_MODE_HBLANK,
    VBlank = STAT_MODE_VBLANK,
    Oam = STAT_MODE_OAM,
    Transfer = STAT_MODE_TRANSFER,
}

impl PpuMode {
    fn from_stat_mode_bits(bits: u8) -> Self {
        match bits & 0x03 {
            STAT_MODE_HBLANK => Self::HBlank,
            STAT_MODE_VBLANK => Self::VBlank,
            STAT_MODE_OAM => Self::Oam,
            STAT_MODE_TRANSFER => Self::Transfer,
            _ => unreachable!(),
        }
    }

    fn stat_mode_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::memory) struct PpuModeEdgeEvents {
    pub(in crate::memory) entered_hblank: bool,
    pub(in crate::memory) entered_vblank: bool,
    pub(in crate::memory) entered_oam: bool,
    pub(in crate::memory) entered_transfer: bool,
}

impl PpuModeEdgeEvents {
    fn for_entered_mode(mode: PpuMode) -> Self {
        let mut events = Self::default();
        match mode {
            PpuMode::HBlank => events.entered_hblank = true,
            PpuMode::VBlank => events.entered_vblank = true,
            PpuMode::Oam => events.entered_oam = true,
            PpuMode::Transfer => events.entered_transfer = true,
        }
        events
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
enum BgFetchPhase {
    #[default]
    TileIndex = 0,
    TileDataLow = 1,
    TileDataHigh = 2,
    Push = 3,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum BgPushSubstate {
    #[default]
    ReadyNormal,
    ReadyAfterRecovery,
    Stalled,
    RecoverySleep,
}

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

#[derive(Clone, Copy, Default)]
struct BgFifoPixel {
    color_id: u8,
}

#[derive(Clone, Copy, Default)]
struct ObjFifoPixel {
    color_id: u8,
    attr: u8,
}

impl ObjFifoPixel {
    const TRANSPARENT: Self = Self {
        color_id: 0,
        attr: 0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode3PixelSource {
    Bg,
    Obj,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Mode3PixelPriorityFlags {
    obj_behind_bg: bool,
    bg_color_nonzero: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DmgPaletteSelector {
    ForcedWhite,
    Bg,
    Obj0,
    Obj1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mode3PixelMeta {
    color_id: u8,
    source: Mode3PixelSource,
    priority_flags: Mode3PixelPriorityFlags,
    dmg_palette: DmgPaletteSelector,
}

#[derive(Clone, Copy, Default)]
struct Mode3ObjSprite {
    x_left: i16,
    low: u8,
    high: u8,
    attr: u8,
    fetch_dots: u8,
    post_fetch_dots: u8,
}

impl Mode3ObjSprite {
    const EMPTY: Self = Self {
        x_left: 0,
        low: 0,
        high: 0,
        attr: 0,
        fetch_dots: OBJ_FETCH_BASE_DOTS,
        post_fetch_dots: 0,
    };
}

#[derive(Default)]
struct Mode3FifoState {
    active: bool,
    warmup_dots: u8,
    discard_pixels: u8,
    output_x: u8,
    fetch_screen_x: i16,
    window_active: bool,
    window_start_x: i16,
    head: usize,
    len: usize,
    pixels: [BgFifoPixel; BG_FIFO_CAPACITY],
    bg_fetch_phase: BgFetchPhase,
    bg_fetch_dots_remaining: u8,
    bg_fetch_tile_index: u8,
    bg_fetch_tile_line_addr: usize,
    bg_fetch_low: u8,
    bg_fetch_high: u8,
    bg_push_substate: BgPushSubstate,
    obj_head: usize,
    obj_len: usize,
    obj_pixels: [ObjFifoPixel; BG_FIFO_CAPACITY],
    obj_sprites: [Mode3ObjSprite; MAX_SPRITES_PER_LINE],
    obj_sprite_count: usize,
    obj_next_sprite: usize,
    obj_active_sprite: Option<Mode3ObjSprite>,
    obj_fetch_dots_remaining: u8,
    obj_shutdown_dots_remaining: u8,
}

impl Mode3FifoState {
    fn start(&mut self, discard_pixels: u8) {
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

    fn reset(&mut self) {
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

    fn can_push_8(&self) -> bool {
        self.len <= 8
    }

    fn restart_for_window(&mut self, trigger_x: i16) {
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
        self.bg_push_substate = BgPushSubstate::ReadyNormal;
    }

    fn push(&mut self, pixel: BgFifoPixel) {
        if self.len == self.pixels.len() {
            return;
        }
        let tail = (self.head + self.len) % self.pixels.len();
        self.pixels[tail] = pixel;
        self.len += 1;
    }

    fn pop(&mut self) -> Option<BgFifoPixel> {
        if self.len == 0 {
            return None;
        }
        let pixel = self.pixels[self.head];
        self.head = (self.head + 1) % self.pixels.len();
        self.len -= 1;
        Some(pixel)
    }

    fn obj_set_sprites(&mut self, sprites: [Mode3ObjSprite; MAX_SPRITES_PER_LINE], count: usize) {
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

    fn obj_ensure_len(&mut self, needed: usize) {
        while self.obj_len < needed && self.obj_len < self.obj_pixels.len() {
            let tail = (self.obj_head + self.obj_len) % self.obj_pixels.len();
            self.obj_pixels[tail] = ObjFifoPixel::TRANSPARENT;
            self.obj_len += 1;
        }
    }

    fn obj_set_if_transparent(&mut self, offset: usize, pixel: ObjFifoPixel) {
        self.obj_ensure_len(offset.saturating_add(1));
        if offset >= self.obj_len {
            return;
        }
        let index = (self.obj_head + offset) % self.obj_pixels.len();
        if self.obj_pixels[index].color_id == 0 {
            self.obj_pixels[index] = pixel;
        }
    }

    fn obj_pop(&mut self) -> ObjFifoPixel {
        if self.obj_len == 0 {
            return ObjFifoPixel::TRANSPARENT;
        }
        let pixel = self.obj_pixels[self.obj_head];
        self.obj_pixels[self.obj_head] = ObjFifoPixel::TRANSPARENT;
        self.obj_head = (self.obj_head + 1) % self.obj_pixels.len();
        self.obj_len -= 1;
        pixel
    }

    fn obj_clear_pending(&mut self) {
        self.obj_head = 0;
        self.obj_len = 0;
        self.obj_pixels.fill(ObjFifoPixel::TRANSPARENT);
        self.obj_active_sprite = None;
        self.obj_fetch_dots_remaining = 0;
        self.obj_shutdown_dots_remaining = 0;
    }
}

pub(super) struct PpuState {
    pub(super) ly_counter: u16,
    pub(super) startup_line: bool,
    pub(super) post_enable_phase: u8,
    pub(super) enable_delay: u8,
    pub(super) mode: PpuMode,
    pub(super) mode_edge_events: PpuModeEdgeEvents,
    pub(super) stat_irq_line: bool,
    pub(super) stat_mode0_enabled_this_line: bool,
    pub(super) frame_counter: u64,
    window_line_counter: u8,
    window_triggered_this_line: bool,
    window_trigger_pending: bool,
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
            mode: PpuMode::HBlank,
            mode_edge_events: PpuModeEdgeEvents::default(),
            stat_irq_line: false,
            stat_mode0_enabled_this_line: false,
            frame_counter: 0,
            window_line_counter: 0,
            window_triggered_this_line: false,
            window_trigger_pending: false,
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
                bus.ppu.window_line_counter = 0;
                bus.ppu.window_triggered_this_line = false;
                bus.ppu.window_trigger_pending = false;
                bus.ppu.mode3_dots_latched = 0;
                bus.ppu.mode3_fifo.reset();
                bus.ppu.bg_color_ids_line.fill(0);
                Self::force_ppu_mode(bus, PpuMode::HBlank);
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
                bus.ppu.window_line_counter = 0;
                bus.ppu.window_triggered_this_line = false;
                bus.ppu.window_trigger_pending = false;
                bus.ppu.mode3_dots_latched = 0;
                bus.ppu.mode3_fifo.reset();
                bus.ppu.bg_color_ids_line.fill(0);
                Self::force_ppu_mode(bus, PpuMode::HBlank);
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
        bus.ppu.window_line_counter = 0;
        bus.ppu.window_triggered_this_line = false;
        bus.ppu.window_trigger_pending = false;
        bus.ppu.mode3_dots_latched = 0;
        bus.ppu.mode3_fifo.reset();
        bus.ppu.bg_color_ids_line.fill(0);
        Self::force_ppu_mode(bus, PpuMode::HBlank);
        if Self::lcd_enabled(bus) {
            Self::update_lyc_flag(bus);
        }
        Self::update_stat_irq_line(bus);
    }

    pub(super) fn step(bus: &mut Bus) {
        bus.ppu.mode_edge_events = PpuModeEdgeEvents::default();

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
            bus.ppu.window_triggered_this_line = false;
            bus.ppu.window_trigger_pending = false;
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
                if bus.ppu.window_triggered_this_line {
                    bus.ppu.window_line_counter = bus.ppu.window_line_counter.wrapping_add(1);
                }
                bus.ppu.mode3_fifo.reset();
            }
            let next_ly = if ly >= 153 { 0 } else { ly.wrapping_add(1) };
            bus.io[0x44] = next_ly;
            bus.ppu.stat_mode0_enabled_this_line = false;
            bus.ppu.window_triggered_this_line = false;
            bus.ppu.window_trigger_pending = false;

            if bus.ppu.startup_line && ly == 0 {
                bus.ppu.startup_line = false;
                bus.ppu.post_enable_phase = 2;
            } else if bus.ppu.post_enable_phase > 0 {
                bus.ppu.post_enable_phase -= 1;
            }
            if next_ly == 0 {
                bus.ppu.window_line_counter = 0;
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
        let mode_edges = Self::set_ppu_mode(bus, PpuMode::from_stat_mode_bits(mode));
        if mode_edges.entered_vblank {
            let iflags = bus.interrupt_flags() | (1 << 0);
            bus.set_interrupt_flags(iflags);
            bus.ppu.frame_counter = bus.ppu.frame_counter.wrapping_add(1);
        }
        Self::update_lyc_flag(bus);
        Self::update_stat_irq_line(bus);
    }

    fn lcd_enabled(bus: &Bus) -> bool {
        (bus.io[0x40] & 0x80) != 0
    }

    fn ppu_mode(bus: &Bus) -> u8 {
        bus.ppu.mode.stat_mode_bits()
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
        let extra = Self::mode3_extra_tcycles(bus);
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

    fn mode3_extra_tcycles(bus: &Bus) -> u16 {
        (bus.io[0x43] & 0x07) as u16
    }

    fn obj_session_startup_adjust(x_mod: u8) -> i16 {
        match x_mod {
            0 | 1 => 2,
            4..=7 => -2,
            _ => 0,
        }
    }

    fn obj_session_shutdown_penalty(x_mod: u8) -> u16 {
        OBJ_SESSION_SHUTDOWN_PENALTY[x_mod as usize] as u16
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
            Self::prepare_mode3_obj_line(bus, ly as usize);
        }

        if !bus.ppu.mode3_fifo.active {
            return;
        }

        let screen_x = Self::mode3_current_screen_x(bus);
        Self::mode3_maybe_trigger_window(bus, ly, startup_line, screen_x);
        if Self::mode3_step_obj_fetch(bus, screen_x) {
            Self::extend_mode3_for_obj_contention(bus, ly, startup_line);
            return;
        }

        let lcdc = bus.io[0x40];
        let y = ly as usize;
        Self::mode3_step_bg_fetch(bus, lcdc, y);

        let bg_pixel = if bus.ppu.mode3_fifo.warmup_dots > 0 {
            bus.ppu.mode3_fifo.warmup_dots -= 1;
            None
        } else {
            bus.ppu.mode3_fifo.pop()
        };

        let Some(bg_pixel) = bg_pixel else {
            return;
        };
        Self::mode3_latch_bg_push_recovery_sleep_after_pop(bus);

        if bus.ppu.mode3_fifo.discard_pixels > 0 {
            // Fine-scroll discard consumes the pixel stream before the first visible
            // dot, so OBJ FIFO must advance in lockstep with BG to keep sprite columns
            // aligned at the left edge when SCX has a sub-tile offset.
            let _ = Self::mode3_pop_obj_pixel(bus);
            bus.ppu.mode3_fifo.discard_pixels -= 1;
            return;
        }

        if (bus.ppu.mode3_fifo.output_x as usize) < super::LCD_WIDTH {
            let x = bus.ppu.mode3_fifo.output_x as usize;
            bus.ppu.mode3_fifo.output_x = bus.ppu.mode3_fifo.output_x.saturating_add(1);
            let obj_pixel = Self::mode3_pop_obj_pixel(bus);
            let pixel_meta = Self::compose_mode3_pixel_meta(lcdc, bg_pixel, obj_pixel);
            let shade_id = Self::map_mode3_dmg_shade_id(bus, pixel_meta);
            let row_start = y * super::LCD_WIDTH;
            bus.ppu.bg_color_ids_line[x] = bg_pixel.color_id;
            bus.framebuffer[row_start + x] = DMG_SHADE_TO_LUMA[shade_id as usize];
        }
    }

    fn mode3_current_screen_x(bus: &Bus) -> i16 {
        bus.ppu.mode3_fifo.output_x as i16 - bus.ppu.mode3_fifo.discard_pixels as i16
    }

    fn mode3_latch_bg_push_recovery_sleep_after_pop(bus: &mut Bus) {
        if bus.ppu.mode3_fifo.bg_fetch_phase != BgFetchPhase::Push {
            return;
        }
        if bus.ppu.mode3_fifo.bg_push_substate != BgPushSubstate::Stalled {
            return;
        }
        if bus.ppu.mode3_fifo.can_push_8() {
            bus.ppu.mode3_fifo.bg_push_substate = BgPushSubstate::RecoverySleep;
        }
    }

    fn mode3_window_enabled_on_line(bus: &Bus, ly: u8) -> bool {
        let lcdc = bus.io[0x40];
        let wy = bus.io[0x4A];
        let wx = bus.io[0x4B];
        (lcdc & 0x20) != 0 && wy < 144 && wx <= 166 && ly >= wy
    }

    fn mode3_window_trigger_screen_x(bus: &Bus) -> i16 {
        bus.io[0x4B] as i16 - 7
    }

    fn mode3_bg_takeover_boundary(bus: &Bus) -> bool {
        let push_stalled_boundary = bus.ppu.mode3_fifo.bg_fetch_phase == BgFetchPhase::Push
            && bus.ppu.mode3_fifo.bg_push_substate == BgPushSubstate::Stalled
            && !bus.ppu.mode3_fifo.can_push_8();
        let push_ready_boundary = Self::mode3_bg_push_ready_takeover_boundary(bus);
        (bus.ppu.mode3_fifo.bg_fetch_phase == BgFetchPhase::TileIndex
            && bus.ppu.mode3_fifo.bg_fetch_dots_remaining == 0)
            || push_stalled_boundary
            || push_ready_boundary
    }

    #[cfg(test)]
    fn mode3_bg_push_recovery_sleep_pending(bus: &Bus) -> bool {
        bus.ppu.mode3_fifo.bg_fetch_phase == BgFetchPhase::Push
            && bus.ppu.mode3_fifo.bg_push_substate == BgPushSubstate::RecoverySleep
    }

    fn mode3_bg_push_ready_takeover_boundary(bus: &Bus) -> bool {
        bus.ppu.mode3_fifo.bg_fetch_phase == BgFetchPhase::Push
            && bus.ppu.mode3_fifo.bg_push_substate == BgPushSubstate::ReadyAfterRecovery
            && bus.ppu.mode3_fifo.bg_fetch_dots_remaining == 0
    }

    fn mode3_obj_takeover_boundary(bus: &Bus) -> bool {
        Self::mode3_bg_takeover_boundary(bus)
            && bus.ppu.mode3_fifo.obj_fetch_dots_remaining == 0
            && bus.ppu.mode3_fifo.obj_shutdown_dots_remaining == 0
    }

    fn mode3_window_takeover_boundary(bus: &Bus) -> bool {
        Self::mode3_obj_takeover_boundary(bus)
    }

    fn mode3_window_restart_now(bus: &mut Bus, ly: u8, startup_line: bool, trigger_x: i16) {
        bus.ppu.window_trigger_pending = false;
        bus.ppu.window_triggered_this_line = true;
        bus.ppu.mode3_fifo.restart_for_window(trigger_x);
        Self::extend_mode3_dots(bus, ly, startup_line, MODE3_WINDOW_RESTART_DOTS);
    }

    fn mode3_window_trigger_is_immediate(trigger_x: i16) -> bool {
        trigger_x <= 0
    }

    fn mode3_obj_can_takeover_now(bus: &Bus, screen_x: i16) -> bool {
        if (bus.io[0x40] & 0x02) == 0 || !Self::mode3_obj_takeover_boundary(bus) {
            return false;
        }
        if bus.ppu.mode3_fifo.obj_next_sprite >= bus.ppu.mode3_fifo.obj_sprite_count {
            return false;
        }
        let sprite = bus.ppu.mode3_fifo.obj_sprites[bus.ppu.mode3_fifo.obj_next_sprite];
        let obj_fetch_lookahead = sprite.fetch_dots as i16;
        sprite.x_left <= screen_x + obj_fetch_lookahead
    }

    fn mode3_maybe_trigger_window(bus: &mut Bus, ly: u8, startup_line: bool, screen_x: i16) {
        if bus.ppu.window_triggered_this_line {
            return;
        }
        if !Self::mode3_window_enabled_on_line(bus, ly) {
            bus.ppu.window_trigger_pending = false;
            return;
        }

        let trigger_x = Self::mode3_window_trigger_screen_x(bus);
        let output_x = bus.ppu.mode3_fifo.output_x as i16;
        let reached_trigger = output_x == trigger_x || (trigger_x <= 0 && output_x == 0);
        if !bus.ppu.window_trigger_pending {
            if !reached_trigger {
                return;
            }

            // WX<=7 can start at the beginning of the visible line without
            // waiting for a later BG takeover boundary.
            if Self::mode3_window_trigger_is_immediate(trigger_x) {
                Self::mode3_window_restart_now(bus, ly, startup_line, trigger_x);
                return;
            }
            bus.ppu.window_trigger_pending = true;
        }

        if !Self::mode3_window_takeover_boundary(bus) {
            return;
        }
        if Self::mode3_obj_can_takeover_now(bus, screen_x) {
            return;
        }

        Self::mode3_window_restart_now(bus, ly, startup_line, trigger_x);
    }

    fn mode3_step_bg_fetch(bus: &mut Bus, lcdc: u8, y: usize) {
        match bus.ppu.mode3_fifo.bg_fetch_phase {
            BgFetchPhase::TileIndex | BgFetchPhase::TileDataLow | BgFetchPhase::TileDataHigh => {
                if bus.ppu.mode3_fifo.bg_fetch_dots_remaining > 0 {
                    bus.ppu.mode3_fifo.bg_fetch_dots_remaining -= 1;
                }
                if bus.ppu.mode3_fifo.bg_fetch_dots_remaining != 0 {
                    return;
                }

                match bus.ppu.mode3_fifo.bg_fetch_phase {
                    BgFetchPhase::TileIndex => {
                        let (tile_index, tile_line_addr) =
                            Self::mode3_fetch_tile_index_and_line_addr(
                                bus,
                                lcdc,
                                y,
                                bus.ppu.mode3_fifo.fetch_screen_x,
                            );
                        bus.ppu.mode3_fifo.bg_fetch_tile_index = tile_index;
                        bus.ppu.mode3_fifo.bg_fetch_tile_line_addr = tile_line_addr;
                        bus.ppu.mode3_fifo.bg_fetch_phase = BgFetchPhase::TileDataLow;
                        bus.ppu.mode3_fifo.bg_fetch_dots_remaining = BG_FETCH_PHASE_DOTS;
                        return;
                    }
                    BgFetchPhase::TileDataLow => {
                        bus.ppu.mode3_fifo.bg_fetch_low = bus
                            .read_vram_index_internal(bus.ppu.mode3_fifo.bg_fetch_tile_line_addr);
                        bus.ppu.mode3_fifo.bg_fetch_phase = BgFetchPhase::TileDataHigh;
                        bus.ppu.mode3_fifo.bg_fetch_dots_remaining = BG_FETCH_PHASE_DOTS;
                        return;
                    }
                    BgFetchPhase::TileDataHigh => {
                        bus.ppu.mode3_fifo.bg_fetch_high = bus.read_vram_index_internal(
                            bus.ppu.mode3_fifo.bg_fetch_tile_line_addr + 1,
                        );
                        bus.ppu.mode3_fifo.bg_push_substate = BgPushSubstate::ReadyNormal;
                        bus.ppu.mode3_fifo.bg_fetch_phase = BgFetchPhase::Push;
                    }
                    BgFetchPhase::Push => {}
                }
            }
            BgFetchPhase::Push => {}
        }

        if bus.ppu.mode3_fifo.bg_fetch_phase != BgFetchPhase::Push {
            return;
        }
        if !bus.ppu.mode3_fifo.can_push_8() {
            bus.ppu.mode3_fifo.bg_push_substate = BgPushSubstate::Stalled;
            return;
        }
        if bus.ppu.mode3_fifo.bg_push_substate == BgPushSubstate::RecoverySleep {
            // Explicit one-dot "sleep" micro-op after a FIFO-full push stall. This
            // keeps the fetcher in Push for one more dot before the actual push.
            bus.ppu.mode3_fifo.bg_push_substate = BgPushSubstate::ReadyAfterRecovery;
            return;
        }
        if bus.ppu.mode3_fifo.bg_push_substate == BgPushSubstate::Stalled {
            // Fallback if the prior dot did not latch the explicit recovery-sleep
            // substate after FIFO drain.
            bus.ppu.mode3_fifo.bg_push_substate = BgPushSubstate::RecoverySleep;
            return;
        }

        let fetch_screen_x = bus.ppu.mode3_fifo.fetch_screen_x;
        let bit_x_start = Self::mode3_fetch_bit_x_start(bus, fetch_screen_x);
        if bit_x_start == 0 {
            for lane in 0..8u8 {
                let bit = 7u8.wrapping_sub(lane);
                let color_id = (((bus.ppu.mode3_fifo.bg_fetch_high >> bit) & 1) << 1)
                    | ((bus.ppu.mode3_fifo.bg_fetch_low >> bit) & 1);
                bus.ppu.mode3_fifo.push(BgFifoPixel { color_id });
            }
        } else {
            let mut fetched_pixels = [BgFifoPixel::default(); 8];
            for (lane, pixel) in fetched_pixels.iter_mut().enumerate() {
                *pixel = BgFifoPixel {
                    color_id: Self::mode3_bg_color_id_for_screen_x(
                        bus,
                        lcdc,
                        y,
                        fetch_screen_x + lane as i16,
                    ),
                };
            }
            for pixel in fetched_pixels {
                bus.ppu.mode3_fifo.push(pixel);
            }
        }

        bus.ppu.mode3_fifo.fetch_screen_x += 8;
        bus.ppu.mode3_fifo.bg_fetch_phase = BgFetchPhase::TileIndex;
        bus.ppu.mode3_fifo.bg_fetch_dots_remaining = 0;
        bus.ppu.mode3_fifo.bg_push_substate = BgPushSubstate::ReadyNormal;
    }

    fn extend_mode3_dots(bus: &mut Bus, ly: u8, startup_line: bool, dots: u16) {
        let line_len = Self::line_length_tcycles(bus, ly);
        let mode3_start = Self::mode3_start_tcycle(bus, startup_line);
        let mode3_max_dots = line_len.saturating_sub(mode3_start);
        if bus.ppu.mode3_dots_latched >= mode3_max_dots {
            return;
        }

        let remaining = mode3_max_dots - bus.ppu.mode3_dots_latched;
        let extend = remaining.min(dots);
        bus.ppu.mode3_dots_latched = bus.ppu.mode3_dots_latched.saturating_add(extend);
    }

    fn extend_mode3_for_obj_contention(bus: &mut Bus, ly: u8, startup_line: bool) {
        Self::extend_mode3_dots(bus, ly, startup_line, 1);
    }

    fn mode3_bg_color_id_for_screen_x(bus: &Bus, lcdc: u8, y: usize, screen_x: i16) -> u8 {
        if (lcdc & 0x01) == 0 {
            return 0;
        }

        if bus.ppu.mode3_fifo.window_active {
            return Self::window_color_id_for_screen_x(bus, lcdc, screen_x);
        }

        Self::background_color_id_for_screen_x(bus, lcdc, y, screen_x)
    }

    fn mode3_fetch_tile_index_and_line_addr(
        bus: &Bus,
        lcdc: u8,
        y: usize,
        screen_x: i16,
    ) -> (u8, usize) {
        if bus.ppu.mode3_fifo.window_active {
            let window_map_base = if (lcdc & 0x40) != 0 {
                0x1C00usize
            } else {
                0x1800usize
            };
            let window_x = (screen_x - bus.ppu.mode3_fifo.window_start_x).max(0) as usize;
            let window_y = bus.ppu.window_line_counter as usize;
            let tile_map_index = (window_y / 8) * 32 + (window_x / 8);
            let tile_index = bus.read_vram_index_internal(window_map_base + tile_map_index);
            let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, window_y & 0x07);
            return (tile_index, tile_line_addr);
        }

        let scx = bus.io[0x43];
        let scy = bus.io[0x42];
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
        let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, (bg_y & 0x07) as usize);
        (tile_index, tile_line_addr)
    }

    fn mode3_fetch_bit_x_start(bus: &Bus, screen_x: i16) -> u8 {
        if bus.ppu.mode3_fifo.window_active {
            let window_x = (screen_x - bus.ppu.mode3_fifo.window_start_x).max(0) as usize;
            return (window_x & 0x07) as u8;
        }

        let scx = bus.io[0x43];
        let bg_x = (screen_x as i32 + scx as i32).rem_euclid(256) as usize;
        (bg_x & 0x07) as u8
    }

    fn window_color_id_for_screen_x(bus: &Bus, lcdc: u8, screen_x: i16) -> u8 {
        let window_map_base = if (lcdc & 0x40) != 0 {
            0x1C00usize
        } else {
            0x1800usize
        };

        let window_x = screen_x - bus.ppu.mode3_fifo.window_start_x;
        if window_x < 0 {
            return 0;
        }
        let window_x = window_x as usize;
        let window_y = bus.ppu.window_line_counter as usize;

        let tile_map_index = (window_y / 8) * 32 + (window_x / 8);
        let tile_index = bus.read_vram_index_internal(window_map_base + tile_map_index);
        let tile_line_addr = Self::bg_tile_line_addr(lcdc, tile_index, window_y & 0x07);
        let low = bus.read_vram_index_internal(tile_line_addr);
        let high = bus.read_vram_index_internal(tile_line_addr + 1);
        let bit = 7u8.wrapping_sub((window_x & 0x07) as u8);
        (((high >> bit) & 1) << 1) | ((low >> bit) & 1)
    }

    fn background_color_id_for_screen_x(bus: &Bus, lcdc: u8, y: usize, screen_x: i16) -> u8 {
        let scx = bus.io[0x43];
        let scy = bus.io[0x42];

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

    fn prepare_mode3_obj_line(bus: &mut Bus, y: usize) {
        let lcdc = bus.io[0x40];
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

        bus.ppu.mode3_fifo.obj_set_sprites(sprites, candidate_count);
    }

    fn mode3_step_obj_fetch(bus: &mut Bus, screen_x: i16) -> bool {
        if (bus.io[0x40] & 0x02) == 0 {
            bus.ppu.mode3_fifo.obj_clear_pending();
            return false;
        }

        if bus.ppu.mode3_fifo.obj_fetch_dots_remaining > 0 {
            bus.ppu.mode3_fifo.obj_fetch_dots_remaining -= 1;
            if bus.ppu.mode3_fifo.obj_fetch_dots_remaining == 0
                && let Some(sprite) = bus.ppu.mode3_fifo.obj_active_sprite.take()
            {
                Self::mode3_merge_sprite_into_obj_fifo(bus, sprite, screen_x);
                bus.ppu.mode3_fifo.obj_shutdown_dots_remaining = sprite.post_fetch_dots;
            }
            return true;
        }

        if bus.ppu.mode3_fifo.obj_shutdown_dots_remaining > 0 {
            bus.ppu.mode3_fifo.obj_shutdown_dots_remaining -= 1;
            return true;
        }

        if !Self::mode3_obj_takeover_boundary(bus) {
            return false;
        }

        if bus.ppu.mode3_fifo.obj_next_sprite < bus.ppu.mode3_fifo.obj_sprite_count {
            let sprite = bus.ppu.mode3_fifo.obj_sprites[bus.ppu.mode3_fifo.obj_next_sprite];
            let obj_fetch_lookahead = sprite.fetch_dots as i16;
            if sprite.x_left <= screen_x + obj_fetch_lookahead {
                bus.ppu.mode3_fifo.obj_next_sprite += 1;
                bus.ppu.mode3_fifo.obj_active_sprite = Some(sprite);
                bus.ppu.mode3_fifo.obj_fetch_dots_remaining = sprite.fetch_dots.max(1);
                bus.ppu.mode3_fifo.obj_fetch_dots_remaining -= 1;
                if bus.ppu.mode3_fifo.obj_fetch_dots_remaining == 0 {
                    bus.ppu.mode3_fifo.obj_active_sprite = None;
                    Self::mode3_merge_sprite_into_obj_fifo(bus, sprite, screen_x);
                    bus.ppu.mode3_fifo.obj_shutdown_dots_remaining = sprite.post_fetch_dots;
                }
                return true;
            }
        }

        false
    }

    fn mode3_pop_obj_pixel(bus: &mut Bus) -> ObjFifoPixel {
        if (bus.io[0x40] & 0x02) == 0 {
            bus.ppu.mode3_fifo.obj_clear_pending();
            return ObjFifoPixel::TRANSPARENT;
        }
        bus.ppu.mode3_fifo.obj_ensure_len(1);
        bus.ppu.mode3_fifo.obj_pop()
    }

    fn mode3_merge_sprite_into_obj_fifo(bus: &mut Bus, sprite: Mode3ObjSprite, screen_x: i16) {
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
            let pixel = ObjFifoPixel {
                color_id,
                attr: sprite.attr,
            };
            bus.ppu.mode3_fifo.obj_set_if_transparent(rel, pixel);
        }
    }

    fn compose_mode3_pixel_meta(
        lcdc: u8,
        bg_pixel: BgFifoPixel,
        obj_pixel: ObjFifoPixel,
    ) -> Mode3PixelMeta {
        let bg_enabled = (lcdc & 0x01) != 0;
        let bg_visible_color_id = if bg_enabled { bg_pixel.color_id } else { 0 };
        let priority_flags = Mode3PixelPriorityFlags {
            obj_behind_bg: (obj_pixel.attr & 0x80) != 0,
            bg_color_nonzero: bg_visible_color_id != 0,
        };

        if obj_pixel.color_id == 0
            || (priority_flags.obj_behind_bg && priority_flags.bg_color_nonzero)
        {
            return Mode3PixelMeta {
                color_id: bg_visible_color_id,
                source: Mode3PixelSource::Bg,
                priority_flags,
                dmg_palette: if bg_enabled {
                    DmgPaletteSelector::Bg
                } else {
                    DmgPaletteSelector::ForcedWhite
                },
            };
        }

        Mode3PixelMeta {
            color_id: obj_pixel.color_id,
            source: Mode3PixelSource::Obj,
            priority_flags,
            dmg_palette: if (obj_pixel.attr & 0x10) != 0 {
                DmgPaletteSelector::Obj1
            } else {
                DmgPaletteSelector::Obj0
            },
        }
    }

    fn map_mode3_dmg_shade_id(bus: &Bus, pixel: Mode3PixelMeta) -> u8 {
        let _ = pixel.source;
        let _ = pixel.priority_flags;
        let palette = match pixel.dmg_palette {
            DmgPaletteSelector::ForcedWhite => return 0,
            DmgPaletteSelector::Bg => bus.io[0x47],
            DmgPaletteSelector::Obj0 => bus.io[0x48],
            DmgPaletteSelector::Obj1 => bus.io[0x49],
        };
        (palette >> (pixel.color_id * 2)) & 0x03
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

    fn force_ppu_mode(bus: &mut Bus, mode: PpuMode) {
        bus.ppu.mode = mode;
        bus.ppu.mode_edge_events = PpuModeEdgeEvents::default();
        bus.io[0x41] = (bus.io[0x41] & !0x03) | mode.stat_mode_bits();
    }

    fn set_ppu_mode(bus: &mut Bus, mode: PpuMode) -> PpuModeEdgeEvents {
        if bus.ppu.mode == mode {
            bus.io[0x41] = (bus.io[0x41] & !0x03) | mode.stat_mode_bits();
            bus.ppu.mode_edge_events = PpuModeEdgeEvents::default();
            return bus.ppu.mode_edge_events;
        }

        let edges = PpuModeEdgeEvents::for_entered_mode(mode);
        bus.ppu.mode = mode;
        bus.ppu.mode_edge_events = edges;
        bus.io[0x41] = (bus.io[0x41] & !0x03) | mode.stat_mode_bits();
        edges
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
        let mode = bus.ppu_mode_kind().stat_mode_bits();
        let lyc = (stat & 0x04) != 0;
        let mode_edges = bus.ppu_mode_edge_events();
        // DMG quirk: if mode 2 interrupt is enabled, entering LY=144 also
        // raises STAT alongside VBlank.
        let mode2_or_vblank_start =
            mode == STAT_MODE_OAM || (mode == STAT_MODE_VBLANK && mode_edges.entered_vblank);
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
    pub(super) fn sync_ppu_mode_from_stat_register(&mut self) {
        PpuState::force_ppu_mode(self, PpuMode::from_stat_mode_bits(self.io[0x41]));
    }

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

    pub(in crate::memory) fn ppu_mode_kind(&self) -> PpuMode {
        self.ppu.mode
    }

    pub(in crate::memory) fn ppu_mode_edge_events(&self) -> PpuModeEdgeEvents {
        self.ppu.mode_edge_events
    }

    #[cfg(test)]
    pub(super) fn debug_ppu_mode_kind(&self) -> PpuMode {
        self.ppu.mode
    }

    #[cfg(test)]
    pub(super) fn debug_ppu_mode_edge_events(&self) -> PpuModeEdgeEvents {
        self.ppu.mode_edge_events
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_fifo_len(&self) -> usize {
        self.ppu.mode3_fifo.len
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_fetch_dots_remaining(&self) -> u8 {
        self.ppu.mode3_fifo.bg_fetch_dots_remaining
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_fetch_phase(&self) -> u8 {
        self.ppu.mode3_fifo.bg_fetch_phase as u8
    }

    #[cfg(test)]
    pub(super) fn mode3_obj_fetch_dots_remaining(&self) -> u8 {
        self.ppu.mode3_fifo.obj_fetch_dots_remaining
    }

    #[cfg(test)]
    pub(super) fn mode3_obj_shutdown_dots_remaining(&self) -> u8 {
        self.ppu.mode3_fifo.obj_shutdown_dots_remaining
    }

    #[cfg(test)]
    pub(super) fn mode3_obj_next_sprite_index(&self) -> usize {
        self.ppu.mode3_fifo.obj_next_sprite
    }

    #[cfg(test)]
    pub(super) fn mode3_window_triggered_this_line(&self) -> bool {
        self.ppu.window_triggered_this_line
    }

    #[cfg(test)]
    pub(super) fn mode3_window_trigger_pending(&self) -> bool {
        self.ppu.window_trigger_pending
    }

    #[cfg(test)]
    pub(super) fn mode3_window_takeover_boundary(&self) -> bool {
        PpuState::mode3_window_takeover_boundary(self)
    }

    #[cfg(test)]
    pub(super) fn mode3_output_x(&self) -> u8 {
        self.ppu.mode3_fifo.output_x
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_push_stalled_for_fifo(&self) -> bool {
        self.ppu.mode3_fifo.bg_push_substate == BgPushSubstate::Stalled
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_push_recovery_sleep_pending(&self) -> bool {
        PpuState::mode3_bg_push_recovery_sleep_pending(self)
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_push_ready_takeover_boundary(&self) -> bool {
        PpuState::mode3_bg_push_ready_takeover_boundary(self)
    }

    #[cfg(test)]
    pub(super) fn mode3_bg_takeover_boundary(&self) -> bool {
        PpuState::mode3_bg_takeover_boundary(self)
    }

    #[cfg(test)]
    pub(super) fn debug_compose_mode3_pixel_metadata_and_shade(
        &self,
        lcdc: u8,
        bg_color_id: u8,
        obj_color_id: u8,
        obj_attr: u8,
    ) -> (u8, u8, u8, bool, bool, u8) {
        let pixel = PpuState::compose_mode3_pixel_meta(
            lcdc,
            BgFifoPixel {
                color_id: bg_color_id,
            },
            ObjFifoPixel {
                color_id: obj_color_id,
                attr: obj_attr,
            },
        );
        let palette_code = match pixel.dmg_palette {
            DmgPaletteSelector::ForcedWhite => 0,
            DmgPaletteSelector::Bg => 1,
            DmgPaletteSelector::Obj0 => 2,
            DmgPaletteSelector::Obj1 => 3,
        };
        let source_code = match pixel.source {
            Mode3PixelSource::Bg => 0,
            Mode3PixelSource::Obj => 1,
        };
        let shade_id = PpuState::map_mode3_dmg_shade_id(self, pixel);
        (
            source_code,
            pixel.color_id,
            palette_code,
            pixel.priority_flags.obj_behind_bg,
            pixel.priority_flags.bg_color_nonzero,
            shade_id,
        )
    }
}
