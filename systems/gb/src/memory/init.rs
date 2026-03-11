mod boot_profiles;
mod io_defaults;

use super::Bus;
use super::bus_access::{VRAM_STORAGE_BYTES, WRAM_STORAGE_BYTES};
use crate::bootrom::{BOOT_ROM_WINDOW_SIZE, BootRomData};
use crate::cartridge::Cartridge;
use crate::hardware::HardwareModel;
use crate::timing::ClockRatios;

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self::new_with_model(cartridge, HardwareModel::default())
    }

    pub fn new_with_model(cartridge: Cartridge, model: HardwareModel) -> Self {
        Self::new_with_model_and_boot_rom(cartridge, model, None)
    }

    pub fn new_with_model_and_boot_rom(
        cartridge: Cartridge,
        model: HardwareModel,
        boot_rom: Option<BootRomData>,
    ) -> Self {
        let (boot_rom, boot_rom_active) = match boot_rom {
            Some(data) => (data, true),
            None => ([0; BOOT_ROM_WINDOW_SIZE], false),
        };

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
            framebuffer_palette_selectors: [1; super::LCD_FRAME_PIXELS],
            clock_ratios: ClockRatios::dmg(),
            hardware_model: model,
            cgb_mmio: Default::default(),
            boot_rom,
            boot_rom_active,
            recent_key_mmio_writes: [(0, 0); super::RECENT_KEY_MMIO_WRITES_LEN],
            recent_key_mmio_writes_head: 0,
            recent_key_mmio_writes_len: 0,
            key_mmio_write_events: Default::default(),
            emulated_tcycles: 0,
        };
        bus.configure_dma_model_gates(model);
        bus.configure_ppu_model_gates(model);
        if !bus.boot_rom_active {
            bus.apply_boot_defaults(model);
        }
        bus.sync_apu_boot_state(model);
        bus.sync_ppu_mode_from_stat_register();
        bus.ppu.stat_irq_line = bus.stat_irq_source_active();
        bus
    }
}
