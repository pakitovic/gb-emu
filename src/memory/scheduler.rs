use super::Bus;

pub(super) struct HardwareScheduler;

impl HardwareScheduler {
    pub(super) fn tick(bus: &mut Bus, cycles: u8) {
        for _ in 0..cycles {
            Self::tick_once(bus);
        }
    }

    fn tick_once(bus: &mut Bus) {
        // Keep DMG timing order stable:
        // TIMA reload -> PPU -> OAM DMA -> DIV edge/serial/TIMA edge -> reload block.
        bus.step_tima_reload();
        bus.step_ppu();
        bus.step_oam_dma();

        let divider_step = bus.step_timer_divider();
        bus.step_apu_frame_sequencer_from_divider(divider_step.old_div, divider_step.new_div);
        bus.step_apu_tcycle();
        bus.step_serial(divider_step.old_div, divider_step.new_div);
        bus.step_timer_falling_edge(divider_step);

        bus.step_tima_reload_block();
    }
}

impl Bus {
    pub fn tick(&mut self, cycles: u8) {
        HardwareScheduler::tick(self, cycles);
    }
}
