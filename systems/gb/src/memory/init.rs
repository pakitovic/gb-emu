mod boot_profiles;
mod io_defaults;

use super::Bus;
use super::bus_access::{VRAM_STORAGE_BYTES, WRAM_STORAGE_BYTES};
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;
use crate::timing::ClockRatios;

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self::new_with_model(cartridge, HardwareModel::default())
    }

    pub fn new_with_model(cartridge: Cartridge, model: HardwareModel) -> Self {
        let mut bus = Self {
            cartridge,
            vram: [0; VRAM_STORAGE_BYTES],
            wram: [0; WRAM_STORAGE_BYTES],
            oam: [0; 0x00A0],
            io: [0; 0x0080],
            hram: [0; 0x007F],
            ie: 0,
            timer: Default::default(),
            ppu: Default::default(),
            dma: Default::default(),
            apu: Default::default(),
            serial: Default::default(),
            joypad: Default::default(),
            framebuffer: [0xFF; super::LCD_FRAME_PIXELS],
            clock_ratios: ClockRatios::dmg(),
            hardware_model: model,
            cgb_mmio: Default::default(),
        };
        bus.configure_dma_model_gates(model);
        bus.configure_ppu_model_gates(model);
        bus.apply_boot_defaults(model);
        bus.sync_apu_boot_state(model);
        bus.sync_ppu_mode_from_stat_register();
        bus.ppu.stat_irq_line = bus.stat_irq_source_active();
        bus
    }
}
