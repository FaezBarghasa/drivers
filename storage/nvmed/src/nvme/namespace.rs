//! NVMe Namespace Management

/// NVMe namespace
pub struct Namespace {
    /// Namespace ID
    pub id: u32,
    /// Size in blocks
    pub size: u64,
    /// Block size in bytes
    pub block_size: u32,
}

impl Namespace {
    /// Create new namespace
    pub fn new(id: u32, size: u64, block_size: u32) -> Self {
        log::info!(
            "Namespace {}: {} blocks, {} bytes/block",
            id,
            size,
            block_size
        );

        Self {
            id,
            size,
            block_size,
        }
    }

    /// Get size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size * self.block_size as u64
    }
}
