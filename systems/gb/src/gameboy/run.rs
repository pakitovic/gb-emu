use super::GameBoy;
use crate::timing::DMG_T_CYCLES_PER_FRAME;

impl GameBoy {
    fn push_recent_pc(&mut self, pc: u16) {
        self.recent_pc_trace[self.recent_pc_trace_head] = pc;
        self.recent_pc_trace_head = (self.recent_pc_trace_head + 1) % self.recent_pc_trace.len();
        if self.recent_pc_trace_len < self.recent_pc_trace.len() {
            self.recent_pc_trace_len += 1;
        }
    }

    // Execute one CPU instruction/dispatch step and return DMG base t-cycles consumed.
    pub fn step(&mut self) -> u8 {
        self.push_recent_pc(self.cpu.registers().pc);
        self.cpu.step(&mut self.bus)
    }

    pub fn run_frame_with_limit(&mut self, max_steps: usize) -> Option<u64> {
        let start_frame = self.frame_counter();
        let mut total_tcycles = 0u64;
        let mut lcd_off_tcycles = 0u64;
        for _ in 0..max_steps {
            let tcycles = self.step();
            total_tcycles = total_tcycles.wrapping_add(tcycles as u64);
            if self.frame_counter() != start_frame {
                return Some(total_tcycles);
            }
            if (self.bus.read_byte(0xFF40) & 0x80) == 0 {
                lcd_off_tcycles = lcd_off_tcycles.wrapping_add(tcycles as u64);
                if lcd_off_tcycles >= DMG_T_CYCLES_PER_FRAME {
                    return Some(total_tcycles);
                }
            } else {
                lcd_off_tcycles = 0;
            }
        }
        None
    }
}
