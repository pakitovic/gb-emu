use super::Bus;
use super::bus_access::{AddressSegment, SegmentAccess, address_segment};

impl Bus {
    pub fn read_byte(&self, addr: u16) -> u8 {
        match address_segment(addr) {
            AddressSegment::Vram => self.read_vram(addr, SegmentAccess::Cpu),
            AddressSegment::Oam => self.read_oam(addr, SegmentAccess::Cpu),
            _ => self.read_byte_raw(addr),
        }
    }
}
