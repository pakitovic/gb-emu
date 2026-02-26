use super::Bus;

const REG_KEY1: u16 = 0xFF4D;
const REG_VBK: u16 = 0xFF4F;
const REG_SVBK: u16 = 0xFF70;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum CgbMmioRegister {
    Key1,
    Vbk,
    Svbk,
}

#[derive(Default)]
pub(super) struct CgbMmioState {
    key1_shadow: u8,
    vbk_shadow: u8,
    svbk_shadow: u8,
}

pub(in crate::memory) fn cgb_mmio_register(addr: u16) -> Option<CgbMmioRegister> {
    match addr {
        REG_KEY1 => Some(CgbMmioRegister::Key1),
        REG_VBK => Some(CgbMmioRegister::Vbk),
        REG_SVBK => Some(CgbMmioRegister::Svbk),
        _ => None,
    }
}

impl CgbMmioState {
    fn record_dmg_scaffold_write(&mut self, reg: CgbMmioRegister, value: u8) {
        match reg {
            // Store only future-relevant logical bits while keeping DMG behavior as no-op.
            CgbMmioRegister::Key1 => self.key1_shadow = value & 0x01,
            CgbMmioRegister::Vbk => self.vbk_shadow = value & 0x01,
            CgbMmioRegister::Svbk => self.svbk_shadow = value & 0x07,
        }
    }

    pub(in crate::memory) fn dmg_effective_vram_bank(&self) -> u8 {
        let _future_vbk = self.vbk_shadow;
        0
    }

    pub(in crate::memory) fn dmg_effective_wram_bank_slot(&self) -> u8 {
        let _future_svbk = self.svbk_shadow;
        // CGB semantics map SVBK=0 to bank 1 for the switchable D000-DFFF window.
        // DMG has a fixed second 4 KiB region, so we keep the effective slot pinned to 1.
        1
    }
}

impl Bus {
    // DMG-only scaffolding for future CGB MMIO decode/wiring. Reads stay unmapped-visible
    // (0xFF), writes remain behavioral no-ops, but we keep internal shadow state to make the
    // later CGB integration a local change in this module.
    pub(super) fn read_cgb_mmio_scaffold(&self, addr: u16) -> Option<u8> {
        cgb_mmio_register(addr)?;
        Some(0xFF)
    }

    pub(super) fn write_cgb_mmio_scaffold(&mut self, addr: u16, value: u8) -> bool {
        let Some(reg) = cgb_mmio_register(addr) else {
            return false;
        };
        self.cgb_mmio.record_dmg_scaffold_write(reg, value);
        true
    }

    #[cfg(test)]
    pub(super) fn debug_cgb_mmio_shadows(&self) -> (u8, u8, u8) {
        (
            self.cgb_mmio.key1_shadow,
            self.cgb_mmio.vbk_shadow,
            self.cgb_mmio.svbk_shadow,
        )
    }

    #[cfg(test)]
    pub(super) fn debug_cgb_effective_bank_selection(&self) -> (u8, u8) {
        (
            self.cgb_mmio.dmg_effective_vram_bank(),
            self.cgb_mmio.dmg_effective_wram_bank_slot(),
        )
    }
}
