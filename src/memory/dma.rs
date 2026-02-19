use super::Bus;

#[derive(Default)]
pub(super) struct DmaState {
    pub(super) active: bool,
    pub(super) source: u16,
    pub(super) pending_source: u16,
    pub(super) cycles_remaining: u16,
    pub(super) start_delay: u8,
    pub(super) cycle_accum: u8,
    pub(super) index: u8,
}

impl DmaState {
    pub(super) fn write_register(bus: &mut Bus, source_high: u8) {
        bus.io[0x46] = source_high;
        Self::start(bus, source_high);
    }

    pub(super) fn start(bus: &mut Bus, source_high: u8) {
        // On DMG/SGB hardware, FE/FF source pages are remapped to DE/DF.
        let source_high = match source_high {
            0xFE | 0xFF => source_high.wrapping_sub(0x20),
            _ => source_high,
        };
        bus.dma.pending_source = (source_high as u16) << 8;
        bus.dma.start_delay = 8; // M=0 write, M=1 idle, M=2 DMA starts
    }

    pub(super) fn step(bus: &mut Bus) {
        if bus.dma.start_delay > 0 {
            bus.dma.start_delay -= 1;
            if bus.dma.start_delay == 0 {
                // Start (or restart) DMA. For restarts, previous DMA keeps running
                // until this moment, then the new transfer takes over.
                bus.dma.active = true;
                bus.dma.source = bus.dma.pending_source;
                bus.dma.cycles_remaining = 640; // 160 bytes * 4 t-cycles
                bus.dma.cycle_accum = 0;
                bus.dma.index = 0;
                return;
            }
        }

        if !bus.dma.active {
            return;
        }

        bus.dma.cycle_accum = bus.dma.cycle_accum.wrapping_add(1);
        if bus.dma.cycle_accum == 4 {
            bus.dma.cycle_accum = 0;
            if bus.dma.index < 0xA0 {
                let src = bus.dma.source.wrapping_add(bus.dma.index as u16);
                let value = bus.read_byte_raw(src);
                bus.oam[bus.dma.index as usize] = value;
                bus.dma.index = bus.dma.index.wrapping_add(1);
            }
        }

        bus.dma.cycles_remaining = bus.dma.cycles_remaining.saturating_sub(1);
        if bus.dma.cycles_remaining == 0 {
            bus.dma.active = false;
        }
    }
}

impl Bus {
    pub(super) fn write_dma(&mut self, source_high: u8) {
        DmaState::write_register(self, source_high);
    }

    pub(super) fn step_oam_dma(&mut self) {
        DmaState::step(self);
    }
}
