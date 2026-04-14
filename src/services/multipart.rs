/// Utilitaire pour construire une réponse `multipart/mixed`.
///
/// # Pourquoi ?
/// Simplifie le contrôleur en déportant la manipulation des buffers et des boundaries.
pub struct MultipartBuilder {
    boundary: String,
    buffer: Vec<u8>,
}

impl MultipartBuilder {
    #[must_use]
    pub fn new(boundary: &str) -> Self {
        Self {
            boundary: boundary.to_string(),
            buffer: Vec::new(),
        }
    }

    /// Ajoute une partie (une image) au corps multipart.
    pub fn add_part(&mut self, filename: &str, content_type: &str, data: &[u8]) {
        self.buffer
            .extend_from_slice(format!("--{}\r\n", self.boundary).as_bytes());
        self.buffer
            .extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
        self.buffer.extend_from_slice(
            format!("Content-Disposition: attachment; filename=\"{filename}\"\r\n\r\n").as_bytes(),
        );
        self.buffer.extend_from_slice(data);
        self.buffer.extend_from_slice(b"\r\n");
    }

    /// Finalise le corps multipart avec la boundary finale.
    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        if !self.buffer.is_empty() {
            self.buffer
                .extend_from_slice(format!("--{}--\r\n", self.boundary).as_bytes());
        }
        self.buffer
    }
}
