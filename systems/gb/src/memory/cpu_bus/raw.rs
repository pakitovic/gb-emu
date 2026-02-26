use super::super::Bus;
use super::super::bus_access::{AddressSegment, SegmentAccess, address_segment};

impl Bus {
    pub(in crate::memory) fn read_byte_raw(&self, addr: u16) -> u8 {
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
}
