use super::*;

impl DmaState {
    pub(super) fn request_oam_transfer(bus: &mut Bus, source_high: u8) {
        bus.dma.oam.pending_source = Some(Self::normalize_oam_source(source_high));
        // M=0 write, M=1 idle, M=2 transfer starts (DMG/SGB behavior).
        bus.dma.oam.start_delay_tcycles = OAM_DMA_START_DELAY_T_CYCLES;
    }

    fn normalize_oam_source(source_high: u8) -> u16 {
        // On DMG/SGB hardware, FE/FF source pages are remapped to DE/DF.
        let normalized = match source_high {
            0xFE | 0xFF => source_high.wrapping_sub(0x20),
            _ => source_high,
        };
        (normalized as u16) << 8
    }

    pub(super) fn step_oam_start_delay(bus: &mut Bus) -> bool {
        if bus.dma.oam.start_delay_tcycles == 0 {
            return false;
        }

        bus.dma.oam.start_delay_tcycles = bus.dma.oam.start_delay_tcycles.saturating_sub(1);
        bus.dma.oam.start_delay_tcycles == 0
    }

    pub(super) fn step_oam_transfer_tcycle(bus: &mut Bus) {
        debug_assert!(bus.dma.oam.transfer_tcycles_remaining > 0);

        bus.dma.oam.byte_phase_tcycles = bus.dma.oam.byte_phase_tcycles.wrapping_add(1);
        if bus.dma.oam.byte_phase_tcycles == OAM_DMA_BYTE_PERIOD_T_CYCLES {
            bus.dma.oam.byte_phase_tcycles = 0;
            if bus.dma.oam.index < OAM_DMA_BYTES {
                let src = bus.dma.oam.source.wrapping_add(bus.dma.oam.index as u16);
                let value = bus.read_byte_raw(src);
                bus.write_oam_index_internal(bus.dma.oam.index as usize, value);
                bus.dma.oam.index = bus.dma.oam.index.wrapping_add(1);
            }
        }

        bus.dma.oam.transfer_tcycles_remaining =
            bus.dma.oam.transfer_tcycles_remaining.saturating_sub(1);
        if bus.dma.oam.transfer_tcycles_remaining == 0 {
            Self::set_mode(bus, DmaSchedulerMode::Idle);
        }
    }
    pub(super) fn start_or_restart_oam_transfer(bus: &mut Bus) {
        let Some(source) = bus.dma.oam.pending_source.take() else {
            return;
        };

        bus.dma.oam.source = source;
        bus.dma.oam.transfer_tcycles_remaining = OAM_DMA_TRANSFER_T_CYCLES;
        bus.dma.oam.byte_phase_tcycles = 0;
        bus.dma.oam.index = 0;
        Self::set_mode(bus, DmaSchedulerMode::Oam);
    }
}
