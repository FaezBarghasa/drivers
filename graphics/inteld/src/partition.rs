//! GPU partition management: VRAM range allocation and isolation enforcement.
//!
//! Each `GpuPartition` owns an exclusive VRAM range and a core mask.
//! The `PartitionTable` enforces that no two partitions overlap in either
//! VRAM space or EU core assignment.

use spin::Mutex;
use std::vec::Vec;

use crate::mmio::{
    MmioRegs, FENCE_BASE, FENCE_COUNT, FENCE_STRIDE, FENCE_TILE_XMAJOR, FENCE_VALID,
};

/// A single GPU partition: an exclusive VRAM range and EU core mask.
#[derive(Debug, Clone)]
pub struct GpuPartition {
    /// Partition index (0-based).
    pub id: usize,
    /// Bitmask of EU slices assigned to this partition.
    pub core_mask: u64,
    /// Start of the VRAM range (bytes from VRAM base).
    pub vram_start: u64,
    /// Size of the VRAM range in bytes.
    pub vram_size: u64,
}

impl GpuPartition {
    /// Returns the exclusive end of the VRAM range.
    pub fn vram_end(&self) -> u64 {
        self.vram_start + self.vram_size
    }

    /// Returns true if this partition's VRAM range overlaps with `other`.
    pub fn vram_overlaps(&self, other: &GpuPartition) -> bool {
        self.vram_start < other.vram_end() && other.vram_start < self.vram_end()
    }

    /// Returns true if this partition's core mask overlaps with `other`.
    pub fn cores_overlap(&self, other: &GpuPartition) -> bool {
        self.core_mask & other.core_mask != 0
    }
}

/// Global partition table: tracks all active GPU partitions and enforces isolation.
pub struct PartitionTable {
    partitions: Mutex<Vec<GpuPartition>>,
    /// Total VRAM size in bytes (used for range validation).
    total_vram: u64,
}

impl PartitionTable {
    pub fn new(total_vram: u64) -> Self {
        Self {
            partitions: Mutex::new(Vec::new()),
            total_vram,
        }
    }

    /// Register a new partition.
    ///
    /// Returns `Err` if the partition overlaps an existing one in VRAM or cores,
    /// or if the VRAM range exceeds the total VRAM size.
    pub fn add_partition(&self, partition: GpuPartition) -> Result<(), &'static str> {
        if partition.vram_size == 0 {
            return Err("partition VRAM size must be non-zero");
        }
        if partition.vram_end() > self.total_vram {
            return Err("partition VRAM range exceeds total VRAM");
        }
        if partition.core_mask == 0 {
            return Err("partition core mask must be non-zero");
        }

        let mut partitions = self.partitions.lock();
        for existing in partitions.iter() {
            if partition.vram_overlaps(existing) {
                return Err("VRAM range overlaps existing partition");
            }
            if partition.cores_overlap(existing) {
                return Err("core mask overlaps existing partition");
            }
        }
        partitions.push(partition);
        Ok(())
    }

    /// Remove a partition by ID.
    pub fn remove_partition(&self, id: usize) {
        self.partitions.lock().retain(|p| p.id != id);
    }

    /// Look up a partition by ID.
    pub fn get_partition(&self, id: usize) -> Option<GpuPartition> {
        self.partitions.lock().iter().find(|p| p.id == id).cloned()
    }

    /// Program VRAM fence registers for all partitions.
    ///
    /// Each partition gets one fence register that limits its VRAM access to
    /// `[vram_start, vram_end)`. Unused fence slots are cleared.
    ///
    /// # Safety
    /// `mmio` must be a valid MMIO accessor for the GPU device.
    pub fn program_fences(&self, mmio: &MmioRegs) {
        let partitions = self.partitions.lock();
        for (slot, partition) in partitions.iter().enumerate() {
            if slot >= FENCE_COUNT {
                break;
            }
            let base_off = FENCE_BASE + slot * FENCE_STRIDE;
            // Lower DWORD: [31:12] = vram_start >> 12, tile mode, valid.
            let lo = ((partition.vram_start >> 12) as u32) << 12 | FENCE_TILE_XMAJOR | FENCE_VALID;
            // Upper DWORD: [31:12] = (vram_end - 1) >> 12.
            let hi = (((partition.vram_end() - 1) >> 12) as u32) << 12;
            mmio.write32(base_off, lo);
            mmio.write32(base_off + 4, hi);
        }
        // Clear unused fence slots.
        for slot in partitions.len()..FENCE_COUNT {
            let base_off = FENCE_BASE + slot * FENCE_STRIDE;
            mmio.write32(base_off, 0);
            mmio.write32(base_off + 4, 0);
        }
    }

    pub fn check_core_overlap(&self, core_mask: u64) -> bool {
        let partitions = self.partitions.lock();
        for existing in partitions.iter() {
            if (existing.core_mask & core_mask) != 0 {
                return false;
            }
        }
        true
    }
}
