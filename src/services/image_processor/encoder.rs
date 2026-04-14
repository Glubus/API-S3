use crate::services::errors::ImageProcessingError;
use fast_image_resize::PixelType;

/// Encode une image en WebP lossy via libwebp avec optimisation vitesse.
///
/// # Arguments
/// * `data` - Le buffer contenant les pixels bruts
/// * `width` - Largeur de l'image
/// * `height` - Hauteur de l'image
/// * `pixel_type` - Type de pixel (U8x3 pour RGB, U8x4 pour RGBA)
/// * `quality` - La qualité de l'encodage (0-100).
///
/// # Returns
/// * `Ok(Vec<u8>)` - Le buffer WebP encodé.
/// * `Err(ImageProcessingError)` - Si l'encodage échoue.
///
/// # Errors
/// * `ImageProcessingError::EncodeError` - Si l'encodage échoue.
pub fn encode_image(
    data: &[u8],
    width: u32,
    height: u32,
    pixel_type: PixelType,
    quality: u8,
) -> Result<Vec<u8>, ImageProcessingError> {
    let encoder = match pixel_type {
        PixelType::U8x3 => webp::Encoder::from_rgb(data, width, height),
        PixelType::U8x4 => webp::Encoder::from_rgba(data, width, height),
        _ => {
            return Err(ImageProcessingError::EncodeError(
                "Unsupported pixel type for encoding".into(),
            ));
        }
    };

    let mut config = webp::WebPConfig::new()
        .map_err(|()| ImageProcessingError::EncodeError("WebPConfig init failed".to_string()))?;
    config.quality = f32::from(quality);
    config.method = 0; // Fastest mode

    let webp_data = encoder
        .encode_advanced(&config)
        .map_err(|e| ImageProcessingError::EncodeError(format!("WebP encode failed: {e:?}")))?;

    Ok(webp_data.to_vec())
}
