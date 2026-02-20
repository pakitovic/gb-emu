use super::{
    Bus,
    io_map::{io_unused_bits_mask, is_unmapped_io},
};

impl Bus {
    pub(super) fn write_io_register(&mut self, addr: u16, value: u8) {
        if is_unmapped_io(addr) {
            return;
        }

        match addr {
            0xFF00 => self.write_p1(value),
            0xFF0F => self.write_if(value),
            0xFF40 => self.write_lcdc(value),
            0xFF41 => self.write_stat(value),
            0xFF24 => self.write_nr50(value),
            0xFF25 => self.write_nr51(value),
            0xFF26 => self.write_nr52(value),
            0xFF04 => self.write_div(value),
            0xFF46 => self.write_dma(value),
            0xFF02 => self.write_sc(value),
            0xFF07 => self.write_tac(value),
            0xFF44 => self.write_ly(value),
            0xFF45 => self.write_lyc(value),
            0xFF05 => self.write_tima(value),
            0xFF06 => self.write_tma(value),
            _ => self.io[(addr - 0xFF00) as usize] = value,
        }
    }

    pub(super) fn read_io_register(&self, addr: u16) -> u8 {
        if is_unmapped_io(addr) {
            return 0xFF;
        }

        let value = match addr {
            0xFF00 => self.read_p1(),
            0xFF04 => self.read_div(),
            0xFF41 => self.stat_read_value(),
            0xFF26 => self.read_nr52(),
            _ => self.io[(addr - 0xFF00) as usize],
        };
        value | io_unused_bits_mask(addr)
    }
}
