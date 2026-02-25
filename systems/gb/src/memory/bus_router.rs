use super::Bus;
use super::bus_access::{AddressSegment, SegmentAccess, address_segment};

impl Bus {
    pub(super) fn read_byte_raw(&self, addr: u16) -> u8 {
        match address_segment(addr) {
            AddressSegment::Rom => self.cartridge.read_rom_byte(addr),
            AddressSegment::Vram => self.read_vram(addr, SegmentAccess::Hardware),
            AddressSegment::Eram => self.cartridge.read_ram_byte(addr),
            AddressSegment::Wram | AddressSegment::EchoWram => self.read_wram(addr),
            AddressSegment::Oam => self.read_oam(addr, SegmentAccess::Hardware),
            AddressSegment::NotUsable => 0xFF,
            AddressSegment::Io => self.read_io_register(addr),
            AddressSegment::Hram => self.hram[(addr - 0xFF80) as usize],
            AddressSegment::Ie => self.ie,
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match address_segment(addr) {
            AddressSegment::Rom => self.cartridge.write_rom_control(addr, value),
            AddressSegment::Vram => self.write_vram(addr, value, SegmentAccess::Cpu),
            AddressSegment::Eram => self.cartridge.write_ram_byte(addr, value),
            AddressSegment::Wram | AddressSegment::EchoWram => self.write_wram(addr, value),
            AddressSegment::Oam => self.write_oam(addr, value, SegmentAccess::Cpu),
            AddressSegment::NotUsable => {}
            AddressSegment::Io => self.write_io_register(addr, value),
            AddressSegment::Hram => self.hram[(addr - 0xFF80) as usize] = value,
            AddressSegment::Ie => self.ie = value,
        }
    }
}
