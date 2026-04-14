use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default, utoipa::ToSchema)]
pub enum DeviceTarget {
    Phone,
    Tablet,
    #[default]
    Desktop,
}

impl DeviceTarget {
    #[must_use]
    pub fn default_quality(&self) -> u8 {
        match self {
            Self::Phone => 75,
            Self::Tablet => 80,
            Self::Desktop => 85,
        }
    }
}

impl std::str::FromStr for DeviceTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "phone" => Ok(Self::Phone),
            "tablet" => Ok(Self::Tablet),
            "desktop" => Ok(Self::Desktop),
            _ => Err(format!("Invalid device target: {s}")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct DeviceConfig {
    pub target: DeviceTarget,
    pub width: u32,
    pub height: u32,
    pub scale: Option<f32>,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            target: DeviceTarget::Desktop,
            width: 1920,
            height: 1080,
            scale: None,
        }
    }
}
