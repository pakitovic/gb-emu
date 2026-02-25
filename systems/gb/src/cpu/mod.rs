mod flags;
mod helpers;
mod init;
mod instr;
mod interrupts;
pub mod registers;
mod step;

use registers::Registers;

use flags::{
    get_flag_c, get_flag_h, get_flag_n, get_flag_z, set_flag_c, set_flag_h, set_flag_n, set_flag_z,
};

pub trait CpuContext {
    fn read_byte(&self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, value: u8);
    // Advance hardware by DMG base t-cycles (4_194_304 Hz domain).
    fn tick(&mut self, tcycles: u8);
    fn pending_interrupts(&self) -> u8;
    fn interrupt_flags(&self) -> u8;
    fn set_interrupt_flags(&mut self, value: u8);
}

pub struct Cpu {
    pub registers: Registers,
    ime: bool,
    ime_enable_delay: u8,
    halted: bool,
    halt_bug: bool,
    step_tcycles: u8,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
