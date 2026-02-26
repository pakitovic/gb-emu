use super::{Cartridge, CartridgeMapper, public_mapper};

const HEADER_CGB_FLAG_OFFSET: usize = 0x0143;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CartridgeCgbSupport {
    None,
    Supported,
    Required,
}

impl CartridgeCgbSupport {
    #[inline]
    fn from_header_flag_raw(flag: u8) -> Self {
        match flag {
            0x80 => Self::Supported,
            0xC0 => Self::Required,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CartridgeCapabilities {
    pub(crate) mapper: CartridgeMapper,
    pub(crate) has_declared_ram: bool,
    pub(crate) has_effective_ram: bool,
    pub(crate) compatibility_ram_mode: bool,
    pub(crate) has_battery: bool,
    pub(crate) has_timer: bool,
    pub(crate) has_rumble: bool,
    pub(crate) has_battery_save: bool,
    pub(crate) cgb_header_flag_raw: u8,
    pub(crate) cgb_support: CartridgeCgbSupport,
    pub(crate) supports_cgb: bool,
    pub(crate) cgb_only: bool,
}

impl Cartridge {
    pub(crate) fn capabilities(&self) -> CartridgeCapabilities {
        let cgb_header_flag_raw = self
            .rom
            .get(HEADER_CGB_FLAG_OFFSET)
            .copied()
            .unwrap_or(0x00);

        let cgb_support = CartridgeCgbSupport::from_header_flag_raw(cgb_header_flag_raw);

        CartridgeCapabilities {
            mapper: public_mapper(self.mapper),
            has_declared_ram: self.declared_ram_size_bytes > 0,
            has_effective_ram: !self.ram.is_empty(),
            compatibility_ram_mode: self.compatibility_ram_mode,
            has_battery: self.has_battery,
            has_timer: self.has_timer,
            has_rumble: self.has_rumble,
            has_battery_save: self.has_battery_save(),
            cgb_header_flag_raw,
            cgb_support,
            supports_cgb: matches!(
                cgb_support,
                CartridgeCgbSupport::Supported | CartridgeCgbSupport::Required
            ),
            cgb_only: matches!(cgb_support, CartridgeCgbSupport::Required),
        }
    }
}
