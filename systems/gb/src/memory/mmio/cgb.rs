use super::super::Bus;

const REG_KEY1: u16 = 0xFF4D;
const REG_VBK: u16 = 0xFF4F;
const REG_BGPI: u16 = 0xFF68;
const REG_BGPD: u16 = 0xFF69;
const REG_OBPI: u16 = 0xFF6A;
const REG_OBPD: u16 = 0xFF6B;
const REG_SVBK: u16 = 0xFF70;
const CGB_PALETTE_INDEX_MASK: u8 = 0x3F;
const CGB_PALETTE_AUTOINC_MASK: u8 = 0x80;
const CGB_PALETTE_INDEX_REG_MASK: u8 = CGB_PALETTE_AUTOINC_MASK | CGB_PALETTE_INDEX_MASK;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::memory) enum CgbMmioRegister {
    Key1,
    Vbk,
    Bgpi,
    Bgpd,
    Obpi,
    Obpd,
    Svbk,
}

pub(in crate::memory) struct CgbMmioState {
    key1_shadow: u8,
    vbk_shadow: u8,
    bgpi_shadow: u8,
    bg_palette_shadow: [u8; 0x40],
    obpi_shadow: u8,
    obj_palette_shadow: [u8; 0x40],
    svbk_shadow: u8,
}

impl Default for CgbMmioState {
    fn default() -> Self {
        Self {
            key1_shadow: 0,
            vbk_shadow: 0,
            bgpi_shadow: 0,
            bg_palette_shadow: [0; 0x40],
            obpi_shadow: 0,
            obj_palette_shadow: [0; 0x40],
            svbk_shadow: 0,
        }
    }
}

pub(in crate::memory) fn cgb_mmio_register(addr: u16) -> Option<CgbMmioRegister> {
    match addr {
        REG_KEY1 => Some(CgbMmioRegister::Key1),
        REG_VBK => Some(CgbMmioRegister::Vbk),
        REG_BGPI => Some(CgbMmioRegister::Bgpi),
        REG_BGPD => Some(CgbMmioRegister::Bgpd),
        REG_OBPI => Some(CgbMmioRegister::Obpi),
        REG_OBPD => Some(CgbMmioRegister::Obpd),
        REG_SVBK => Some(CgbMmioRegister::Svbk),
        _ => None,
    }
}

impl CgbMmioState {
    #[inline]
    fn write_palette_data(index_reg: &mut u8, data: &mut [u8; 0x40], value: u8) {
        let index = (*index_reg & CGB_PALETTE_INDEX_MASK) as usize;
        data[index] = value;

        if (*index_reg & CGB_PALETTE_AUTOINC_MASK) != 0 {
            let next_index = index_reg.wrapping_add(1) & CGB_PALETTE_INDEX_MASK;
            *index_reg = (*index_reg & CGB_PALETTE_AUTOINC_MASK) | next_index;
        }
    }

    fn record_dmg_scaffold_write(&mut self, reg: CgbMmioRegister, value: u8) {
        match reg {
            // Store only future-relevant logical bits while keeping DMG behavior as no-op.
            CgbMmioRegister::Key1 => self.key1_shadow = value & 0x01,
            CgbMmioRegister::Vbk => self.vbk_shadow = value & 0x01,
            CgbMmioRegister::Bgpi => self.bgpi_shadow = value & CGB_PALETTE_INDEX_REG_MASK,
            CgbMmioRegister::Bgpd => {
                Self::write_palette_data(&mut self.bgpi_shadow, &mut self.bg_palette_shadow, value);
            }
            CgbMmioRegister::Obpi => self.obpi_shadow = value & CGB_PALETTE_INDEX_REG_MASK,
            CgbMmioRegister::Obpd => {
                Self::write_palette_data(
                    &mut self.obpi_shadow,
                    &mut self.obj_palette_shadow,
                    value,
                );
            }
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
    pub(in crate::memory) fn debug_cgb_mmio_shadows(&self) -> (u8, u8, u8) {
        (
            self.cgb_mmio.key1_shadow,
            self.cgb_mmio.vbk_shadow,
            self.cgb_mmio.svbk_shadow,
        )
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_cgb_palette_index_shadows(&self) -> (u8, u8) {
        (self.cgb_mmio.bgpi_shadow, self.cgb_mmio.obpi_shadow)
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_cgb_palette_shadow_byte(&self, is_obj: bool, index: u8) -> u8 {
        let index = (index & CGB_PALETTE_INDEX_MASK) as usize;
        if is_obj {
            self.cgb_mmio.obj_palette_shadow[index]
        } else {
            self.cgb_mmio.bg_palette_shadow[index]
        }
    }

    #[cfg(test)]
    pub(in crate::memory) fn debug_cgb_effective_bank_selection(&self) -> (u8, u8) {
        (
            self.cgb_mmio.dmg_effective_vram_bank(),
            self.cgb_mmio.dmg_effective_wram_bank_slot(),
        )
    }
}
