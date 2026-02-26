use super::bus::PpuStateAdapter;
use super::*;

impl PpuState {
    pub(in crate::memory) fn step(bus: &mut Bus) {
        bus.ppu_state_mut().mode_edge_events = PpuModeEdgeEvents::default();

        if !Self::lcd_enabled(bus) {
            return;
        }

        if bus.ppu_state().enable_delay > 0 {
            bus.ppu_state_mut().enable_delay -= 1;
            Self::update_lyc_flag(bus);
            Self::update_stat_irq_line(bus);
            return;
        }

        let ly = bus.ppu_ly();
        if ly < 144 && bus.ppu_state().ly_counter == 0 {
            let startup_line = bus.ppu_state().startup_line && ly == 0;
            bus.ppu_state_mut().mode3_dots_latched =
                Self::mode3_length_tcycles(bus, ly, startup_line);
            bus.ppu_state_mut().window_triggered_this_line = false;
            bus.ppu_state_mut().window_trigger_pending = false;
        }

        if ly < 144 {
            let startup_line = bus.ppu_state().startup_line && ly == 0;
            let ly_counter = bus.ppu_state().ly_counter;
            Self::render_mode3_dot(bus, ly, ly_counter, startup_line);
        }

        let line_length = Self::line_length_tcycles(bus, ly);
        bus.ppu_state_mut().ly_counter = bus.ppu_state().ly_counter.wrapping_add(1);
        if bus.ppu_state().ly_counter >= line_length {
            bus.ppu_state_mut().ly_counter = 0;
            if ly < 144 {
                if bus.ppu_state().window_triggered_this_line {
                    let window_line_counter = bus.ppu_state().window_line_counter.wrapping_add(1);
                    bus.ppu_state_mut().window_line_counter = window_line_counter;
                }
                bus.ppu_state_mut().mode3_fifo.reset();
            }
            let next_ly = if ly >= 153 { 0 } else { ly.wrapping_add(1) };
            bus.ppu_set_ly(next_ly);
            bus.ppu_state_mut().stat_mode0_enabled_this_line = false;
            bus.ppu_state_mut().window_triggered_this_line = false;
            bus.ppu_state_mut().window_trigger_pending = false;

            if bus.ppu_state().startup_line && ly == 0 {
                bus.ppu_state_mut().startup_line = false;
                bus.ppu_state_mut().post_enable_phase = 2;
            } else if bus.ppu_state().post_enable_phase > 0 {
                bus.ppu_state_mut().post_enable_phase -= 1;
            }
            if next_ly == 0 {
                bus.ppu_state_mut().window_line_counter = 0;
            }
        }

        let ly = bus.ppu_ly();
        let mode = if ly >= 144 {
            STAT_MODE_VBLANK
        } else {
            Self::mode_for_visible_line(
                bus,
                ly,
                bus.ppu_state().ly_counter,
                bus.ppu_state().startup_line && ly == 0,
            )
        };
        let mode_edges = Self::set_ppu_mode(bus, PpuMode::from_stat_mode_bits(mode));
        if mode_edges.entered_vblank {
            let iflags = bus.interrupt_flags() | (1 << 0);
            bus.set_interrupt_flags(iflags);
            bus.ppu_state_mut().frame_counter = bus.ppu_state().frame_counter.wrapping_add(1);
        }
        Self::update_lyc_flag(bus);
        Self::update_stat_irq_line(bus);
    }

    pub(super) fn line_length_tcycles(bus: &Bus, ly: u8) -> u16 {
        if bus.ppu_state().startup_line && ly == 0 {
            STARTUP_LINE_DOTS
        } else {
            456
        }
    }

    pub(super) fn mode_for_visible_line(
        bus: &Bus,
        ly: u8,
        line_cycle: u16,
        startup_line: bool,
    ) -> u8 {
        let mode3_dots = if line_cycle == 0 {
            Self::mode3_length_tcycles(bus, ly, startup_line)
        } else {
            bus.ppu_state().mode3_dots_latched
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
            let mode2_end = match bus.ppu_state().post_enable_phase {
                2 => 84u16,
                1 => 84u16,
                _ => 80u16,
            };

            if bus.ppu_state().post_enable_phase == 0 {
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

    pub(super) fn mode3_length_tcycles(bus: &Bus, ly: u8, startup_line: bool) -> u16 {
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

    pub(super) fn mode3_extra_tcycles(bus: &Bus) -> u16 {
        (bus.ppu_scx() & 0x07) as u16
    }

    pub(super) fn extend_mode3_dots(bus: &mut Bus, ly: u8, startup_line: bool, dots: u16) {
        let line_len = Self::line_length_tcycles(bus, ly);
        let mode3_start = Self::mode3_start_tcycle(bus, startup_line);
        let mode3_max_dots = line_len.saturating_sub(mode3_start);
        if bus.ppu_state().mode3_dots_latched >= mode3_max_dots {
            return;
        }

        let remaining = mode3_max_dots - bus.ppu_state().mode3_dots_latched;
        let extend = remaining.min(dots);
        bus.ppu_state_mut().mode3_dots_latched =
            bus.ppu_state().mode3_dots_latched.saturating_add(extend);
    }

    pub(super) fn extend_mode3_for_obj_contention(bus: &mut Bus, ly: u8, startup_line: bool) {
        Self::extend_mode3_dots(bus, ly, startup_line, 1);
    }
}
