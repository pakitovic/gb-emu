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

impl Bus {
    pub(super) fn start_oam_dma(&mut self, source_high: u8) {
        // On DMG/SGB hardware, FE/FF source pages are remapped to DE/DF.
        let source_high = match source_high {
            0xFE | 0xFF => source_high.wrapping_sub(0x20),
            _ => source_high,
        };
        self.dma.pending_source = (source_high as u16) << 8;
        self.dma.start_delay = 8; // M=0 write, M=1 idle, M=2 DMA starts
    }

    pub(super) fn step_oam_dma(&mut self) {
        if self.dma.start_delay > 0 {
            self.dma.start_delay -= 1;
            if self.dma.start_delay == 0 {
                // Start (or restart) DMA. For restarts, previous DMA keeps running
                // until this moment, then the new transfer takes over.
                self.dma.active = true;
                self.dma.source = self.dma.pending_source;
                self.dma.cycles_remaining = 640; // 160 bytes * 4 t-cycles
                self.dma.cycle_accum = 0;
                self.dma.index = 0;
                return;
            }
        }

        if !self.dma.active {
            return;
        }

        self.dma.cycle_accum = self.dma.cycle_accum.wrapping_add(1);
        if self.dma.cycle_accum == 4 {
            self.dma.cycle_accum = 0;
            if self.dma.index < 0xA0 {
                let src = self.dma.source.wrapping_add(self.dma.index as u16);
                let value = self.read_byte_raw(src);
                self.oam[self.dma.index as usize] = value;
                self.dma.index = self.dma.index.wrapping_add(1);
            }
        }

        self.dma.cycles_remaining = self.dma.cycles_remaining.saturating_sub(1);
        if self.dma.cycles_remaining == 0 {
            self.dma.active = false;
        }
    }
}
