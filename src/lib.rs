pub mod audio;
pub mod cartridge;
pub mod cpu;
pub mod gameboy;
pub mod hardware;
pub mod input;
pub mod memory;
pub mod timing;

#[cfg(feature = "frontend-web")]
pub mod web;
