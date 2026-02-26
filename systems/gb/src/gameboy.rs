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

pub struct GameBoy {
    cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new(cartridge: Cartridge) -> Self {
        Self::new_with_model(cartridge, HardwareModel::default())
    }

    pub fn new_with_model(cartridge: Cartridge, model: HardwareModel) -> Self {
        Self {
            cpu: Cpu::new_with_model(model),
            bus: Bus::new_with_model(cartridge, model),
        }
    }
}
