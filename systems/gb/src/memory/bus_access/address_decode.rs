#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum AddressSegment {
    Rom,
    Vram,
    Eram,
    Wram,
    EchoWram,
    Oam,
    NotUsable,
    Io,
    Hram,
    Ie,
}

pub(in crate::memory) fn address_segment(addr: u16) -> AddressSegment {
    match addr {
        0x0000..=0x7FFF => AddressSegment::Rom,
        0x8000..=0x9FFF => AddressSegment::Vram,
        0xA000..=0xBFFF => AddressSegment::Eram,
        0xC000..=0xDFFF => AddressSegment::Wram,
        0xE000..=0xFDFF => AddressSegment::EchoWram,
        0xFE00..=0xFE9F => AddressSegment::Oam,
        0xFEA0..=0xFEFF => AddressSegment::NotUsable,
        0xFF00..=0xFF7F => AddressSegment::Io,
        0xFF80..=0xFFFE => AddressSegment::Hram,
        0xFFFF => AddressSegment::Ie,
    }
}
