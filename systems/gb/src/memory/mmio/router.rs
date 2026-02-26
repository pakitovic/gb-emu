use super::super::Bus;
use super::map::io_unused_bits_mask;

mod decode;

use self::decode::{IoRegisterRoute, decode_io_register};

impl Bus {
    pub(in crate::memory) fn write_io_register(&mut self, addr: u16, value: u8) {
        match decode_io_register(addr) {
            IoRegisterRoute::CgbDmaScaffold => {
                debug_assert!(self.write_cgb_dma_mmio_scaffold(addr, value));
            }
            IoRegisterRoute::CgbMmioScaffold => {
                debug_assert!(self.write_cgb_mmio_scaffold(addr, value));
            }
            IoRegisterRoute::ReservedUnmapped => {}
            IoRegisterRoute::P1 => self.write_p1(value),
            IoRegisterRoute::Sc => self.write_sc(value),
            IoRegisterRoute::Div => self.write_div(value),
            IoRegisterRoute::Tima => self.write_tima(value),
            IoRegisterRoute::Tma => self.write_tma(value),
            IoRegisterRoute::Tac => self.write_tac(value),
            IoRegisterRoute::If => self.write_if(value),
            IoRegisterRoute::ApuWindow => self.write_apu_io_register(addr, value),
            IoRegisterRoute::Lcdc => self.write_lcdc(value),
            IoRegisterRoute::Stat => self.write_stat(value),
            IoRegisterRoute::Ly => self.write_ly(value),
            IoRegisterRoute::Lyc => self.write_lyc(value),
            IoRegisterRoute::Dma => self.write_dma(value),
            IoRegisterRoute::RawBacked => self.io[(addr - 0xFF00) as usize] = value,
        }
    }

    pub(in crate::memory) fn read_io_register(&self, addr: u16) -> u8 {
        let value = match decode_io_register(addr) {
            IoRegisterRoute::CgbDmaScaffold => self
                .read_cgb_dma_mmio_scaffold(addr)
                .expect("CGB DMA scaffold route must decode to a DMA scaffold register"),
            IoRegisterRoute::CgbMmioScaffold => self
                .read_cgb_mmio_scaffold(addr)
                .expect("CGB MMIO scaffold route must decode to a CGB scaffold register"),
            IoRegisterRoute::ReservedUnmapped => 0xFF,
            IoRegisterRoute::P1 => self.read_p1(),
            IoRegisterRoute::Div => self.read_div(),
            IoRegisterRoute::Stat => self.stat_read_value(),
            IoRegisterRoute::ApuWindow => self
                .read_apu_io_register(addr)
                .unwrap_or(self.io[(addr - 0xFF00) as usize]),
            IoRegisterRoute::Sc
            | IoRegisterRoute::Tima
            | IoRegisterRoute::Tma
            | IoRegisterRoute::Tac
            | IoRegisterRoute::If
            | IoRegisterRoute::Lcdc
            | IoRegisterRoute::Ly
            | IoRegisterRoute::Lyc
            | IoRegisterRoute::Dma
            | IoRegisterRoute::RawBacked => self.io[(addr - 0xFF00) as usize],
        };
        value | io_unused_bits_mask(addr)
    }
}
