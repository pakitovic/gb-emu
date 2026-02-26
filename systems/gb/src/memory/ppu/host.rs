use super::*;

impl Bus {
    pub(in crate::memory) fn configure_ppu_model_gates(&mut self, model: HardwareModel) {
        PpuState::configure_model_gates(self, model);
    }

    pub(in crate::memory) fn sync_ppu_mode_from_stat_register(&mut self) {
        PpuState::force_ppu_mode(self, PpuMode::from_stat_mode_bits(self.ppu_stat()));
    }

    pub(in crate::memory) fn ppu_blocks_oam_read(&self) -> bool {
        PpuState::ppu_blocks_oam_read(self)
    }

    pub(in crate::memory) fn ppu_blocks_oam_write(&self) -> bool {
        PpuState::ppu_blocks_oam_write(self)
    }

    pub(in crate::memory) fn ppu_blocks_vram_read(&self) -> bool {
        PpuState::ppu_blocks_vram_read(self)
    }

    pub(in crate::memory) fn ppu_blocks_vram_write(&self) -> bool {
        PpuState::ppu_blocks_vram_write(self)
    }

    pub(in crate::memory) fn stat_read_value(&self) -> u8 {
        PpuState::stat_read_value(self)
    }

    pub(in crate::memory) fn write_lcdc(&mut self, value: u8) {
        PpuState::write_lcdc(self, value);
    }

    pub(in crate::memory) fn write_stat(&mut self, value: u8) {
        PpuState::write_stat(self, value);
    }

    pub(in crate::memory) fn write_lyc(&mut self, value: u8) {
        PpuState::write_lyc(self, value);
    }

    pub(in crate::memory) fn write_ly(&mut self, value: u8) {
        PpuState::write_ly(self, value);
    }

    pub(in crate::memory) fn step_ppu(&mut self) {
        PpuState::step(self);
    }

    pub(in crate::memory) fn stat_irq_source_active(&self) -> bool {
        PpuState::stat_irq_source_active(self)
    }

    pub(in crate::memory) fn ppu_mode_kind(&self) -> PpuMode {
        self.ppu.mode
    }

    pub(in crate::memory) fn ppu_mode_edge_events(&self) -> PpuModeEdgeEvents {
        self.ppu.mode_edge_events
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_ppu_mode_kind(&self) -> PpuMode {
        self.ppu.mode
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_ppu_mode_edge_events(&self) -> PpuModeEdgeEvents {
        self.ppu.mode_edge_events
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_bg_fifo_len(&self) -> usize {
        self.ppu.mode3_fifo.len
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_bg_fetch_dots_remaining(&self) -> u8 {
        self.ppu.mode3_fifo.bg_fetch_dots_remaining
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_bg_fetch_phase(&self) -> u8 {
        self.ppu.mode3_fifo.bg_fetch_phase as u8
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_obj_fetch_dots_remaining(&self) -> u8 {
        self.ppu.mode3_fifo.obj_fetch_dots_remaining
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_obj_shutdown_dots_remaining(&self) -> u8 {
        self.ppu.mode3_fifo.obj_shutdown_dots_remaining
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_obj_next_sprite_index(&self) -> usize {
        self.ppu.mode3_fifo.obj_next_sprite
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_window_triggered_this_line(&self) -> bool {
        self.ppu.window_triggered_this_line
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_window_trigger_pending(&self) -> bool {
        self.ppu.window_trigger_pending
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_window_takeover_boundary(&self) -> bool {
        PpuState::mode3_window_takeover_boundary(self)
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_output_x(&self) -> u8 {
        self.ppu.mode3_fifo.output_x
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_bg_push_stalled_for_fifo(&self) -> bool {
        self.ppu.mode3_fifo.bg_push_substate == BgPushSubstate::Stalled
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_bg_push_recovery_sleep_pending(&self) -> bool {
        PpuState::mode3_bg_push_recovery_sleep_pending(self)
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_bg_push_ready_takeover_boundary(&self) -> bool {
        PpuState::mode3_bg_push_ready_takeover_boundary(self)
    }

    #[cfg(test)]
    pub(in crate::memory) fn mode3_bg_takeover_boundary(&self) -> bool {
        PpuState::mode3_bg_takeover_boundary(self)
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_compose_mode3_pixel_metadata_and_shade(
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
                cgb_bg_attrs: CgbBgTileAttrsScaffold::default(),
            },
            ObjFifoPixel {
                color_id: obj_color_id,
                attr: obj_attr,
                cgb_obj_attrs: PpuState::decode_cgb_obj_attrs_scaffold(obj_attr),
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

    #[cfg(test)]
    pub(in crate::memory) fn debug_compose_mode3_pixel_cgb_scaffold_and_shade(
        &self,
        lcdc: u8,
        bg_color_id: u8,
        bg_attr_byte: u8,
        obj_color_id: u8,
        obj_attr: u8,
    ) -> (u8, u8, bool, bool, u8, u8, u8) {
        let bg_attrs = PpuState::decode_cgb_bg_tile_attrs_scaffold(bg_attr_byte);
        let obj_attrs = PpuState::decode_cgb_obj_attrs_scaffold(obj_attr);
        let pixel = PpuState::compose_mode3_pixel_meta(
            lcdc,
            BgFifoPixel {
                color_id: bg_color_id,
                cgb_bg_attrs: bg_attrs,
            },
            ObjFifoPixel {
                color_id: obj_color_id,
                attr: obj_attr,
                cgb_obj_attrs: obj_attrs,
            },
        );
        let shade_id = PpuState::map_mode3_dmg_shade_id(self, pixel);
        (
            pixel.cgb_scaffold.bg_attrs.palette_index,
            pixel.cgb_scaffold.bg_attrs.vram_bank,
            pixel.cgb_scaffold.bg_attrs.x_flip,
            pixel.cgb_scaffold.bg_attrs.bg_priority,
            pixel.cgb_scaffold.obj_attrs.palette_index,
            pixel.cgb_scaffold.obj_attrs.vram_bank,
            shade_id,
        )
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_mode3_bg_tile_attrs_scaffold_for_screen_x(
        &self,
        lcdc: u8,
        y: usize,
        screen_x: i16,
    ) -> (u8, u8, bool, bool, bool) {
        let (_tile_index, attrs, _tile_line_addr) =
            PpuState::mode3_fetch_tile_index_and_line_addr(self, lcdc, y, screen_x);
        (
            attrs.palette_index,
            attrs.vram_bank,
            attrs.x_flip,
            attrs.y_flip,
            attrs.bg_priority,
        )
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_force_enable_ppu_cgb_scaffold_runtime(&mut self, enabled: bool) {
        self.ppu.cgb_scaffold_runtime_enabled = enabled;
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_ppu_cgb_scaffold_runtime_enabled(&self) -> bool {
        self.ppu.cgb_scaffold_runtime_enabled
    }
}
