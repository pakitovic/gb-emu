use crate::memory::{cgb_mmio::cgb_mmio_register, dma::cgb_dma_mmio_register};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory::io_router) enum IoRegisterRoute {
    CgbDmaScaffold,
    CgbMmioScaffold,
    ReservedUnmapped,
    P1,
    Sc,
    Div,
    Tima,
    Tma,
    Tac,
    If,
    ApuWindow,
    Lcdc,
    Stat,
    Ly,
    Lyc,
    Dma,
    RawBacked,
}

#[inline]
pub(in crate::memory::io_router) fn decode_io_register(addr: u16) -> IoRegisterRoute {
    if cgb_dma_mmio_register(addr).is_some() {
        return IoRegisterRoute::CgbDmaScaffold;
    }
    if cgb_mmio_register(addr).is_some() {
        return IoRegisterRoute::CgbMmioScaffold;
    }

    match addr {
        0xFF00 => IoRegisterRoute::P1,
        0xFF02 => IoRegisterRoute::Sc,
        0xFF04 => IoRegisterRoute::Div,
        0xFF05 => IoRegisterRoute::Tima,
        0xFF06 => IoRegisterRoute::Tma,
        0xFF07 => IoRegisterRoute::Tac,
        0xFF0F => IoRegisterRoute::If,
        0xFF10..=0xFF14 | 0xFF16..=0xFF1E | 0xFF20..=0xFF26 | 0xFF30..=0xFF3F => {
            IoRegisterRoute::ApuWindow
        }
        0xFF40 => IoRegisterRoute::Lcdc,
        0xFF41 => IoRegisterRoute::Stat,
        0xFF44 => IoRegisterRoute::Ly,
        0xFF45 => IoRegisterRoute::Lyc,
        0xFF46 => IoRegisterRoute::Dma,

        // Reserved / currently unmapped in DMG-family scope.
        0xFF03 => IoRegisterRoute::ReservedUnmapped,
        0xFF08..=0xFF0E => IoRegisterRoute::ReservedUnmapped,
        0xFF15 => IoRegisterRoute::ReservedUnmapped,
        0xFF1F => IoRegisterRoute::ReservedUnmapped,
        0xFF27..=0xFF2F => IoRegisterRoute::ReservedUnmapped,
        // Most of 0xFF4C..=0xFF7F remains reserved/unmapped in current DMG scope.
        // CGB scaffolds (KEY1/VBK/HDMA/SVBK) are matched above before this range.
        0xFF4C..=0xFF7F => IoRegisterRoute::ReservedUnmapped,

        _ => IoRegisterRoute::RawBacked,
    }
}

#[cfg(test)]
mod tests {
    use super::{IoRegisterRoute, decode_io_register};

    #[test]
    fn cgb_scaffold_registers_are_not_classified_as_reserved_unmapped() {
        for addr in [
            0xFF4D, 0xFF4F, 0xFF51, 0xFF52, 0xFF53, 0xFF54, 0xFF55, 0xFF70,
        ] {
            let route = decode_io_register(addr);
            assert!(
                route == IoRegisterRoute::CgbMmioScaffold
                    || route == IoRegisterRoute::CgbDmaScaffold,
                "expected CGB scaffold route for 0x{addr:04X}, got {route:?}"
            );
        }
    }

    #[test]
    fn reserved_ranges_stay_classified_as_unmapped() {
        for addr in [
            0xFF03, 0xFF08, 0xFF0E, 0xFF15, 0xFF1F, 0xFF27, 0xFF2F, 0xFF4C, 0xFF7F,
        ] {
            assert_eq!(decode_io_register(addr), IoRegisterRoute::ReservedUnmapped);
        }
    }

    #[test]
    fn side_effect_registers_decode_to_explicit_routes() {
        assert_eq!(decode_io_register(0xFF00), IoRegisterRoute::P1);
        assert_eq!(decode_io_register(0xFF04), IoRegisterRoute::Div);
        assert_eq!(decode_io_register(0xFF40), IoRegisterRoute::Lcdc);
        assert_eq!(decode_io_register(0xFF41), IoRegisterRoute::Stat);
        assert_eq!(decode_io_register(0xFF46), IoRegisterRoute::Dma);
        assert_eq!(decode_io_register(0xFF10), IoRegisterRoute::ApuWindow);
        assert_eq!(decode_io_register(0xFF3F), IoRegisterRoute::ApuWindow);
    }
}
