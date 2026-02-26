use super::super::Bus;

#[derive(Default)]
pub(in crate::memory) struct SerialState {
    pub(in crate::memory) output: String,
    pub(super) bits_remaining: u8,
    pub(super) tx_byte: u8,
}

impl SerialState {
    pub(in crate::memory) fn write_control(bus: &mut Bus, value: u8) {
        // On DMG/MGB/SGB, only bit7 (transfer start) and bit0 (clock source)
        // are writable/meaningful.
        bus.io[0x02] = value & 0x81;

        if (value & 0x81) == 0x81 {
            // Start/restart internal-clock transfer.
            bus.serial.bits_remaining = 8;
            bus.serial.tx_byte = bus.io[0x01];
        } else if (value & 0x80) == 0 {
            // Explicit stop.
            bus.serial.bits_remaining = 0;
        }
    }

    pub(in crate::memory) fn step(bus: &mut Bus, old_div: u16, new_div: u16) {
        if bus.serial.bits_remaining == 0 {
            return;
        }

        // Internal clock transfer only (SC bit0 = 1).
        if (bus.io[0x02] & 0x81) != 0x81 {
            return;
        }

        // Internal clock is phase-aligned to DIV bit 8 falling edge.
        let old_clock_high = ((old_div >> 8) & 1) != 0;
        let new_clock_high = ((new_div >> 8) & 1) != 0;
        if !old_clock_high || new_clock_high {
            return;
        }

        // Shift one bit; receive line is high (no external device attached).
        bus.io[0x01] = (bus.io[0x01] << 1) | 0x01;
        bus.serial.bits_remaining -= 1;
        if bus.serial.bits_remaining != 0 {
            return;
        }

        // Transfer finished.
        bus.io[0x02] &= !0x80;
        let iflags = bus.interrupt_flags() | (1 << 3);
        bus.set_interrupt_flags(iflags);

        // Keep Blargg/Mooneye serial output compatible.
        let ch = bus.serial.tx_byte as char;
        bus.serial.output.push(ch);
    }
}

impl Bus {
    pub(in crate::memory) fn write_sc(&mut self, value: u8) {
        SerialState::write_control(self, value);
    }

    pub(in crate::memory) fn step_serial(&mut self, old_div: u16, new_div: u16) {
        SerialState::step(self, old_div, new_div);
    }
}
