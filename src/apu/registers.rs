use super::{NR10_INDEX, NR50_INDEX, NR51_INDEX, WAVE_RAM_END_INDEX, WAVE_RAM_START_INDEX};

const NR_WINDOW_LEN: usize = NR51_INDEX - NR10_INDEX + 1;
const WAVE_RAM_LEN: usize = WAVE_RAM_END_INDEX - WAVE_RAM_START_INDEX + 1;

#[derive(Clone)]
pub(in crate::apu) struct ApuRegisters {
    nr_window: [u8; NR_WINDOW_LEN],
    wave_ram: [u8; WAVE_RAM_LEN],
}

impl Default for ApuRegisters {
    fn default() -> Self {
        Self {
            nr_window: [0; NR_WINDOW_LEN],
            wave_ram: [0; WAVE_RAM_LEN],
        }
    }
}

impl ApuRegisters {
    pub(in crate::apu) fn from_io(io: &[u8; 0x80]) -> Self {
        let mut registers = Self::default();
        registers
            .nr_window
            .copy_from_slice(&io[NR10_INDEX..=NR51_INDEX]);
        registers
            .wave_ram
            .copy_from_slice(&io[WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX]);
        registers
    }

    pub(in crate::apu) fn write_index(&mut self, index: usize, value: u8) {
        if (NR10_INDEX..=NR51_INDEX).contains(&index) {
            self.nr_window[index - NR10_INDEX] = value;
            return;
        }
        if (WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX).contains(&index) {
            self.wave_ram[index - WAVE_RAM_START_INDEX] = value;
        }
    }

    pub(in crate::apu) fn clear_nr_window(&mut self) {
        self.nr_window.fill(0);
    }

    pub(in crate::apu) fn nr50(&self) -> u8 {
        self.nr_window[NR50_INDEX - NR10_INDEX]
    }

    pub(in crate::apu) fn nr51(&self) -> u8 {
        self.nr_window[NR51_INDEX - NR10_INDEX]
    }

    pub(in crate::apu) fn wave_sample_byte(&self, wave_position: u8) -> u8 {
        self.wave_ram[(wave_position as usize) / 2]
    }
}
