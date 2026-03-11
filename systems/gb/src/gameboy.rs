use crate::bootrom::BootRomData;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::hardware::HardwareModel;
use crate::memory::Bus;

mod access;
mod run;
#[cfg(test)]
mod tests;

pub const SCREEN_WIDTH: usize = crate::memory::LCD_WIDTH;
pub const SCREEN_HEIGHT: usize = crate::memory::LCD_HEIGHT;
const RECENT_PC_TRACE_LEN: usize = 16;

pub struct GameBoy {
    cpu: Cpu,
    pub bus: Bus,
    recent_pc_trace: [u16; RECENT_PC_TRACE_LEN],
    recent_pc_trace_head: usize,
    recent_pc_trace_len: usize,
}

impl GameBoy {
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
        let boot_rom_active = boot_rom.is_some();
        Self {
            cpu: Cpu::new_with_model_and_boot_rom(model, boot_rom_active),
            bus: Bus::new_with_model_and_boot_rom(cartridge, model, boot_rom),
            recent_pc_trace: [0; RECENT_PC_TRACE_LEN],
            recent_pc_trace_head: 0,
            recent_pc_trace_len: 0,
        }
    }
}
