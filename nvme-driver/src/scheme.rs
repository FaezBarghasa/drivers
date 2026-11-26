// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

//! High-performance NVMe scheme handler with multi-queue support

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::bail;
use crossbeam_queue::ArrayQueue;
use log::{debug, error, info, trace, warn};
use parking_lot::{Mutex, RwLock};
use spin::RwLock as SpinRwLock;

use nvme::{
    Command, CompletionQueue, Controller, Doorbell, InterruptMethod, Nvme, NvmeFuture,
    SubmissionQueue,
};
use syscall::{physmap, physunmap, Io, Physmap};

use crate::queue::{IoQueue, PendingCommand, QueuePair};
use crate::stats::GLOBAL_STATS;
use crate::{DriverConfig, IoSchedulerType};

/// Maximum number of namespaces
const MAX_NAMESPACES: usize = 256;

/// Maximum commands in flight per queue
const MAX_QUEUE_DEPTH: usize = 4096;

/// NVMe namespace information
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    pub id: u32,
    pub size: u64,       // Total size in bytes
    pub block_size: u32, // Block size in bytes
    pub blocks: u64,     // Number of blocks
    pub optimal_write_size: u32,
    pub max_transfer_size: u32,
}

/// Handle to an open NVMe namespace
pub struct NvmeHandle {
    /// Namespace ID
    pub ns_id: u32,
    /// Namespace info
    pub ns_info: NamespaceInfo,
    /// Assigned queue for this handle
    pub queue_id: usize,
    /// Current offset for sequential operations
    pub offset: AtomicU64,
    /// Handle flags
    pub flags: u32,
    /// Creation time for debugging
    pub created_at: Instant,
}

/// Submission queue entry with priority
#[derive(Debug)]
pub struct SubmissionEntry {
    pub command_id: u16,
    pub priority: u8,
    pub deadline_us: u64,
    pub submitted_at: Instant,
}

/// NVMe scheme implementation
pub struct NvmeScheme {
    /// PCI handle
    pci_handle: usize,
    /// NVMe controller instance
    nvme: Nvme,
    /// Detected namespaces
    namespaces: BTreeMap<u32, NamespaceInfo>,
    /// Open handles
    handles: RwLock<BTreeMap<u64, NvmeHandle>>,
    /// Next handle ID
    next_handle_id: AtomicU64,
    /// I/O queue pairs (one per CPU core)
    pub queues: Vec<Arc<QueuePair>>,
    /// Queue selection counter for round-robin
    queue_counter: AtomicUsize,
    /// Driver configuration
    config: DriverConfig,
    /// Admin queue for controller commands
    admin_queue: Arc<QueuePair>,
}

impl NvmeScheme {
    /// Create a new NVMe scheme
    pub fn new(
        pci_handle: usize,
        pci_config: &[u8],
        config: &DriverConfig,
    ) -> anyhow::Result<Self> {
        let mut nvme = Nvme::new(pci_config)?;

        // Initialize with MSI-X for best multi-queue performance
        nvme.init(InterruptMethod::MsiX)?;

        // Discover namespaces
        let mut namespaces = BTreeMap::new();
        let ctrl_info = nvme.identify_controller();

        info!("NVMe Controller: {:?}", ctrl_info.model_number);
        info!("  Serial: {:?}", ctrl_info.serial_number);
        info!("  Firmware: {:?}", ctrl_info.firmware_revision);
        info!("  Max Namespaces: {}", ctrl_info.nvm_ns_count);

        for i in 0..ctrl_info.nvm_ns_count {
            let ns_id = i + 1;
            if let Some(ctrl) = nvme.namespace(ns_id) {
                let ns_info = NamespaceInfo {
                    id: ns_id,
                    size: ctrl.size(),
                    block_size: ctrl.block_size() as u32,
                    blocks: ctrl.blocks(),
                    optimal_write_size: ctrl.optimal_write_size().unwrap_or(128 * 1024),
                    max_transfer_size: ctrl.max_transfer_size().unwrap_or(1024 * 1024),
                };

                info!(
                    "  Namespace {}: {} GB, {} byte blocks",
                    ns_id,
                    ns_info.size / (1024 * 1024 * 1024),
                    ns_info.block_size
                );

                namespaces.insert(ns_id, ns_info);
            }
        }

        // Determine number of I/O queues
        let num_queues = if config.num_queues == 0 {
            num_cpus::get().min(ctrl_info.max_sq_entries as usize)
        } else {
            config.num_queues.min(ctrl_info.max_sq_entries as usize)
        };

        info!("Creating {} I/O queue pairs", num_queues);

        // Create I/O queue pairs
        let queue_pairs = nvme
            .create_io_queues(num_queues)?
            .into_iter()
            .enumerate()
            .map(|(id, (sq, cq, doorbell))| {
                Arc::new(QueuePair::new(id, sq, cq, doorbell, config.queue_depth))
            })
            .collect();

        // Create admin queue pair
        let admin_queue = Arc::new(QueuePair::new_admin(
            nvme.admin_sq(),
            nvme.admin_cq(),
            nvme.admin_doorbell(),
        ));

        Ok(Self {
            pci_handle,
            nvme,
            namespaces,
            handles: RwLock::new(BTreeMap::new()),
            next_handle_id: AtomicU64::new(1),
            queues: queue_pairs,
            queue_counter: AtomicUsize::new(0),
            config: config.clone(),
            admin_queue,
        })
    }

    /// Get IRQ number for a queue
    pub fn get_queue_irq(&self, queue_id: usize) -> u16 {
        // MSI-X typically maps queue N to vector N+1 (vector 0 is admin)
        (queue_id + 1) as u16
    }

    /// Select a queue for a new I/O operation
    fn select_queue(&self, handle: &NvmeHandle) -> usize {
        match self.config.scheduler {
            IoSchedulerType::None | IoSchedulerType::RoundRobin => {
                // Simple round-robin
                self.queue_counter.fetch_add(1, Ordering::Relaxed) % self.queues.len()
            }
            IoSchedulerType::CpuAffinity => {
                // Use handle's assigned queue (set based on opening CPU)
                handle.queue_id
            }
            IoSchedulerType::Priority | IoSchedulerType::Deadline => {
                // For priority/deadline, use dedicated high-priority queue if available
                if self.queues.len() > 1 {
                    // Queue 0 for high priority, others for normal
                    handle.queue_id.min(self.queues.len() - 1)
                } else {
                    0
                }
            }
        }
    }

    /// Process completions for a specific queue
    pub fn process_completions(&mut self, queue_id: usize) -> usize {
        let queue = &self.queues[queue_id];
        let mut count = 0;

        while let Some(completion) = queue.poll_completion() {
            let latency = completion.submitted_at.elapsed();

            // Update statistics
            #[cfg(feature = "performance-counters")]
            {
                GLOBAL_STATS.record_io_complete(completion.bytes, completion.is_write, latency);
            }

            // Complete the pending request
            if let Some(pending) = queue.complete_command(completion.command_id) {
                // Unmap physical memory if needed
                if let Some(phys) = pending.phys {
                    unsafe {
                        let _ = physunmap(phys.address, phys.size);
                    }
                }

                // Send response to caller
                let mut packet = pending.packet;
                packet.a = if completion.status == 0 {
                    completion.bytes
                } else {
                    syscall::Error::new(syscall::EIO).to_errno()
                };

                let _ = syscall::write(self.pci_handle, &packet);
            }

            count += 1;
        }

        count
    }

    /// Handle a scheme packet
    pub fn handle(&mut self, packet: &mut libredox::Packet) {
        let (a, b, c, d) = libredox::flag::decode_usize(packet.a);

        match (a, b, c, d) {
            (libredox::flag::SYS_OPEN, _, _, _) => {
                self.handle_open(packet);
            }
            (libredox::flag::SYS_READ, _, _, _) => {
                self.handle_read(packet);
            }
            (libredox::flag::SYS_WRITE, _, _, _) => {
                self.handle_write(packet);
            }
            (libredox::flag::SYS_FSTAT, _, _, _) => {
                self.handle_fstat(packet);
            }
            (libredox::flag::SYS_FPATH, _, _, _) => {
                self.handle_fpath(packet);
            }
            (libredox::flag::SYS_LSEEK, _, _, _) => {
                self.handle_lseek(packet);
            }
            (libredox::flag::SYS_FSYNC, _, _, _) => {
                self.handle_fsync(packet);
            }
            (libredox::flag::SYS_CLOSE, _, _, _) => {
                self.handle_close(packet);
            }
            (libredox::flag::SYS_FTRUNCATE, _, _, _) => {
                // NVMe doesn't support truncate
                packet.a = syscall::Error::new(syscall::ENOSYS).to_errno();
            }
            _ => {
                error!("nvme: unknown syscall {}", a);
                packet.a = syscall::Error::new(syscall::ENOSYS).to_errno();
            }
        }
    }

    /// Handle SYS_OPEN
    fn handle_open(&mut self, packet: &mut libredox::Packet) {
        let path = unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                packet.c as *const u8,
                packet.d,
            ))
        };

        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            // Root: list namespaces
            // TODO: implement directory listing
            packet.a = syscall::Error::new(syscall::EISDIR).to_errno();
            return;
        }

        let ns_id: u32 = match parts[0].parse() {
            Ok(id) => id,
            Err(_) => {
                packet.a = syscall::Error::new(syscall::ENOENT).to_errno();
                return;
            }
        };

        let ns_info = match self.namespaces.get(&ns_id) {
            Some(info) => info.clone(),
            None => {
                packet.a = syscall::Error::new(syscall::ENODEV).to_errno();
                return;
            }
        };

        // Assign a queue based on CPU affinity or round-robin
        let queue_id = self.queue_counter.fetch_add(1, Ordering::Relaxed) % self.queues.len();

        let handle_id = self.next_handle_id.fetch_add(1, Ordering::Relaxed);

        let handle = NvmeHandle {
            ns_id,
            ns_info,
            queue_id,
            offset: AtomicU64::new(0),
            flags: packet.b as u32,
            created_at: Instant::now(),
        };

        self.handles.write().insert(handle_id, handle);

        debug!("nvme: opened namespace {} as handle {}", ns_id, handle_id);

        packet.a = handle_id as usize;
    }

    /// Handle SYS_READ - async read with zero-copy support
    fn handle_read(&mut self, packet: &mut libredox::Packet) {
        let handle_id = packet.b as u64;
        let offset = packet.e as u64;
        let size = packet.d;

        let handles = self.handles.read();
        let handle = match handles.get(&handle_id) {
            Some(h) => h,
            None => {
                packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                return;
            }
        };

        let queue_id = self.select_queue(handle);
        let queue = &self.queues[queue_id];
        let ns_info = &handle.ns_info;

        // Calculate LBA and block count
        let lba = offset / ns_info.block_size as u64;
        let blocks =
            ((size + ns_info.block_size as usize - 1) / ns_info.block_size as usize) as u16;

        // Zero-copy mode: use physical address directly
        let (phys, data_ptr) = if self.config.zero_copy && (packet.c & 1 == 1) {
            // Physical address passed in
            let phys_addr = packet.c & !1;
            let phys = match unsafe { physmap(phys_addr, size, 0) } {
                Ok(p) => p,
                Err(_) => {
                    packet.a = syscall::Error::new(syscall::EFAULT).to_errno();
                    return;
                }
            };
            (Some(phys), phys.address)
        } else {
            // Virtual address - need to allocate DMA buffer
            (None, packet.c)
        };

        // Submit read command
        let cmd_id = match queue.submit_read(ns_info.id, lba, blocks, data_ptr, size) {
            Some(id) => id,
            None => {
                // Queue full
                if let Some(p) = phys {
                    unsafe {
                        let _ = physunmap(p.address, p.size);
                    }
                }
                packet.a = syscall::Error::new(syscall::EAGAIN).to_errno();
                return;
            }
        };

        // Store pending request
        queue.add_pending(
            cmd_id,
            PendingCommand {
                packet: *packet,
                phys,
                submitted_at: Instant::now(),
                is_write: false,
                bytes: size,
            },
        );

        #[cfg(feature = "performance-counters")]
        {
            GLOBAL_STATS.record_io_submit(size, false);
        }

        // Don't set packet.a - completion will be async
    }

    /// Handle SYS_WRITE - async write with zero-copy support
    fn handle_write(&mut self, packet: &mut libredox::Packet) {
        let handle_id = packet.b as u64;
        let offset = packet.e as u64;
        let size = packet.d;

        let handles = self.handles.read();
        let handle = match handles.get(&handle_id) {
            Some(h) => h,
            None => {
                packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                return;
            }
        };

        let queue_id = self.select_queue(handle);
        let queue = &self.queues[queue_id];
        let ns_info = &handle.ns_info;

        // Calculate LBA and block count
        let lba = offset / ns_info.block_size as u64;
        let blocks =
            ((size + ns_info.block_size as usize - 1) / ns_info.block_size as usize) as u16;

        // Zero-copy mode
        let (phys, data_ptr) = if self.config.zero_copy && (packet.c & 1 == 1) {
            let phys_addr = packet.c & !1;
            let phys = match unsafe { physmap(phys_addr, size, 0) } {
                Ok(p) => p,
                Err(_) => {
                    packet.a = syscall::Error::new(syscall::EFAULT).to_errno();
                    return;
                }
            };
            (Some(phys), phys.address)
        } else {
            (None, packet.c)
        };

        // Submit write command
        let cmd_id = match queue.submit_write(ns_info.id, lba, blocks, data_ptr, size) {
            Some(id) => id,
            None => {
                if let Some(p) = phys {
                    unsafe {
                        let _ = physunmap(p.address, p.size);
                    }
                }
                packet.a = syscall::Error::new(syscall::EAGAIN).to_errno();
                return;
            }
        };

        queue.add_pending(
            cmd_id,
            PendingCommand {
                packet: *packet,
                phys,
                submitted_at: Instant::now(),
                is_write: true,
                bytes: size,
            },
        );

        #[cfg(feature = "performance-counters")]
        {
            GLOBAL_STATS.record_io_submit(size, true);
        }
    }

    /// Handle SYS_FSTAT
    fn handle_fstat(&self, packet: &mut libredox::Packet) {
        let handle_id = packet.b as u64;

        let handles = self.handles.read();
        let handle = match handles.get(&handle_id) {
            Some(h) => h,
            None => {
                packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                return;
            }
        };

        let stat = libredox::Stat {
            st_mode: libredox::flag::MODE_FILE,
            st_size: handle.ns_info.size,
            st_blksize: handle.ns_info.block_size as u64,
            st_blocks: handle.ns_info.blocks as u64,
            ..Default::default()
        };

        let buf = unsafe { std::slice::from_raw_parts_mut(packet.c as *mut libredox::Stat, 1) };
        buf[0] = stat;

        packet.a = 0;
    }

    /// Handle SYS_FPATH
    fn handle_fpath(&self, packet: &mut libredox::Packet) {
        let handle_id = packet.b as u64;

        let handles = self.handles.read();
        let handle = match handles.get(&handle_id) {
            Some(h) => h,
            None => {
                packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                return;
            }
        };

        let path = format!("nvme:{}/", handle.ns_id);
        let buf = unsafe { std::slice::from_raw_parts_mut(packet.c as *mut u8, packet.d) };

        let copy_len = path.len().min(buf.len());
        buf[..copy_len].copy_from_slice(&path.as_bytes()[..copy_len]);

        packet.a = copy_len;
    }

    /// Handle SYS_LSEEK
    fn handle_lseek(&self, packet: &mut libredox::Packet) {
        let handle_id = packet.b as u64;
        let offset = packet.c as i64;
        let whence = packet.d;

        let handles = self.handles.read();
        let handle = match handles.get(&handle_id) {
            Some(h) => h,
            None => {
                packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                return;
            }
        };

        let current = handle.offset.load(Ordering::Relaxed);
        let size = handle.ns_info.size;

        let new_offset = match whence as i32 {
            libredox::flag::SEEK_SET => offset as u64,
            libredox::flag::SEEK_CUR => (current as i64 + offset) as u64,
            libredox::flag::SEEK_END => (size as i64 + offset) as u64,
            _ => {
                packet.a = syscall::Error::new(syscall::EINVAL).to_errno();
                return;
            }
        };

        handle.offset.store(new_offset, Ordering::Relaxed);
        packet.a = new_offset as usize;
    }

    /// Handle SYS_FSYNC - flush writes to stable storage
    fn handle_fsync(&mut self, packet: &mut libredox::Packet) {
        let handle_id = packet.b as u64;

        let handles = self.handles.read();
        let handle = match handles.get(&handle_id) {
            Some(h) => h,
            None => {
                packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                return;
            }
        };

        let queue = &self.queues[handle.queue_id];

        // Submit flush command
        if let Some(cmd_id) = queue.submit_flush(handle.ns_info.id) {
            queue.add_pending(
                cmd_id,
                PendingCommand {
                    packet: *packet,
                    phys: None,
                    submitted_at: Instant::now(),
                    is_write: false,
                    bytes: 0,
                },
            );
        } else {
            packet.a = syscall::Error::new(syscall::EAGAIN).to_errno();
        }
    }

    /// Handle SYS_CLOSE
    fn handle_close(&mut self, packet: &mut libredox::Packet) {
        let handle_id = packet.b as u64;

        if self.handles.write().remove(&handle_id).is_some() {
            debug!("nvme: closed handle {}", handle_id);
            packet.a = 0;
        } else {
            packet.a = syscall::Error::new(syscall::EBADF).to_errno();
        }
    }
}

impl Drop for NvmeScheme {
    fn drop(&mut self) {
        info!("NVMe driver shutting down");

        // Wait for pending I/Os
        for queue in &self.queues {
            queue.wait_idle();
        }
    }
}
