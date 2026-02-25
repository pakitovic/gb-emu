pub const DMG_T_CYCLES_PER_SECOND: u64 = 4_194_304;
pub const DMG_T_CYCLES_PER_FRAME: u64 = 70_224;
pub const DMG_CPU_T_CYCLES_PER_M_CYCLE: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockRatios {
    cpu_t_cycles_per_m_cycle: u8,
}

impl ClockRatios {
    pub const fn dmg() -> Self {
        Self {
            cpu_t_cycles_per_m_cycle: DMG_CPU_T_CYCLES_PER_M_CYCLE,
        }
    }

    pub const fn cpu_t_cycles_per_m_cycle(self) -> u8 {
        self.cpu_t_cycles_per_m_cycle
    }

    pub const fn cpu_t_cycles_for_m_cycles(self, mcycles: u8) -> u8 {
        mcycles.saturating_mul(self.cpu_t_cycles_per_m_cycle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmg_clock_ratios_use_four_tcycles_per_mcycle() {
        let ratios = ClockRatios::dmg();
        assert_eq!(ratios.cpu_t_cycles_per_m_cycle(), 4);
        assert_eq!(ratios.cpu_t_cycles_for_m_cycles(1), 4);
        assert_eq!(ratios.cpu_t_cycles_for_m_cycles(2), 8);
        assert_eq!(ratios.cpu_t_cycles_for_m_cycles(5), 20);
    }
}
