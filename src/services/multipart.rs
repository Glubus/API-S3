/// Utility for building `multipart/mixed` responses.
///
/// Moves buffer and boundary manipulation out of controllers,
/// keeping response construction logic in one place.
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

    /// Appends a part (one image) to the multipart body.
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

    /// Finalizes the multipart body by appending the closing boundary.
    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        if !self.buffer.is_empty() {
            self.buffer
                .extend_from_slice(format!("--{}--\r\n", self.boundary).as_bytes());
        }
        self.buffer
    }
}
