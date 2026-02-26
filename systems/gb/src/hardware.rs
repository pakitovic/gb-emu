use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HardwareModel {
    Dmg0,
    #[default]
    Dmg,
    Mgb,
    Sgb,
    Sgb2,
}

impl HardwareModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dmg0 => "dmg0",
            Self::Dmg => "dmg",
            Self::Mgb => "mgb",
            Self::Sgb => "sgb",
            Self::Sgb2 => "sgb2",
        }
    }

    #[inline]
    pub fn supports_cgb_mode(self) -> bool {
        false
    }

    #[inline]
    pub fn supports_sgb_features(self) -> bool {
        matches!(self, Self::Sgb | Self::Sgb2)
    }
}

impl Display for HardwareModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for HardwareModel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "dmg0" => Ok(Self::Dmg0),
            "dmg" | "dmgabc" => Ok(Self::Dmg),
            "mgb" => Ok(Self::Mgb),
            "sgb" => Ok(Self::Sgb),
            "sgb2" => Ok(Self::Sgb2),
            _ => Err(format!(
                "Unsupported model '{value}'. Supported models: dmg0, dmg, mgb, sgb, sgb2"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_models_and_aliases() {
        assert_eq!(
            "dmg0".parse::<HardwareModel>().ok(),
            Some(HardwareModel::Dmg0)
        );
        assert_eq!(
            "dmg".parse::<HardwareModel>().ok(),
            Some(HardwareModel::Dmg)
        );
        assert_eq!(
            "dmgabc".parse::<HardwareModel>().ok(),
            Some(HardwareModel::Dmg)
        );
        assert_eq!(
            "mgb".parse::<HardwareModel>().ok(),
            Some(HardwareModel::Mgb)
        );
        assert_eq!(
            "sgb".parse::<HardwareModel>().ok(),
            Some(HardwareModel::Sgb)
        );
        assert_eq!(
            "sgb2".parse::<HardwareModel>().ok(),
            Some(HardwareModel::Sgb2)
        );
    }

    #[test]
    fn rejects_unknown_model() {
        assert!("cgb".parse::<HardwareModel>().is_err());
    }

    #[test]
    fn reports_current_family_capability_gates() {
        for model in [
            HardwareModel::Dmg0,
            HardwareModel::Dmg,
            HardwareModel::Mgb,
            HardwareModel::Sgb,
            HardwareModel::Sgb2,
        ] {
            assert!(
                !model.supports_cgb_mode(),
                "current model set should remain DMG-family only for CGB-ready scaffolding"
            );
        }
        assert!(!HardwareModel::Dmg.supports_sgb_features());
        assert!(HardwareModel::Sgb.supports_sgb_features());
        assert!(HardwareModel::Sgb2.supports_sgb_features());
    }
}
