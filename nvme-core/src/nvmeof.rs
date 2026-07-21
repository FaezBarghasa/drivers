#![forbid(unsafe_code)]

//! # NVMe-oF (NVMe over Fabrics) Target Daemon & RDMA Zero-Copy Data Plane
//!
//! Exposes local NVMe block storage over RDMA fabrics (Infiniband / RoCEv2). Direct
//! DMA mapping between incoming RDMA memory keys and local NVMe submission rings bypasses
//! kernel network protocol layers, yielding microsecond remote disk I/O latency.
//!
//! ## Mathematical & Latency Model
//! Total remote I/O latency $L_{total}$:
//! $$L_{total} = L_{RDMA\_wire} + L_{DMA\_handoff} + L_{NVMe\_hw}$$
//! Direct DmaBuf mapping enforces $L_{DMA\_handoff} \approx 0 \mu s$.

use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use alloc::sync::Arc;
use spin::Mutex;
use crate::runqueue::{NvmeBlockDescriptor, NvmeOpcode, PerCoreNvmeQueue};

/// RDMA Fabric Command Packet.
#[derive(Debug, Clone, Copy)]
pub struct RdmaFabricCommand {
    pub remote_rkey: u32,
    pub remote_vaddr: u64,
    pub local_lba_start: u64,
    pub length_bytes: u32,
    pub is_write: bool,
    pub command_id: u16,
}

/// NVMe-oF Target Daemon controller.
pub struct NvmeOfTargetDaemon {
    pub is_active: AtomicBool,
    pub total_rdma_reads: AtomicU64,
    pub total_rdma_writes: AtomicU64,
    pub local_queue: Arc<PerCoreNvmeQueue>,
}

impl NvmeOfTargetDaemon {
    /// Creates a new NVMe-oF Target instance bound to a per-core NVMe queue.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn new(local_queue: Arc<PerCoreNvmeQueue>) -> Self {
        Self {
            is_active: AtomicBool::new(true),
            total_rdma_reads: AtomicU64::new(0),
            total_rdma_writes: AtomicU64::new(0),
            local_queue,
        }
    }

    /// Handles an incoming RDMA Read/Write request by zero-copy mapping directly
    /// into local NVMe submission ring.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn handle_rdma_request(&self, cmd: RdmaFabricCommand) -> Result<(), NvmeBlockDescriptor> {
        let opcode = if cmd.is_write {
            self.total_rdma_writes.fetch_add(1, Ordering::Relaxed);
            NvmeOpcode::Write
        } else {
            self.total_rdma_reads.fetch_add(1, Ordering::Relaxed);
            NvmeOpcode::Read
        };

        // Calculate sector count (assuming 512-byte sectors)
        let sector_count = (cmd.length_bytes + 511) / 512;

        let block_desc = NvmeBlockDescriptor {
            opcode,
            lba_start: cmd.local_lba_start,
            sector_count,
            dma_buffer_phys: cmd.remote_vaddr, // Direct zero-copy RDMA buffer
            command_id: cmd.command_id,
        };

        self.local_queue.submit(block_desc)
    }
}
