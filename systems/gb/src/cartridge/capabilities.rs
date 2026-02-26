use super::header::{CartridgeCgbSupport, CartridgeSgbSupport, parse_header_mode_flags};
use super::{Cartridge, CartridgeMapper, public_mapper};
use crate::hardware::HardwareModel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CartridgeModelModeRequest {
    pub cgb_support: CartridgeCgbSupport,
}

impl CartridgeModelModeRequest {
    #[inline]
    pub fn prefers_cgb(self) -> bool {
        matches!(
            self.cgb_support,
            CartridgeCgbSupport::Supported | CartridgeCgbSupport::Required
        )
    }

    #[inline]
    pub fn requires_cgb(self) -> bool {
        matches!(self.cgb_support, CartridgeCgbSupport::Required)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CartridgeModelCompatibility {
    pub model: HardwareModel,
    pub mode_request: CartridgeModelModeRequest,
    pub dmg_mode_allowed: bool,
    pub cgb_mode_supported_by_model: bool,
    pub cgb_mode_possible: bool,
    pub cgb_only_header_on_non_cgb_model: bool,
    pub sgb_support: CartridgeSgbSupport,
    pub sgb_features_requested: bool,
    pub sgb_features_supported_by_model: bool,
    pub sgb_features_possible: bool,
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
    pub(crate) sgb_header_flag_raw: u8,
    pub(crate) sgb_support: CartridgeSgbSupport,
    pub(crate) supports_sgb: bool,
}

impl CartridgeCapabilities {
    pub fn compatibility_for_model(self, model: HardwareModel) -> CartridgeModelCompatibility {
        let mode_request = CartridgeModelModeRequest {
            cgb_support: self.cgb_support,
        };
        let cgb_mode_supported_by_model = model.supports_cgb_mode();
        let cgb_mode_possible = mode_request.prefers_cgb() && cgb_mode_supported_by_model;
        let dmg_mode_allowed = !self.cgb_only;
        let sgb_features_requested = self.supports_sgb;
        let sgb_features_supported_by_model =
            sgb_features_requested && model.supports_sgb_features();

        CartridgeModelCompatibility {
            model,
            mode_request,
            dmg_mode_allowed,
            cgb_mode_supported_by_model,
            cgb_mode_possible,
            cgb_only_header_on_non_cgb_model: self.cgb_only && !cgb_mode_supported_by_model,
            sgb_support: self.sgb_support,
            sgb_features_requested,
            sgb_features_supported_by_model,
            sgb_features_possible: sgb_features_supported_by_model,
        }
    }
}

impl Cartridge {
    pub(crate) fn capabilities(&self) -> CartridgeCapabilities {
        let header_flags = parse_header_mode_flags(&self.rom);

        CartridgeCapabilities {
            mapper: public_mapper(self.mapper),
            has_declared_ram: self.declared_ram_size_bytes > 0,
            has_effective_ram: !self.ram.is_empty(),
            compatibility_ram_mode: self.compatibility_ram_mode,
            has_battery: self.has_battery,
            has_timer: self.has_timer,
            has_rumble: self.has_rumble,
            has_battery_save: self.has_battery_save(),
            cgb_header_flag_raw: header_flags.cgb_header_flag_raw,
            cgb_support: header_flags.cgb_support,
            supports_cgb: header_flags.supports_cgb,
            cgb_only: header_flags.cgb_only,
            sgb_header_flag_raw: header_flags.sgb_header_flag_raw,
            sgb_support: header_flags.sgb_support,
            supports_sgb: header_flags.supports_sgb,
        }
    }

    pub(crate) fn compatibility_for_model(
        &self,
        model: HardwareModel,
    ) -> CartridgeModelCompatibility {
        self.capabilities().compatibility_for_model(model)
    }
}
