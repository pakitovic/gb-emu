use super::super::Cartridge;

impl Cartridge {
    pub(super) fn write_rom_only_control(&mut self, _addr: u16, _value: u8) {
        // ROM-only cartridges ignore writes in the 0x0000..=0x7FFF control window.
    }
}
