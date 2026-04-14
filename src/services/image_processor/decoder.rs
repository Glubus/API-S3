use crate::services::errors::ImageProcessingError;
use zune_core::bytestream::ZCursor;

pub enum RawImage {
    WebP(webp::WebPImage),
    Jpeg {
        width: u32,
        height: u32,
        data: Vec<u8>,
    },
    Fallback(image::DynamicImage),
}

impl RawImage {
    #[must_use]
    pub fn width(&self) -> u32 {
        match self {
            RawImage::WebP(webp_img) => webp_img.width(),
            RawImage::Jpeg { width, .. } => *width,
            RawImage::Fallback(dyn_img) => dyn_img.width(),
        }
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        match self {
            RawImage::WebP(webp_img) => webp_img.height(),
            RawImage::Jpeg { height, .. } => *height,
            RawImage::Fallback(dyn_img) => dyn_img.height(),
        }
    }
}
/// Décode une image depuis un buffer de bytes.
///
/// # Arguments
/// * `data` - Le buffer de bytes contenant l'image.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - Si l'image ne peut pas être décodée.
/// * `ImageProcessingError::InternalError` - Si une erreur interne se produit.
///
/// # Returns
/// * `Ok(RawImage)` - L'image décodée brute.
/// * `Err(ImageProcessingError)` - L'erreur de décodage.
pub fn decode_image(data: &[u8]) -> Result<RawImage, ImageProcessingError> {
    if is_webp(data) {
        return decode_webp(data);
    }
    if is_jpeg(data) {
        return decode_jpeg(data);
    }
    decode_fallback(data)
}

fn is_webp(data: &[u8]) -> bool {
    matches!(data.get(0..12), Some(b) if &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP")
}
/// Décode une image WebP via libwebp (bindings natifs).
///
/// # Pourquoi libwebp ?
/// Le décodeur pur-Rust (image-webp) atteint 70-100% de la vitesse de libwebp.
/// Avec les optimisations assembleur de libwebp, on gagne ~30-40% sur le décodage.
fn decode_webp(data: &[u8]) -> Result<RawImage, ImageProcessingError> {
    let decoder = webp::Decoder::new(data);
    decoder
        .decode()
        .map(RawImage::WebP)
        .ok_or_else(|| ImageProcessingError::DecodeError("Failed to decode WebP".to_string()))
}
fn is_jpeg(data: &[u8]) -> bool {
    data.len() > 2 && data[0] == 0xFF && data[1] == 0xD8
}
/// Helper pour finaliser le décodage zune-jpeg après succès de la décompression.
/// Pourquoi ? Isole la manipulation des buffers zune pour plus de clarté.
/// # Arguments
/// * `decoder` - Le décodeur zune-jpeg.
/// * `pixels` - Les pixels de l'image.
///
/// # Returns
/// Un `RawImage` contenant l'image décodée.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - Si les dimensions sont manquantes ou si la création du buffer échoue.
fn decode_jpeg(data: &[u8]) -> Result<RawImage, ImageProcessingError> {
    let mut decoder = zune_jpeg::JpegDecoder::new(ZCursor::new(data));
    if let Ok(pixels) = decoder.decode()
        && let Some((width, height)) = decoder.dimensions()
    {
        return Ok(RawImage::Jpeg {
            width: u32::try_from(width).map_err(|_| {
                ImageProcessingError::DecodeError("Failed to decode JPEG".to_string())
            })?,
            height: u32::try_from(height).map_err(|_| {
                ImageProcessingError::DecodeError("Failed to decode JPEG".to_string())
            })?,
            data: pixels,
        });
    }
    eprintln!("zune-jpeg failed, falling back");
    decode_fallback(data)
}

fn decode_fallback(data: &[u8]) -> Result<RawImage, ImageProcessingError> {
    image::load_from_memory(data)
        .map(RawImage::Fallback)
        .map_err(|e| ImageProcessingError::DecodeError(e.to_string()))
}
