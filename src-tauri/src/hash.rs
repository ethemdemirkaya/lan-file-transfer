use blake3::Hasher;

/// Streaming blake3 hasher wrapper.
pub struct StreamHasher(Hasher);

impl StreamHasher {
    pub fn new() -> Self {
        Self(Hasher::new())
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    pub fn finalize(self) -> [u8; 32] {
        *self.0.finalize().as_bytes()
    }
}

impl Default for StreamHasher {
    fn default() -> Self {
        Self::new()
    }
}
