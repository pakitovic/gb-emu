mod context;
mod flags;
mod helpers;
mod init;
mod instr;
mod interrupts;
mod registers;
mod step;

pub use context::CpuContext;
use flags::{
    get_flag_c, get_flag_h, get_flag_n, get_flag_z, set_flag_c, set_flag_h, set_flag_n, set_flag_z,
};
pub use registers::Registers;

pub struct Cpu {
    registers: Registers,
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

impl Cpu {
    pub fn registers(&self) -> &Registers {
        &self.registers
    }

    pub fn registers_mut(&mut self) -> &mut Registers {
        &mut self.registers
    }
}

#[cfg(test)]
mod tests;
