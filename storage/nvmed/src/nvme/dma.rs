//! DMA Engine for Zero-Copy Transfers

use std::sync::Arc;

/// Physical Region Page (PRP) entry
pub type PrpEntry = u64;

/// PRP list for scatter-gather
pub struct PrpList {
    entries: Vec<PrpEntry>,
}

impl PrpList {
    /// Create PRP list from buffer
    pub fn from_buffer(phys_addr: u64, size: usize) -> Self {
        let page_size = 4096;
        let num_pages = (size + page_size - 1) / page_size;

        let mut entries = Vec::with_capacity(num_pages);
        for i in 0..num_pages {
            entries.push(phys_addr + (i * page_size) as u64);
        }

        Self { entries }
    }

    /// Get first PRP (PRP1)
    pub fn prp1(&self) -> u64 {
        self.entries.first().copied().unwrap_or(0)
    }

    /// Get second PRP or PRP list pointer (PRP2)
    pub fn prp2(&self) -> u64 {
        if self.entries.len() <= 2 {
            self.entries.get(1).copied().unwrap_or(0)
        } else {
            // Would return pointer to PRP list
            0
        }
    }
}

/// DMA buffer
pub struct DmaBuffer {
    /// Virtual address
    pub virt_addr: usize,
    /// Physical address
    pub phys_addr: u64,
    /// Size in bytes
    pub size: usize,
}

impl DmaBuffer {
    /// Allocate DMA buffer
    pub fn alloc(size: usize) -> Result<Self, &'static str> {
        // Allocate page-aligned buffer
        let aligned_size = (size + 4095) & !4095;

        // TODO: Actual DMA allocation
        Ok(Self {
            virt_addr: 0,
            phys_addr: 0,
            size: aligned_size,
        })
    }

    /// Create PRP list
    pub fn prp_list(&self) -> PrpList {
        PrpList::from_buffer(self.phys_addr, self.size)
    }
}
