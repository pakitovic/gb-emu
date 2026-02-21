use super::super::Bus;
use super::*;

impl Bus {
    pub(in crate::memory) fn write_apu_register(&mut self, addr: u16, value: u8) {
        let index = (addr - 0xFF00) as usize;
        if (WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX).contains(&index) {
            self.io[index] = value;
            return;
        }
        if !self.apu.enabled {
            return;
        }

        self.io[index] = value;
        let length_clocks_next = self.apu.length_clocks_on_next_frame_step();
        let envelope_clocks_next = self.apu.envelope_clocks_on_next_frame_step();
        match index {
            NR10_INDEX => {
                if self.apu.square1.sweep.write_register(value) {
                    self.apu.square1.enabled = false;
                }
            }
            NR11_INDEX => self.apu.square1.write_duty_length(value),
            NR12_INDEX => self.apu.square1.write_envelope(value),
            NR13_INDEX => self.apu.square1.write_frequency_low(value),
            NR14_INDEX => self.apu.square1.write_frequency_high(
                value,
                length_clocks_next,
                envelope_clocks_next,
            ),
            NR21_INDEX => self.apu.square2.write_duty_length(value),
            NR22_INDEX => self.apu.square2.write_envelope(value),
            NR23_INDEX => self.apu.square2.write_frequency_low(value),
            NR24_INDEX => self.apu.square2.write_frequency_high(
                value,
                length_clocks_next,
                envelope_clocks_next,
            ),
            NR30_INDEX => self.apu.wave.write_dac_enable(value),
            NR31_INDEX => self.apu.wave.write_length(value),
            NR32_INDEX => self.apu.wave.write_output_level(value),
            NR33_INDEX => self.apu.wave.write_frequency_low(value),
            NR34_INDEX => self
                .apu
                .wave
                .write_frequency_high(value, length_clocks_next),
            NR41_INDEX => self.apu.noise.write_length(value),
            NR42_INDEX => self.apu.noise.write_envelope(value),
            NR43_INDEX => self.apu.noise.write_polynomial(value),
            NR44_INDEX => {
                self.apu
                    .noise
                    .write_control(value, length_clocks_next, envelope_clocks_next)
            }
            _ => {}
        }
        self.apu.refresh_channel_on_mask();
    }

    pub(in crate::memory) fn clear_apu_registers(&mut self) {
        for index in NR10_INDEX..=NR51_INDEX {
            self.io[index] = 0x00;
        }
    }
}
