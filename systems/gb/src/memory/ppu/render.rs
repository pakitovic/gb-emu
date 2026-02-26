use super::*;

impl PpuState {
    pub(super) fn compose_mode3_pixel_meta(
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
                cgb_scaffold: Mode3CgbPixelMetaScaffold {
                    bg_attrs: bg_pixel.cgb_bg_attrs,
                    obj_attrs: obj_pixel.cgb_obj_attrs,
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
            cgb_scaffold: Mode3CgbPixelMetaScaffold {
                bg_attrs: bg_pixel.cgb_bg_attrs,
                obj_attrs: obj_pixel.cgb_obj_attrs,
            },
        }
    }

    pub(super) fn map_mode3_dmg_shade_id(bus: &Bus, pixel: Mode3PixelMeta) -> u8 {
        let _ = pixel.source;
        let _ = pixel.priority_flags;
        let palette = match pixel.dmg_palette {
            DmgPaletteSelector::ForcedWhite => return 0,
            DmgPaletteSelector::Bg => bus.ppu_bgp(),
            DmgPaletteSelector::Obj0 => bus.ppu_obp0(),
            DmgPaletteSelector::Obj1 => bus.ppu_obp1(),
        };
        (palette >> (pixel.color_id * 2)) & 0x03
    }
}
