use crate::models::device::DeviceConfig;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Catégories de ressources disponibles
#[derive(Debug, Clone, Copy)]
pub enum ResourceCategory {
    Icons,
    ImagesTags,
    ImagesTemporaires,
    Logos,
    Utils,
    Images,
}

impl ResourceCategory {
    /// Retourne le préfixe S3 pour cette catégorie
    #[must_use]
    pub fn s3_prefix(&self) -> &str {
        match self {
            ResourceCategory::Icons => "icons",
            ResourceCategory::ImagesTags => "images_tags",
            ResourceCategory::ImagesTemporaires => "images_temporaires",
            ResourceCategory::Logos => "logos",
            ResourceCategory::Utils => "utils",
            ResourceCategory::Images => "data/images",
        }
    }
}

impl FromStr for ResourceCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "icons" => Ok(ResourceCategory::Icons),
            "images_tags" => Ok(ResourceCategory::ImagesTags),
            "images_temporaires" => Ok(ResourceCategory::ImagesTemporaires),
            "logos" => Ok(ResourceCategory::Logos),
            "utils" => Ok(ResourceCategory::Utils),
            "images" => Ok(ResourceCategory::Images),
            _ => Err(format!("Unknown resource category: {s}")),
        }
    }
}

/// Item pour une requête batch
#[derive(Debug, Deserialize, Serialize, Clone, utoipa::ToSchema)]
pub struct BatchResourceItem {
    pub category: String,
    pub path: String,
}

/// Requête pour récupérer un batch de ressources
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct BatchResourceRequest {
    pub config: DeviceConfig,
    pub items: Vec<BatchResourceItem>,
}
