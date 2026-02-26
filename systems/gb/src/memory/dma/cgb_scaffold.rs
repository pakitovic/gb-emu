use super::*;

impl DmaState {
    pub(super) fn read_cgb_dma_mmio_scaffold(bus: &Bus, addr: u16) -> Option<u8> {
        cgb_dma_mmio_register(addr)?;
        let _ = &bus.dma.cgb_scaffold;
        Some(0xFF)
    }

    pub(super) fn write_cgb_dma_mmio_scaffold(bus: &mut Bus, addr: u16, value: u8) -> bool {
        let Some(reg) = cgb_dma_mmio_register(addr) else {
            return false;
        };
        Self::record_cgb_dma_scaffold_write(bus, reg, value);
        true
    }

    fn record_cgb_dma_scaffold_write(bus: &mut Bus, reg: CgbDmaMmioRegister, value: u8) {
        match reg {
            // Keep only future-relevant bits but remain DMG-noop externally.
            CgbDmaMmioRegister::Hdma1 => bus.dma.cgb_scaffold.hdma1_shadow = value,
            CgbDmaMmioRegister::Hdma2 => bus.dma.cgb_scaffold.hdma2_shadow = value & 0xF0,
            CgbDmaMmioRegister::Hdma3 => bus.dma.cgb_scaffold.hdma3_shadow = value & 0x1F,
            CgbDmaMmioRegister::Hdma4 => bus.dma.cgb_scaffold.hdma4_shadow = value & 0xF0,
            CgbDmaMmioRegister::Hdma5 => {
                bus.dma.cgb_scaffold.hdma5_shadow = value;
                let requested_mode = Self::cgb_scheduler_mode_from_hdma5(value);
                bus.dma.cgb_scaffold.last_requested_mode = Some(requested_mode);
                Self::handle_hdma5_transfer_request_scaffold(bus, value, requested_mode);
            }
        }
    }

    fn handle_hdma5_transfer_request_scaffold(
        bus: &mut Bus,
        value: u8,
        requested_mode: DmaSchedulerMode,
    ) {
        if !bus.dma.cgb_scaffold.runtime_enabled {
            return;
        }

        if matches!(bus.dma.mode, DmaSchedulerMode::Hdma)
            && matches!(requested_mode, DmaSchedulerMode::Gdma)
        {
            // CGB: writing HDMA5 with bit7=0 while HBlank DMA is active requests stop.
            Self::stop_active_hdma_scaffold(bus);
            return;
        }

        if matches!(
            bus.dma.mode,
            DmaSchedulerMode::Gdma | DmaSchedulerMode::Hdma
        ) {
            // Keep the decode/shadow side effects but avoid restart semantics until
            // CGB mode support is implemented for real.
            return;
        }

        let (source, dest, blocks_remaining) =
            Self::cgb_transfer_params_from_hdma_shadows(bus, value);
        bus.dma.cgb_scaffold.transfer_source = source;
        bus.dma.cgb_scaffold.transfer_dest = dest;
        bus.dma.cgb_scaffold.transfer_blocks_remaining = blocks_remaining;
        bus.dma.cgb_scaffold.pending_request_mode = Some(requested_mode);
        Self::update_hdma5_status_shadow(bus);
    }

    fn cgb_scheduler_mode_from_hdma5(value: u8) -> DmaSchedulerMode {
        if (value & 0x80) != 0 {
            DmaSchedulerMode::Hdma
        } else {
            DmaSchedulerMode::Gdma
        }
    }

    fn cgb_transfer_params_from_hdma_shadows(bus: &Bus, hdma5_value: u8) -> (u16, u16, u8) {
        let source = (((bus.dma.cgb_scaffold.hdma1_shadow as u16) << 8)
            | (bus.dma.cgb_scaffold.hdma2_shadow as u16))
            & 0xFFF0;
        let dest = 0x8000
            | ((((bus.dma.cgb_scaffold.hdma3_shadow as u16) & 0x1F) << 8)
                | (bus.dma.cgb_scaffold.hdma4_shadow as u16))
                & 0x1FF0;
        let blocks_remaining = (hdma5_value & 0x7F).wrapping_add(1);
        (source, dest, blocks_remaining)
    }

    pub(super) fn model_supports_cgb_dma(_model: HardwareModel) -> bool {
        // Current project scope exposes only DMG-family models.
        false
    }

    pub(super) fn step_gdma_transfer_scaffold(bus: &mut Bus) {
        if !bus.dma.cgb_scaffold.runtime_enabled {
            return;
        }

        if !Self::cgb_dma_transfer_one_block_scaffold(bus) {
            Self::set_mode(bus, DmaSchedulerMode::Idle);
            Self::update_hdma5_status_shadow(bus);
        }
    }

    pub(super) fn step_hdma_transfer_scaffold(bus: &mut Bus) {
        if !bus.dma.cgb_scaffold.runtime_enabled {
            return;
        }

        if !bus.ppu_mode_edge_events().entered_hblank {
            return;
        }

        if !Self::cgb_dma_transfer_one_block_scaffold(bus) {
            Self::set_mode(bus, DmaSchedulerMode::Idle);
            Self::update_hdma5_status_shadow(bus);
        }
    }

    fn cgb_dma_transfer_one_block_scaffold(bus: &mut Bus) -> bool {
        if bus.dma.cgb_scaffold.transfer_blocks_remaining == 0 {
            return false;
        }

        for _ in 0..0x10 {
            let src = bus.dma.cgb_scaffold.transfer_source;
            let dst = bus.dma.cgb_scaffold.transfer_dest;
            let value = bus.read_byte_raw(src);
            bus.write_vram(dst, value, SegmentAccess::Hardware);
            bus.dma.cgb_scaffold.transfer_source = src.wrapping_add(1);
            bus.dma.cgb_scaffold.transfer_dest = Self::cgb_dma_next_vram_dest_addr(dst);
        }

        bus.dma.cgb_scaffold.transfer_blocks_remaining = bus
            .dma
            .cgb_scaffold
            .transfer_blocks_remaining
            .saturating_sub(1);
        Self::update_hdma5_status_shadow(bus);
        bus.dma.cgb_scaffold.transfer_blocks_remaining > 0
    }

    fn cgb_dma_next_vram_dest_addr(addr: u16) -> u16 {
        let offset = (addr.wrapping_sub(0x8000).wrapping_add(1)) & 0x1FFF;
        0x8000 | offset
    }

    pub(super) fn start_pending_cgb_dma_transfer(bus: &mut Bus) {
        if !bus.dma.cgb_scaffold.runtime_enabled {
            return;
        }
        if !matches!(bus.dma.mode, DmaSchedulerMode::Idle) {
            return;
        }

        let Some(mode) = bus.dma.cgb_scaffold.pending_request_mode.take() else {
            return;
        };
        debug_assert!(matches!(
            mode,
            DmaSchedulerMode::Gdma | DmaSchedulerMode::Hdma
        ));
        if bus.dma.cgb_scaffold.transfer_blocks_remaining == 0 {
            Self::update_hdma5_status_shadow(bus);
            return;
        }

        Self::set_mode(bus, mode);
        Self::update_hdma5_status_shadow(bus);
    }

    fn stop_active_hdma_scaffold(bus: &mut Bus) {
        debug_assert!(matches!(bus.dma.mode, DmaSchedulerMode::Hdma));
        bus.dma.cgb_scaffold.pending_request_mode = None;
        Self::set_mode(bus, DmaSchedulerMode::Idle);
        Self::update_hdma5_status_shadow(bus);
    }

    fn update_hdma5_status_shadow(bus: &mut Bus) {
        let low = bus
            .dma
            .cgb_scaffold
            .transfer_blocks_remaining
            .saturating_sub(1)
            & 0x7F;
        let active = matches!(
            bus.dma.mode,
            DmaSchedulerMode::Gdma | DmaSchedulerMode::Hdma
        ) || matches!(
            bus.dma.cgb_scaffold.pending_request_mode,
            Some(DmaSchedulerMode::Gdma | DmaSchedulerMode::Hdma)
        );
        bus.dma.cgb_scaffold.hdma5_shadow = if active { low } else { 0x80 | low };
    }
}
