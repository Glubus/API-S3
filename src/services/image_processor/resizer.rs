use super::decoder::RawImage;
use crate::models::device::DeviceConfig;
use crate::services::errors::ImageProcessingError;
/// Calcule les dimensions cibles pour l'image redimensionnée.
///
/// # Arguments
/// * `w` - La largeur de l'image source.
/// * `h` - La hauteur de l'image source.
/// * `config` - La configuration du périphérique.
///
/// # Returns
/// Un tuple contenant la largeur et la hauteur cibles.
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn compute_target_dimensions(w: u32, h: u32, config: &DeviceConfig) -> (u32, u32) {
    let max_w: f32 = if config.width > 0 { config.width } else { w } as f32;
    let max_h: f32 = if config.height > 0 { config.height } else { h } as f32;

    let scale_w: f32 = max_w / w as f32;
    let scale_h: f32 = max_h / h as f32;
    let scale: f32 = scale_w.min(scale_h).min(1.0);

    ((w as f32 * scale) as u32, (h as f32 * scale) as u32)
}

fn extract_webp_pixels(webp_img: &webp::WebPImage) -> (&[u8], fast_image_resize::PixelType) {
    let pt = if webp_img.is_alpha() {
        fast_image_resize::PixelType::U8x4
    } else {
        fast_image_resize::PixelType::U8x3
    };
    (&**webp_img, pt)
}

fn extract_jpeg_pixels(data: &[u8]) -> (&[u8], fast_image_resize::PixelType) {
    (data, fast_image_resize::PixelType::U8x3)
}

fn extract_dynamic_pixels(
    dyn_img: &image::DynamicImage,
) -> Result<(&[u8], fast_image_resize::PixelType), ImageProcessingError> {
    match dyn_img {
        image::DynamicImage::ImageRgb8(rgb) => {
            Ok((rgb.as_raw().as_slice(), fast_image_resize::PixelType::U8x3))
        }
        image::DynamicImage::ImageRgba8(rgba) => {
            Ok((rgba.as_raw().as_slice(), fast_image_resize::PixelType::U8x4))
        }
        _ => Err(ImageProcessingError::InternalError(
            "Unsupported DynamicImage type".into(),
        )),
    }
}
/// Extrait les pixels d'une image brute.
///
/// # Arguments
/// * `src_image` - L'image source.
/// # Errors
/// * `ImageProcessingError::InternalError` - Si le type d'image n'est pas supporté.
pub fn extract_pixels(
    src_image: &RawImage,
) -> Result<(&[u8], fast_image_resize::PixelType), ImageProcessingError> {
    match src_image {
        RawImage::WebP(webp_img) => Ok(extract_webp_pixels(webp_img)),
        RawImage::Jpeg { data, .. } => Ok(extract_jpeg_pixels(data)),
        RawImage::Fallback(dyn_img) => extract_dynamic_pixels(dyn_img),
    }
}
/// Redimensionne une image aux dimensions cibles.
///
/// # Arguments
/// * `src_image` - L'image source.
/// * `w` - La largeur de l'image source.
/// * `h` - La hauteur de l'image source.
/// * `target_w` - La largeur de l'image cible.
/// * `target_h` - La hauteur de l'image cible.
///
/// # Errors
/// * `ImageProcessingError::InternalError` - Si l'image source ou cible a une dimension de 0.
pub fn resize_image(
    src_image: &RawImage,
    w: u32,
    h: u32,
    target_w: u32,
    target_h: u32,
) -> Result<(Vec<u8>, fast_image_resize::PixelType), ImageProcessingError> {
    if w == 0 || h == 0 || target_w == 0 || target_h == 0 {
        return Err(ImageProcessingError::InternalError(
            "Image dimension is 0".to_string(),
        ));
    }

    let (src_buffer, dst_pixel_type) = extract_pixels(src_image)?;

    let src_img_fast = fast_image_resize::images::ImageRef::new(w, h, src_buffer, dst_pixel_type)
        .map_err(|e| {
        ImageProcessingError::InternalError(format!("Failed to create source image ref: {e:?}"))
    })?;

    let mut dst_img_fast =
        fast_image_resize::images::Image::new(target_w, target_h, dst_pixel_type);

    let mut resizer = fast_image_resize::Resizer::new();
    let options = fast_image_resize::ResizeOptions::new().resize_alg(
        fast_image_resize::ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3),
    );

    resizer
        .resize(&src_img_fast, &mut dst_img_fast, &options)
        .map_err(|e| ImageProcessingError::InternalError(format!("Resize failed: {e:?}")))?;

    Ok((dst_img_fast.into_vec(), dst_pixel_type))
}
