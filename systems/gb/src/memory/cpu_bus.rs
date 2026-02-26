mod raw;

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
