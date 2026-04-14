pub mod decoder;
pub mod encoder;
pub mod resizer;

use crate::models::device::DeviceConfig;
use crate::services::cache::get_global_cache;
use crate::services::errors::ImageProcessingError;
use crate::services::storage::StorageService;
use std::io::Cursor;

pub use decoder::{RawImage, decode_image};
pub use encoder::encode_image;
pub use resizer::{compute_target_dimensions, resize_image};

/// Orchestrates image fetching, processing, and caching.
///
/// # Why this function?
/// Centralizes the complex pipeline logic to avoid duplication between
/// the single-image and bulk routes, and ensures consistent cache handling.
///
/// # Arguments
/// * `name` - The image key (S3 path).
/// * `config` - Device configuration (target, width, height).
///
/// # Errors
/// * `ImageProcessingError::NotFound` - If the image does not exist in storage.
/// * `ImageProcessingError::InternalError` - If an internal error occurs.
///
/// # Returns
/// * `Ok(Vec<u8>)` - Byte buffer containing the processed image.
/// * `Err(ImageProcessingError)` - Processing error.
#[tracing::instrument(fields(image.key = %name, image.width = config.width, image.height = config.height))]
pub async fn get_or_process_cached(
    name: String,
    config: DeviceConfig,
) -> Result<Vec<u8>, ImageProcessingError> {
    let cache = get_global_cache();

    if let Some(cached) = cache.get(&name, config.width, config.height) {
        tracing::debug!(cache.hit = true, "Cache hit");
        return Ok(cached);
    }
    tracing::debug!(cache.hit = false, "Cache miss, fetching from S3");

    let image_data = match StorageService::get_image(&name).await {
        Ok(data) => data,
        Err(crate::services::errors::StorageError::NotFound(e)) => {
            return Err(ImageProcessingError::NotFound(e));
        }
        Err(e) => {
            return Err(ImageProcessingError::InternalError(format!(
                "Storage error: {e}"
            )));
        }
    };

    let processed_data = process_image(image_data, config.clone()).await?;

    cache.set(&name, config.width, config.height, processed_data.clone());

    Ok(processed_data)
}

/// Processes an image by resizing it according to the device configuration.
///
/// Delegates to `process_image_with_quality` using the device's default quality.
///
/// # Arguments
/// * `image_data` - Raw image bytes.
/// * `config` - Device configuration.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - If the image cannot be decoded.
/// * `ImageProcessingError::InternalError` - If an internal error occurs.
///
/// # Returns
/// * `Ok(Vec<u8>)` - Processed image bytes.
/// * `Err(ImageProcessingError)` - Processing error.
pub async fn process_image(
    image_data: Vec<u8>,
    config: DeviceConfig,
) -> Result<Vec<u8>, ImageProcessingError> {
    let quality = config.target.default_quality();
    process_image_with_quality(image_data, config, quality).await
}

/// Extracts the dimensions of an image without full decoding.
///
/// # Arguments
/// * `data` - Raw image bytes.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - If the image cannot be read.
pub fn extract_dimensions(data: &[u8]) -> Result<(u32, u32), ImageProcessingError> {
    let reader = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| ImageProcessingError::DecodeError(e.to_string()))?;
    reader
        .into_dimensions()
        .map_err(|e| ImageProcessingError::DecodeError(e.to_string()))
}

use image::{ImageBuffer, Pixel};

// Generic helper that handles any u8-based pixel type
fn process_unsharpen<P>(
    w: u32,
    h: u32,
    buffer: Vec<u8>,
    err_msg: &str,
) -> Result<Vec<u8>, ImageProcessingError>
where
    P: Pixel<Subpixel = u8> + 'static,
{
    let img_buf = ImageBuffer::<P, _>::from_raw(w, h, buffer)
        .ok_or_else(|| ImageProcessingError::InternalError(err_msg.into()))?;
    // unsharpen always allocates a new buffer for the result
    let sharpened = image::imageops::unsharpen(&img_buf, 0.5, 1);
    Ok(sharpened.into_raw())
}

fn apply_sharpening(
    buffer: Vec<u8>,
    w: u32,
    h: u32,
    pt: fast_image_resize::PixelType,
) -> Result<Vec<u8>, ImageProcessingError> {
    if pt == fast_image_resize::PixelType::U8x3 {
        process_unsharpen::<image::Rgb<u8>>(w, h, buffer, "Failed to create RGB dest buffer")
    } else {
        process_unsharpen::<image::Rgba<u8>>(w, h, buffer, "Failed to create RGBA dest buffer")
    }
}

/// Processes an image: decode → resize → sharpen → encode WebP.
///
/// Runs inside `spawn_blocking` to avoid blocking the async runtime.
///
/// # Arguments
/// * `image_data` - Raw image bytes.
/// * `config` - Device configuration.
/// * `quality` - WebP encoding quality (0–100).
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - If the image cannot be decoded.
/// * `ImageProcessingError::InternalError` - If an internal error occurs.
///
/// # Returns
/// * `Ok(Vec<u8>)` - Processed image bytes.
/// * `Err(ImageProcessingError)` - Processing error.
pub async fn process_image_with_quality(
    image_data: Vec<u8>,
    config: DeviceConfig,
    quality: u8,
) -> Result<Vec<u8>, ImageProcessingError> {
    let (width, height) = extract_dimensions(&image_data)?;
    let (target_w, target_h) = compute_target_dimensions(width, height, &config);

    if target_w == width && target_h == height {
        tracing::debug!("Fast Bypass: image is already at target dimensions or smaller.");
        return Ok(image_data);
    }

    tokio::task::spawn_blocking(move || {
        let src_image = decode_image(&image_data)?;
        let (dst_buffer, pixel_type) = resize_image(&src_image, width, height, target_w, target_h)?;
        let sharpened_buffer = apply_sharpening(dst_buffer, target_w, target_h, pixel_type)?;
        encode_image(&sharpened_buffer, target_w, target_h, pixel_type, quality)
    })
    .await
    .map_err(|e| ImageProcessingError::InternalError(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::device::DeviceTarget;
    use image::GenericImageView;

    #[tokio::test]
    async fn test_process_image_keeps_size() {
        let img = image::ImageBuffer::from_fn(40, 40, |_, _| image::Rgb([255u8, 0, 0]));
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();

        let config = DeviceConfig {
            target: DeviceTarget::Phone,
            width: 50,
            height: 50,
            scale: None,
        };
        let result = process_image_with_quality(bytes, config, 80).await.unwrap();
        let result_img = image::load_from_memory(&result).unwrap();
        assert_eq!(result_img.dimensions(), (40, 40));
    }
}
