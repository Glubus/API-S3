use crate::services::errors::ImageProcessingError;
use fast_image_resize::PixelType;

/// Encodes raw pixel data to lossy WebP via libwebp with speed optimization.
///
/// # Arguments
/// * `data` - Raw pixel buffer.
/// * `width` - Image width in pixels.
/// * `height` - Image height in pixels.
/// * `pixel_type` - Pixel format (`U8x3` for RGB, `U8x4` for RGBA).
/// * `quality` - WebP encoding quality (0–100).
///
/// # Returns
/// * `Ok(Vec<u8>)` - Encoded WebP bytes.
/// * `Err(ImageProcessingError)` - If encoding fails.
///
/// # Errors
/// * `ImageProcessingError::EncodeError` - If encoding fails.
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
