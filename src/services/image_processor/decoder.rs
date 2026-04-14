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

/// Decodes an image from a byte buffer.
///
/// Tries WebP, then JPEG (zune-jpeg), then falls back to the `image` crate.
///
/// # Arguments
/// * `data` - Raw image bytes.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - If the image cannot be decoded.
/// * `ImageProcessingError::InternalError` - If an internal error occurs.
///
/// # Returns
/// * `Ok(RawImage)` - Decoded image in its raw form.
/// * `Err(ImageProcessingError)` - Decode error.
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

/// Decodes a WebP image via libwebp (native FFI bindings).
///
/// # Why libwebp over pure-Rust image-webp?
/// The pure-Rust decoder reaches 70-100% of libwebp speed.
/// libwebp's assembly optimizations give an additional ~30-40% on decode.
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

/// Decodes a JPEG using zune-jpeg, falling back to the `image` crate on failure.
///
/// Isolates zune buffer manipulation for clarity.
///
/// # Arguments
/// * `data` - Raw JPEG bytes.
///
/// # Returns
/// A `RawImage::Jpeg` on success, or falls back to `decode_fallback`.
///
/// # Errors
/// * `ImageProcessingError::DecodeError` - If dimensions are missing or buffer creation fails.
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
