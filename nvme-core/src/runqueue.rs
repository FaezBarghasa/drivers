#![forbid(unsafe_code)]

//! # Lock-Free NVMe Per-Core Runqueue & I/O Coalescing Engine
//!
//! Provides isolated per-core submission and completion ring queues using lock-free
//! atomic structures, eliminating global NVMe lock contention. Features a dynamic
//! I/O coalescing engine that merges adjacent sector requests into single DMA operations.
//!
//! ## Mathematical & Coalescing Model
//! Given two adjacent I/O submission descriptors $D_a$ and $D_b$ with starting LBA
//! $L_a, L_b$ and sector counts $S_a, S_b$:
//! $$\text{Coalesceable}(D_a, D_b) = \begin{cases} \text{true} & \text{if } L_a + S_a = L_b \\ \text{false} & \text{otherwise} \end{cases}$$

use alloc::sync::Arc;
use alloc::vec::Vec;
use crossbeam_queue::ArrayQueue;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// NVMe Block Command Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeOpcode {
    Read = 0x02,
    Write = 0x01,
    Flush = 0x00,
}

/// NVMe Block Submission Descriptor.
#[derive(Debug, Clone, Copy)]
pub struct NvmeBlockDescriptor {
    pub opcode: NvmeOpcode,
    pub lba_start: u64,
    pub sector_count: u32,
    pub dma_buffer_phys: u64,
    pub command_id: u16,
}

/// NVMe Block Completion Response.
#[derive(Debug, Clone, Copy)]
pub struct NvmeCompletionResponse {
    pub command_id: u16,
    pub status_code: u16,
}

/// Lock-free per-core queue instance for high-throughput NVMe transactions.
pub struct PerCoreNvmeQueue {
    pub core_id: usize,
    pub submission_ring: ArrayQueue<NvmeBlockDescriptor>,
    pub completion_ring: ArrayQueue<NvmeCompletionResponse>,
    pub total_ops_submitted: AtomicU64,
    pub total_coalesced: AtomicU64,
}

impl PerCoreNvmeQueue {
    /// Creates a new per-core lock-free NVMe queue pair.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn new(core_id: usize, capacity: usize) -> Self {
        Self {
            core_id,
            submission_ring: ArrayQueue::new(capacity),
            completion_ring: ArrayQueue::new(capacity),
            total_ops_submitted: AtomicU64::new(0),
            total_coalesced: AtomicU64::new(0),
        }
    }

    /// Enqueues a block descriptor into the submission ring without locking.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn submit(&self, desc: NvmeBlockDescriptor) -> Result<(), NvmeBlockDescriptor> {
        self.submission_ring.push(desc)?;
        self.total_ops_submitted.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Merges adjacent sector submission descriptors in batch before sending to hardware.
    ///
    /// # Mathematical Model
    /// Merge condition: $L_a + S_a = L_b$ for identical opcodes.
    ///
    /// Complexity: $\mathcal{O}(N)$ where $N$ is pending batch size.
    pub fn coalesce_pending_requests(&self) -> Vec<NvmeBlockDescriptor> {
        let mut raw_batch = Vec::new();
        while let Some(desc) = self.submission_ring.pop() {
            raw_batch.push(desc);
        }

        if raw_batch.is_empty() {
            return Vec::new();
        }

        let mut coalesced = Vec::with_capacity(raw_batch.len());
        let mut current = raw_batch[0];

        for next in raw_batch.into_iter().skip(1) {
            if current.opcode == next.opcode
                && current.opcode != NvmeOpcode::Flush
                && current.lba_start + (current.sector_count as u64) == next.lba_start
            {
                // Merge adjacent sector requests!
                current.sector_count += next.sector_count;
                self.total_coalesced.fetch_add(1, Ordering::Relaxed);
            } else {
                coalesced.push(current);
                current = next;
            }
        }
        coalesced.push(current);

        coalesced
    }
}
