mod interrupts;
mod joypad;
mod serial;
mod timer;

pub(in crate::memory) use joypad::JoypadState;
pub(in crate::memory) use serial::SerialState;
pub(in crate::memory) use timer::TimerState;
