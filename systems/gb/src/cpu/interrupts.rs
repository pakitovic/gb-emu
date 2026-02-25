use super::Cpu;
use crate::cpu::CpuContext;

fn select_interrupt(pending: u8) -> Option<(u8, u16)> {
    if (pending & 0x01) != 0 {
        Some((0, 0x0040))
    } else if (pending & 0x02) != 0 {
        Some((1, 0x0048))
    } else if (pending & 0x04) != 0 {
        Some((2, 0x0050))
    } else if (pending & 0x08) != 0 {
        Some((3, 0x0058))
    } else if (pending & 0x10) != 0 {
        Some((4, 0x0060))
    } else {
        None
    }
}

impl Cpu {
    pub(super) fn service_interrupt(&mut self, bus: &mut impl CpuContext, _pending: u8) -> u8 {
        self.ime = false;
        self.halted = false;
        // A pending HALT-bug latch affects the next opcode fetch path. Interrupt
        // service performs no opcode fetch and should not carry that latch into
        // post-interrupt execution.
        self.halt_bug = false;

        // Interrupt dispatch takes 5 M-cycles on DMG:
        // 2 idle cycles, then PC high/low push, then vector/jump cycle.
        let pc = self.registers.pc;
        let pc_high = (pc >> 8) as u8;
        let pc_low = (pc & 0x00FF) as u8;

        self.tick_m(bus, 2);

        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, pc_high);

        // IE may have been changed by the upper-byte push (SP hitting $FFFF).
        // Re-evaluate pending interrupts at this point.
        let selected = select_interrupt(bus.pending_interrupts());

        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.write_byte(bus, self.registers.sp, pc_low);

        if let Some((bit, vector)) = selected {
            let mut iflags = bus.interrupt_flags();
            iflags &= !(1 << bit);
            bus.set_interrupt_flags(iflags);
            self.registers.pc = vector;
        } else {
            // If IE push cancels the dispatch, execution continues from $0000.
            self.registers.pc = 0x0000;
        }

        self.tick_m(bus, 1);
        self.ret_step_total_m(bus, Self::INTERRUPT_SERVICE_M_CYCLES)
    }
}
