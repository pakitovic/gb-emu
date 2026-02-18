use super::Bus;
use std::io::{self, Write};

impl Bus {
    pub(super) fn write_sc(&mut self, value: u8) {
        // On DMG/MGB/SGB, only bit7 (transfer start) and bit0 (clock source)
        // are writable/meaningful.
        self.io[0x02] = value & 0x81;

        if (value & 0x81) == 0x81 {
            // Start/restart internal-clock transfer.
            self.serial_bits_remaining = 8;
            self.serial_tx_byte = self.io[0x01];
        } else if (value & 0x80) == 0 {
            // Explicit stop.
            self.serial_bits_remaining = 0;
        }
    }

    pub(super) fn step_serial(&mut self, old_div: u16, new_div: u16) {
        if self.serial_bits_remaining == 0 {
            return;
        }

        // Internal clock transfer only (SC bit0 = 1).
        if (self.io[0x02] & 0x81) != 0x81 {
            return;
        }

        // Internal clock is phase-aligned to DIV bit 8 falling edge.
        let old_clock_high = ((old_div >> 8) & 1) != 0;
        let new_clock_high = ((new_div >> 8) & 1) != 0;
        if !old_clock_high || new_clock_high {
            return;
        }

        // Shift one bit; receive line is high (no external device attached).
        self.io[0x01] = (self.io[0x01] << 1) | 0x01;
        self.serial_bits_remaining -= 1;
        if self.serial_bits_remaining != 0 {
            return;
        }

        // Transfer finished.
        self.io[0x02] &= !0x80;
        let iflags = self.interrupt_flags() | (1 << 3);
        self.set_interrupt_flags(iflags);

        // Keep Blargg serial output compatible.
        let ch = self.serial_tx_byte as char;
        self.serial_output.push(ch);
        print!("{ch}");
        let _ = io::stdout().flush();
    }
}
