use super::GameBoy;

impl GameBoy {
    // Execute one CPU instruction/dispatch step and return DMG base t-cycles consumed.
    pub fn step(&mut self) -> u8 {
        self.cpu.step(&mut self.bus)
    }

    pub fn run_frame_with_limit(&mut self, max_steps: usize) -> Option<u64> {
        let start_frame = self.frame_counter();
        let mut total_tcycles = 0u64;
        for _ in 0..max_steps {
            let tcycles = self.step();
            total_tcycles = total_tcycles.wrapping_add(tcycles as u64);
            if self.frame_counter() != start_frame {
                return Some(total_tcycles);
            }
        }
        None
    }
}
