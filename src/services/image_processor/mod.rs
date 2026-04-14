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
/// Orchestre la récupération, le traitement et la mise en cache d'une image.
///
/// # Pourquoi cette fonction ?
/// Centralise la logique complexe pour éviter la duplication entre les routes
/// image unique et bulk. Permet également d'assurer une gestion cohérente du cache.
///
/// # Arguments
/// * `name` - Le nom de l'image.
/// * `config` - La configuration du périphérique.
///
/// # Errors
/// * `ImageProcessingError::NotFound` - Si l'image n'est pas trouvée.
/// * `ImageProcessingError::InternalError` - Si une erreur interne se produit.
///
/// # Returns
/// * `Ok(Vec<u8>)` - Le buffer de bytes contenant l'image traitée.
/// * `Err(ImageProcessingError)` - L'erreur de traitement.
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
    tracing::debug!(cache.hit = false, "Cache miss — fetching from S3");

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
/// Traite une image en la redimensionnant selon la configuration du périphérique.
///
/// # Arguments
/// * `image_data` - Le buffer de bytes contenant l'image.
/// * `config` - La configuration du périphérique.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - Si l'image ne peut pas être décodée.
/// * `ImageProcessingError::InternalError` - Si une erreur interne se produit.
///
/// # Returns
/// * `Ok(Vec<u8>)` - Le buffer de bytes contenant l'image traitée.
/// * `Err(ImageProcessingError)` - L'erreur de traitement.
pub async fn process_image(
    image_data: Vec<u8>,
    config: DeviceConfig,
) -> Result<Vec<u8>, ImageProcessingError> {
    let quality = config.target.default_quality();
    process_image_with_quality(image_data, config, quality).await
}
/// Extrait les dimensions d'une image.
///
/// # Arguments
/// * `data` - Les données de l'image.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - Si l'image ne peut pas être décodée.
pub fn extract_dimensions(data: &[u8]) -> Result<(u32, u32), ImageProcessingError> {
    let reader = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| ImageProcessingError::DecodeError(e.to_string()))?;
    reader
        .into_dimensions()
        .map_err(|e| ImageProcessingError::DecodeError(e.to_string()))
}

use image::{ImageBuffer, Pixel};

// Fonction générique qui gère n'importe quel type de pixel basé sur des u8
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
    // unsharpen alloue un nouveau buffer de toute façon pour le résultat
    let sharpened = image::imageops::unsharpen(&img_buf, 0.5, 1);
    Ok(sharpened.into_raw())
}

// Ta fonction principale devient super propre :
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
/// Traite une image en la redimensionnant selon la configuration du périphérique.
///
/// # Arguments
/// * `image_data` - Le buffer de bytes contenant l'image.
/// * `config` - La configuration du périphérique.
/// * `quality` - La qualité de l'image.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - Si l'image ne peut pas être décodée.
/// * `ImageProcessingError::InternalError` - Si une erreur interne se produit.
///
/// # Returns
/// * `Ok(Vec<u8>)` - Le buffer de bytes contenant l'image traitée.
/// * `Err(ImageProcessingError)` - L'erreur de traitement.
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
