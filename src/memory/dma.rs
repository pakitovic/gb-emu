use super::Bus;

impl Bus {
    pub(super) fn start_oam_dma(&mut self, source_high: u8) {
        // On DMG/SGB hardware, FE/FF source pages are remapped to DE/DF.
        let source_high = match source_high {
            0xFE | 0xFF => source_high.wrapping_sub(0x20),
            _ => source_high,
        };
        self.dma_active = true;
        self.dma_source = (source_high as u16) << 8;
        self.dma_cycles_remaining = 640; // 160 bytes * 4 t-cycles
        self.dma_start_delay = 8; // includes one extra M-cycle setup delay
        self.dma_cycle_accum = 0;
        self.dma_index = 0;
    }

    pub(super) fn step_oam_dma(&mut self) {
        if !self.dma_active {
            return;
        }

        if self.dma_start_delay > 0 {
            self.dma_start_delay -= 1;
            return;
        }

        self.dma_cycle_accum = self.dma_cycle_accum.wrapping_add(1);
        if self.dma_cycle_accum == 4 {
            self.dma_cycle_accum = 0;
            if self.dma_index < 0xA0 {
                let src = self.dma_source.wrapping_add(self.dma_index as u16);
                let value = self.read_byte_raw(src);
                self.oam[self.dma_index as usize] = value;
                self.dma_index = self.dma_index.wrapping_add(1);
            }
        }

        self.dma_cycles_remaining = self.dma_cycles_remaining.saturating_sub(1);
        if self.dma_cycles_remaining == 0 {
            self.dma_active = false;
        }
    }
}
