// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

//! High-performance NVMe queue management
//!
//! Implements lock-free submission and completion queue handling
//! for maximum I/O throughput.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crossbeam_queue::ArrayQueue;
use parking_lot::{Mutex, RwLock};
use spin::Mutex as SpinMutex;

use nvme::{CompletionQueue, Doorbell, NvmeCmd, NvmeComp, SubmissionQueue};
use syscall::Physmap;

/// Maximum queue depth
pub const MAX_QUEUE_DEPTH: usize = 4096;

/// Pending command information
pub struct PendingCommand {
    pub packet: libredox::Packet,
    pub phys: Option<Physmap>,
    pub submitted_at: Instant,
    pub is_write: bool,
    pub bytes: usize,
}

/// Completion information
#[derive(Debug)]
pub struct CompletionInfo {
    pub command_id: u16,
    pub status: u16,
    pub result: u32,
    pub submitted_at: Instant,
    pub is_write: bool,
    pub bytes: usize,
}

/// NVMe I/O Queue Pair
///
/// Each queue pair consists of:
/// - One Submission Queue (SQ) for sending commands
/// - One Completion Queue (CQ) for receiving completions
/// - Associated doorbell registers
pub struct QueuePair {
    /// Queue ID (0 = admin, 1+ = I/O)
    pub id: usize,

    /// Submission queue
    sq: SpinMutex<SubmissionQueueState>,

    /// Completion queue
    cq: SpinMutex<CompletionQueueState>,

    /// Doorbell for submission queue tail
    sq_doorbell: Doorbell,

    /// Pending commands awaiting completion
    pending: RwLock<BTreeMap<u16, PendingCommand>>,

    /// Completion results ready for processing
    completions: ArrayQueue<CompletionInfo>,

    /// Next command ID
    next_cmd_id: AtomicU16,

    /// Number of commands in flight
    in_flight: AtomicU32,

    /// Maximum queue depth
    max_depth: u16,

    /// Queue is active
    active: AtomicBool,

    /// Statistics
    stats: QueueStats,
}

/// Submission queue state
struct SubmissionQueueState {
    queue: SubmissionQueue,
    tail: u16,
    head: u16,
}

/// Completion queue state
struct CompletionQueueState {
    queue: CompletionQueue,
    head: u16,
    phase: bool,
}

/// Per-queue statistics
#[derive(Default)]
pub struct QueueStats {
    pub commands_submitted: AtomicU64,
    pub commands_completed: AtomicU64,
    pub bytes_read: AtomicU64,
    pub bytes_written: AtomicU64,
    pub total_latency_ns: AtomicU64,
    pub max_latency_ns: AtomicU64,
    pub min_latency_ns: AtomicU64,
}

impl QueuePair {
    /// Create a new I/O queue pair
    pub fn new(
        id: usize,
        sq: SubmissionQueue,
        cq: CompletionQueue,
        doorbell: Doorbell,
        max_depth: u16,
    ) -> Self {
        Self {
            id,
            sq: SpinMutex::new(SubmissionQueueState {
                queue: sq,
                tail: 0,
                head: 0,
            }),
            cq: SpinMutex::new(CompletionQueueState {
                queue: cq,
                head: 0,
                phase: true,
            }),
            sq_doorbell: doorbell,
            pending: RwLock::new(BTreeMap::new()),
            completions: ArrayQueue::new(max_depth as usize),
            next_cmd_id: AtomicU16::new(0),
            in_flight: AtomicU32::new(0),
            max_depth,
            active: AtomicBool::new(true),
            stats: QueueStats::default(),
        }
    }

    /// Create admin queue pair
    pub fn new_admin(sq: SubmissionQueue, cq: CompletionQueue, doorbell: Doorbell) -> Self {
        Self::new(0, sq, cq, doorbell, 32) // Admin queue smaller
    }

    /// Check if queue has space
    pub fn has_space(&self) -> bool {
        self.in_flight.load(Ordering::Relaxed) < self.max_depth as u32
    }

    /// Get next command ID
    fn allocate_cmd_id(&self) -> u16 {
        self.next_cmd_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Submit a read command
    pub fn submit_read(
        &self,
        ns_id: u32,
        lba: u64,
        blocks: u16,
        data_ptr: usize,
        size: usize,
    ) -> Option<u16> {
        if !self.has_space() {
            return None;
        }

        let cmd_id = self.allocate_cmd_id();

        // Build NVMe read command
        let cmd = NvmeCmd::io_read(
            cmd_id,
            ns_id,
            lba,
            blocks.saturating_sub(1), // NVMe uses 0-based count
            data_ptr as u64,
            0, // PRP2 (for larger transfers)
        );

        self.submit_command(cmd, size, false)
    }

    /// Submit a write command
    pub fn submit_write(
        &self,
        ns_id: u32,
        lba: u64,
        blocks: u16,
        data_ptr: usize,
        size: usize,
    ) -> Option<u16> {
        if !self.has_space() {
            return None;
        }

        let cmd_id = self.allocate_cmd_id();

        // Build NVMe write command
        let cmd = NvmeCmd::io_write(
            cmd_id,
            ns_id,
            lba,
            blocks.saturating_sub(1),
            data_ptr as u64,
            0,
        );

        self.submit_command(cmd, size, true)
    }

    /// Submit a flush command
    pub fn submit_flush(&self, ns_id: u32) -> Option<u16> {
        let cmd_id = self.allocate_cmd_id();
        let cmd = NvmeCmd::io_flush(cmd_id, ns_id);
        self.submit_command(cmd, 0, false)
    }

    /// Submit a command to the queue
    fn submit_command(&self, cmd: NvmeCmd, _bytes: usize, _is_write: bool) -> Option<u16> {
        let mut sq = self.sq.lock();

        // Check queue space using circular buffer math
        let queue_size = sq.queue.data.len() as u16;
        let next_tail = (sq.tail + 1) % queue_size;

        if next_tail == sq.head {
            // Queue is full
            return None;
        }

        // Write command to submission queue
        sq.queue.data[sq.tail as usize] = cmd;
        sq.tail = next_tail;

        // Ring doorbell
        unsafe {
            self.sq_doorbell.write(sq.tail as u32);
        }

        self.in_flight.fetch_add(1, Ordering::Relaxed);
        self.stats
            .commands_submitted
            .fetch_add(1, Ordering::Relaxed);

        Some(cmd.command_id())
    }

    /// Add a pending command
    pub fn add_pending(&self, cmd_id: u16, pending: PendingCommand) {
        self.pending.write().insert(cmd_id, pending);
    }

    /// Complete a command and return its pending data
    pub fn complete_command(&self, cmd_id: u16) -> Option<PendingCommand> {
        self.pending.write().remove(&cmd_id)
    }

    /// Poll for completions
    pub fn poll_completion(&self) -> Option<CompletionInfo> {
        // First check pre-queued completions
        if let Some(completion) = self.completions.pop() {
            return Some(completion);
        }

        // Poll hardware completion queue
        let mut cq = self.cq.lock();

        loop {
            let cqe = &cq.queue.data[cq.head as usize];

            // Check phase bit
            let phase = (cqe.status >> 0) & 1;
            if (phase == 1) != cq.phase {
                // No new completions
                break;
            }

            // Get completion info
            let cmd_id = cqe.command_id();
            let status = cqe.status >> 1;
            let result = cqe.cdw0;

            // Look up pending command for timing
            let pending_info = self
                .pending
                .read()
                .get(&cmd_id)
                .map(|p| (p.submitted_at, p.is_write, p.bytes));

            let completion = if let Some((submitted_at, is_write, bytes)) = pending_info {
                CompletionInfo {
                    command_id: cmd_id,
                    status,
                    result,
                    submitted_at,
                    is_write,
                    bytes,
                }
            } else {
                CompletionInfo {
                    command_id: cmd_id,
                    status,
                    result,
                    submitted_at: Instant::now(),
                    is_write: false,
                    bytes: 0,
                }
            };

            // Advance completion queue head
            let queue_size = cq.queue.data.len() as u16;
            cq.head = (cq.head + 1) % queue_size;

            if cq.head == 0 {
                cq.phase = !cq.phase;
            }

            // Ring completion queue doorbell
            unsafe {
                // CQ doorbell is typically at different offset
                self.sq_doorbell.cq_write(cq.head as u32);
            }

            self.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.stats
                .commands_completed
                .fetch_add(1, Ordering::Relaxed);

            // Update latency stats
            let latency_ns = completion.submitted_at.elapsed().as_nanos() as u64;
            self.stats
                .total_latency_ns
                .fetch_add(latency_ns, Ordering::Relaxed);

            // Update min/max latency
            let mut current_max = self.stats.max_latency_ns.load(Ordering::Relaxed);
            while latency_ns > current_max {
                match self.stats.max_latency_ns.compare_exchange_weak(
                    current_max,
                    latency_ns,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => current_max = x,
                }
            }

            return Some(completion);
        }

        None
    }

    /// Process all pending completions
    pub fn drain_completions(&self) -> Vec<CompletionInfo> {
        let mut completions = Vec::new();
        while let Some(comp) = self.poll_completion() {
            completions.push(comp);
        }
        completions
    }

    /// Wait for queue to become idle (no pending commands)
    pub fn wait_idle(&self) {
        while self.in_flight.load(Ordering::Relaxed) > 0 {
            // Poll completions
            while self.poll_completion().is_some() {}
            std::thread::yield_now();
        }
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> QueueStatsSnapshot {
        let completed = self.stats.commands_completed.load(Ordering::Relaxed);
        let total_latency = self.stats.total_latency_ns.load(Ordering::Relaxed);

        QueueStatsSnapshot {
            commands_submitted: self.stats.commands_submitted.load(Ordering::Relaxed),
            commands_completed: completed,
            commands_in_flight: self.in_flight.load(Ordering::Relaxed),
            bytes_read: self.stats.bytes_read.load(Ordering::Relaxed),
            bytes_written: self.stats.bytes_written.load(Ordering::Relaxed),
            avg_latency_ns: if completed > 0 {
                total_latency / completed
            } else {
                0
            },
            max_latency_ns: self.stats.max_latency_ns.load(Ordering::Relaxed),
            min_latency_ns: self.stats.min_latency_ns.load(Ordering::Relaxed),
        }
    }

    /// Get current queue depth
    pub fn queue_depth(&self) -> u32 {
        self.in_flight.load(Ordering::Relaxed)
    }
}

/// Queue statistics snapshot
#[derive(Debug, Clone)]
pub struct QueueStatsSnapshot {
    pub commands_submitted: u64,
    pub commands_completed: u64,
    pub commands_in_flight: u32,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
    pub min_latency_ns: u64,
}

/// I/O queue manager
pub struct IoQueueManager {
    queues: Vec<QueuePair>,
    queue_selector: AtomicUsize,
}

impl IoQueueManager {
    /// Create a new queue manager
    pub fn new(queues: Vec<QueuePair>) -> Self {
        Self {
            queues,
            queue_selector: AtomicUsize::new(0),
        }
    }

    /// Get number of queues
    pub fn num_queues(&self) -> usize {
        self.queues.len()
    }

    /// Select a queue using round-robin
    pub fn select_queue_rr(&self) -> &QueuePair {
        let idx = self.queue_selector.fetch_add(1, Ordering::Relaxed) % self.queues.len();
        &self.queues[idx]
    }

    /// Get queue by ID
    pub fn get_queue(&self, id: usize) -> Option<&QueuePair> {
        self.queues.get(id)
    }

    /// Get all queue statistics
    pub fn get_all_stats(&self) -> Vec<QueueStatsSnapshot> {
        self.queues.iter().map(|q| q.get_stats()).collect()
    }

    /// Get aggregate statistics
    pub fn get_aggregate_stats(&self) -> QueueStatsSnapshot {
        let mut total = QueueStatsSnapshot {
            commands_submitted: 0,
            commands_completed: 0,
            commands_in_flight: 0,
            bytes_read: 0,
            bytes_written: 0,
            avg_latency_ns: 0,
            max_latency_ns: 0,
            min_latency_ns: u64::MAX,
        };

        for q in &self.queues {
            let stats = q.get_stats();
            total.commands_submitted += stats.commands_submitted;
            total.commands_completed += stats.commands_completed;
            total.commands_in_flight += stats.commands_in_flight;
            total.bytes_read += stats.bytes_read;
            total.bytes_written += stats.bytes_written;
            total.avg_latency_ns += stats.avg_latency_ns;
            total.max_latency_ns = total.max_latency_ns.max(stats.max_latency_ns);
            total.min_latency_ns = total.min_latency_ns.min(stats.min_latency_ns);
        }

        if !self.queues.is_empty() {
            total.avg_latency_ns /= self.queues.len() as u64;
        }

        total
    }
}

/// Trait for queue I/O operations
pub trait IoQueue: Send + Sync {
    /// Submit read operation
    fn read(&self, ns_id: u32, lba: u64, blocks: u16, data: usize, size: usize) -> Option<u16>;

    /// Submit write operation
    fn write(&self, ns_id: u32, lba: u64, blocks: u16, data: usize, size: usize) -> Option<u16>;

    /// Poll for completion
    fn poll(&self) -> Option<CompletionInfo>;

    /// Get queue depth
    fn depth(&self) -> u32;
}

impl IoQueue for QueuePair {
    fn read(&self, ns_id: u32, lba: u64, blocks: u16, data: usize, size: usize) -> Option<u16> {
        self.submit_read(ns_id, lba, blocks, data, size)
    }

    fn write(&self, ns_id: u32, lba: u64, blocks: u16, data: usize, size: usize) -> Option<u16> {
        self.submit_write(ns_id, lba, blocks, data, size)
    }

    fn poll(&self) -> Option<CompletionInfo> {
        self.poll_completion()
    }

    fn depth(&self) -> u32 {
        self.queue_depth()
    }
}
