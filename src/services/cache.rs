use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// In-memory cache for processed images.
///
/// # Why this cache?
/// Avoids re-processing the same image at the same dimensions,
/// which is CPU-expensive (decode + resize + encode).
///
/// # Limits
/// - Max 100 MB of RAM.
/// - Key: `(image_name, width, height)`.
pub struct ImageCache {
    storage: RwLock<HashMap<String, Vec<u8>>>,
    current_size: AtomicUsize,
    max_size: usize,
}

impl ImageCache {
    /// Creates a new cache instance with the given byte capacity.
    #[must_use]
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
            current_size: AtomicUsize::new(0),
            max_size: max_size_bytes,
        }
    }

    /// Attempts to retrieve a processed image from the cache.
    pub fn get(&self, name: &str, w: u32, h: u32) -> Option<Vec<u8>> {
        let key = format!("{name}_{w}_{h}");
        let storage = self.storage.read().ok()?;
        storage.get(&key).cloned()
    }

    /// Inserts an image into the cache if capacity allows.
    pub fn set(&self, name: &str, w: u32, h: u32, data: Vec<u8>) {
        let size = data.len();
        let key = format!("{name}_{w}_{h}");

        if let Ok(mut storage) = self.storage.write() {
            if storage.contains_key(&key) {
                return;
            }
            let current = self.current_size.load(Ordering::SeqCst);
            if current + size > self.max_size {
                tracing::warn!(
                    "Cache limit reached ({} bytes), skipping insertion",
                    self.max_size
                );
                return;
            }
            self.current_size.fetch_add(size, Ordering::SeqCst);
            storage.insert(key, data);
        }
    }
}

/// Global singleton cache instance (20 MB).
/// Uses `std::sync::OnceLock` for safe lazy initialization.
///
/// # Why 20 MB?
/// Reduced from 50 MB to leave enough headroom for the image processing pipeline
/// (decode + resize + sharpen + encode) within the 500 Mi Kubernetes pod budget.
pub static GLOBAL_CACHE: std::sync::OnceLock<ImageCache> = std::sync::OnceLock::new();

pub fn get_global_cache() -> &'static ImageCache {
    GLOBAL_CACHE.get_or_init(|| ImageCache::new(20 * 1024 * 1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_limit() {
        let cache = ImageCache::new(10);
        let data = vec![1, 2, 3, 4, 5, 6];

        cache.set("img1", 10, 10, data.clone());
        assert!(cache.get("img1", 10, 10).is_some());

        // This should fail to cache because it would exceed 10 bytes
        cache.set("img2", 10, 10, vec![1, 2, 3, 4, 5]);
        assert!(cache.get("img2", 10, 10).is_none());
    }
}
