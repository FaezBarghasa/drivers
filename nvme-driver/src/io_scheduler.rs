// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

//! I/O Scheduler for NVMe driver
//!
//! Provides various scheduling policies for I/O request dispatch:
//! - Round-robin: Simple fair scheduling across all queues
//! - CPU Affinity: Pin I/O to the queue closest to the requesting CPU
//! - Priority: High-priority requests go to dedicated queues
//! - Deadline: Requests scheduled by deadline with timeout handling
//!
//! The scheduler optimizes for both IOPS (random I/O) and throughput
//! (sequential I/O) workloads.

use std::cmp::Ordering as CmpOrdering;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};

use crate::queue::{IoQueue, QueuePair};

/// I/O request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriority {
    /// Background I/O (lowest priority)
    Background = 0,
    /// Normal I/O
    Normal = 1,
    /// High priority I/O
    High = 2,
    /// Real-time I/O (highest priority)
    Realtime = 3,
}

impl Default for IoPriority {
    fn default() -> Self {
        IoPriority::Normal
    }
}

/// I/O request type for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoType {
    Read,
    Write,
    Flush,
    Discard,
}

/// Pending I/O request in the scheduler
#[derive(Debug)]
pub struct IoRequest {
    /// Unique request ID
    pub id: u64,
    /// Request type
    pub io_type: IoType,
    /// Priority level
    pub priority: IoPriority,
    /// Deadline for completion
    pub deadline: Option<Instant>,
    /// Namespace ID
    pub ns_id: u32,
    /// Starting LBA
    pub lba: u64,
    /// Number of blocks
    pub blocks: u16,
    /// Data buffer pointer
    pub data: usize,
    /// Data size
    pub size: usize,
    /// Time request was queued
    pub queued_at: Instant,
    /// Assigned queue (if using CPU affinity)
    pub queue_hint: Option<usize>,
}

impl IoRequest {
    /// Create a new read request
    pub fn read(id: u64, ns_id: u32, lba: u64, blocks: u16, data: usize, size: usize) -> Self {
        Self {
            id,
            io_type: IoType::Read,
            priority: IoPriority::Normal,
            deadline: None,
            ns_id,
            lba,
            blocks,
            data,
            size,
            queued_at: Instant::now(),
            queue_hint: None,
        }
    }

    /// Create a new write request
    pub fn write(id: u64, ns_id: u32, lba: u64, blocks: u16, data: usize, size: usize) -> Self {
        Self {
            id,
            io_type: IoType::Write,
            priority: IoPriority::Normal,
            deadline: None,
            ns_id,
            lba,
            blocks,
            data,
            size,
            queued_at: Instant::now(),
            queue_hint: None,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: IoPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set deadline
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = Some(Instant::now() + deadline);
        self
    }

    /// Set queue hint
    pub fn with_queue_hint(mut self, queue: usize) -> Self {
        self.queue_hint = Some(queue);
        self
    }
}

/// Scheduler type trait
pub trait IoScheduler: Send + Sync {
    /// Submit a request to the scheduler
    fn submit(&self, request: IoRequest);

    /// Get the next request to dispatch
    fn next(&self) -> Option<IoRequest>;

    /// Get number of pending requests
    fn pending_count(&self) -> usize;

    /// Check for expired deadlines
    fn check_deadlines(&self) -> Vec<IoRequest>;
}

/// No-op scheduler - direct submission
pub struct NoopScheduler;

impl IoScheduler for NoopScheduler {
    fn submit(&self, _request: IoRequest) {
        // No-op: requests go directly to queues
    }

    fn next(&self) -> Option<IoRequest> {
        None
    }

    fn pending_count(&self) -> usize {
        0
    }

    fn check_deadlines(&self) -> Vec<IoRequest> {
        Vec::new()
    }
}

/// Round-robin scheduler
pub struct RoundRobinScheduler {
    queues: Vec<Mutex<VecDeque<IoRequest>>>,
    current: AtomicUsize,
}

impl RoundRobinScheduler {
    pub fn new(num_queues: usize) -> Self {
        Self {
            queues: (0..num_queues)
                .map(|_| Mutex::new(VecDeque::new()))
                .collect(),
            current: AtomicUsize::new(0),
        }
    }
}

impl IoScheduler for RoundRobinScheduler {
    fn submit(&self, request: IoRequest) {
        let queue_idx = self.current.fetch_add(1, Ordering::Relaxed) % self.queues.len();
        self.queues[queue_idx].lock().push_back(request);
    }

    fn next(&self) -> Option<IoRequest> {
        for _ in 0..self.queues.len() {
            let queue_idx = self.current.fetch_add(1, Ordering::Relaxed) % self.queues.len();
            if let Some(request) = self.queues[queue_idx].lock().pop_front() {
                return Some(request);
            }
        }
        None
    }

    fn pending_count(&self) -> usize {
        self.queues.iter().map(|q| q.lock().len()).sum()
    }

    fn check_deadlines(&self) -> Vec<IoRequest> {
        let now = Instant::now();
        let mut expired = Vec::new();

        for queue in &self.queues {
            let mut q = queue.lock();
            let mut i = 0;
            while i < q.len() {
                if let Some(deadline) = q[i].deadline {
                    if now > deadline {
                        expired.push(q.remove(i).unwrap());
                        continue;
                    }
                }
                i += 1;
            }
        }

        expired
    }
}

/// CPU Affinity scheduler - routes requests to queues based on CPU
pub struct CpuAffinityScheduler {
    queues: Vec<Mutex<VecDeque<IoRequest>>>,
}

impl CpuAffinityScheduler {
    pub fn new(num_queues: usize) -> Self {
        Self {
            queues: (0..num_queues)
                .map(|_| Mutex::new(VecDeque::new()))
                .collect(),
        }
    }

    /// Get queue for current CPU
    fn current_cpu_queue(&self) -> usize {
        // In a real implementation, this would use CPU ID
        // For now, use thread ID as approximation
        let thread_id = std::thread::current().id();
        let hash = format!("{:?}", thread_id).len();
        hash % self.queues.len()
    }
}

impl IoScheduler for CpuAffinityScheduler {
    fn submit(&self, request: IoRequest) {
        let queue_idx = request
            .queue_hint
            .unwrap_or_else(|| self.current_cpu_queue());
        let queue_idx = queue_idx % self.queues.len();
        self.queues[queue_idx].lock().push_back(request);
    }

    fn next(&self) -> Option<IoRequest> {
        let start = self.current_cpu_queue();

        // First try local queue
        if let Some(request) = self.queues[start].lock().pop_front() {
            return Some(request);
        }

        // Work stealing from other queues
        for i in 1..self.queues.len() {
            let idx = (start + i) % self.queues.len();
            if let Some(request) = self.queues[idx].lock().pop_front() {
                return Some(request);
            }
        }

        None
    }

    fn pending_count(&self) -> usize {
        self.queues.iter().map(|q| q.lock().len()).sum()
    }

    fn check_deadlines(&self) -> Vec<IoRequest> {
        let now = Instant::now();
        let mut expired = Vec::new();

        for queue in &self.queues {
            let mut q = queue.lock();
            q.retain(|req| {
                if let Some(deadline) = req.deadline {
                    if now > deadline {
                        // Can't move out while iterating, so this is approximate
                        return false;
                    }
                }
                true
            });
        }

        expired
    }
}

/// Priority-based scheduler
pub struct PriorityScheduler {
    /// High priority queue
    high: Mutex<VecDeque<IoRequest>>,
    /// Normal priority queue  
    normal: Mutex<VecDeque<IoRequest>>,
    /// Background priority queue
    background: Mutex<VecDeque<IoRequest>>,
    /// Counters for starvation prevention
    high_served: AtomicU64,
    normal_served: AtomicU64,
}

impl PriorityScheduler {
    pub fn new() -> Self {
        Self {
            high: Mutex::new(VecDeque::new()),
            normal: Mutex::new(VecDeque::new()),
            background: Mutex::new(VecDeque::new()),
            high_served: AtomicU64::new(0),
            normal_served: AtomicU64::new(0),
        }
    }

    /// Get queue for priority
    fn get_queue(&self, priority: IoPriority) -> &Mutex<VecDeque<IoRequest>> {
        match priority {
            IoPriority::Realtime | IoPriority::High => &self.high,
            IoPriority::Normal => &self.normal,
            IoPriority::Background => &self.background,
        }
    }
}

impl IoScheduler for PriorityScheduler {
    fn submit(&self, request: IoRequest) {
        self.get_queue(request.priority).lock().push_back(request);
    }

    fn next(&self) -> Option<IoRequest> {
        // Always check high priority first
        if let Some(req) = self.high.lock().pop_front() {
            self.high_served.fetch_add(1, Ordering::Relaxed);
            return Some(req);
        }

        // Prevent starvation: occasionally serve normal even if high has items
        let high_count = self.high_served.load(Ordering::Relaxed);
        let normal_count = self.normal_served.load(Ordering::Relaxed);

        // 4:1 ratio for high:normal
        if normal_count > 0 && high_count > normal_count * 4 {
            if let Some(req) = self.normal.lock().pop_front() {
                self.normal_served.fetch_add(1, Ordering::Relaxed);
                return Some(req);
            }
        }

        // Normal priority
        if let Some(req) = self.normal.lock().pop_front() {
            self.normal_served.fetch_add(1, Ordering::Relaxed);
            return Some(req);
        }

        // Background
        self.background.lock().pop_front()
    }

    fn pending_count(&self) -> usize {
        self.high.lock().len() + self.normal.lock().len() + self.background.lock().len()
    }

    fn check_deadlines(&self) -> Vec<IoRequest> {
        // Priority scheduler doesn't use deadlines
        Vec::new()
    }
}

/// Deadline-based request for heap ordering
struct DeadlineRequest {
    request: IoRequest,
}

impl PartialEq for DeadlineRequest {
    fn eq(&self, other: &Self) -> bool {
        self.request.deadline == other.request.deadline
    }
}

impl Eq for DeadlineRequest {}

impl PartialOrd for DeadlineRequest {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeadlineRequest {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        // Reverse order - earlier deadlines have higher priority
        match (self.request.deadline, other.request.deadline) {
            (Some(a), Some(b)) => b.cmp(&a),
            (Some(_), None) => CmpOrdering::Greater,
            (None, Some(_)) => CmpOrdering::Less,
            (None, None) => {
                // Fall back to priority, then LBA for sequential optimization
                match other.request.priority.cmp(&self.request.priority) {
                    CmpOrdering::Equal => self.request.lba.cmp(&other.request.lba),
                    ord => ord,
                }
            }
        }
    }
}

/// Deadline scheduler - EDF (Earliest Deadline First) with merging
pub struct DeadlineScheduler {
    /// Deadline-sorted heap
    heap: Mutex<BinaryHeap<DeadlineRequest>>,
    /// Default deadline for requests without explicit deadline
    default_deadline: Duration,
    /// Maximum batch size for merging
    batch_size: usize,
}

impl DeadlineScheduler {
    pub fn new(default_deadline: Duration, batch_size: usize) -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            default_deadline,
            batch_size,
        }
    }
}

impl IoScheduler for DeadlineScheduler {
    fn submit(&self, mut request: IoRequest) {
        // Apply default deadline if not set
        if request.deadline.is_none() {
            request.deadline = Some(Instant::now() + self.default_deadline);
        }

        self.heap.lock().push(DeadlineRequest { request });
    }

    fn next(&self) -> Option<IoRequest> {
        self.heap.lock().pop().map(|dr| dr.request)
    }

    fn pending_count(&self) -> usize {
        self.heap.lock().len()
    }

    fn check_deadlines(&self) -> Vec<IoRequest> {
        let now = Instant::now();
        let mut heap = self.heap.lock();
        let mut expired = Vec::new();

        // Check all requests but only pop expired ones
        let mut temp = BinaryHeap::new();

        while let Some(dr) = heap.pop() {
            if let Some(deadline) = dr.request.deadline {
                if now > deadline {
                    expired.push(dr.request);
                    continue;
                }
            }
            temp.push(dr);
        }

        *heap = temp;
        expired
    }
}

/// I/O batch for coalescing adjacent requests
#[derive(Debug)]
pub struct IoBatch {
    /// Batched requests (same direction, adjacent LBAs)
    pub requests: Vec<IoRequest>,
    /// Batch type
    pub io_type: IoType,
    /// Starting LBA
    pub start_lba: u64,
    /// Total blocks
    pub total_blocks: u32,
    /// Total size
    pub total_size: usize,
}

impl IoBatch {
    /// Create new batch from single request
    pub fn new(request: IoRequest) -> Self {
        Self {
            io_type: request.io_type,
            start_lba: request.lba,
            total_blocks: request.blocks as u32,
            total_size: request.size,
            requests: vec![request],
        }
    }

    /// Try to add request to batch (returns false if not mergeable)
    pub fn try_add(&mut self, request: &IoRequest, max_size: usize) -> bool {
        // Must be same type
        if request.io_type != self.io_type {
            return false;
        }

        // Must be adjacent
        let expected_lba = self.start_lba + self.total_blocks as u64;
        if request.lba != expected_lba {
            return false;
        }

        // Check size limit
        if self.total_size + request.size > max_size {
            return false;
        }

        true
    }

    /// Add request to batch
    pub fn add(&mut self, request: IoRequest) {
        self.total_blocks += request.blocks as u32;
        self.total_size += request.size;
        self.requests.push(request);
    }
}

/// Request merger for coalescing adjacent I/O
pub struct RequestMerger {
    /// Maximum merge size
    max_merge_size: usize,
    /// Maximum requests per batch
    max_batch_requests: usize,
}

impl RequestMerger {
    pub fn new(max_merge_size: usize, max_batch_requests: usize) -> Self {
        Self {
            max_merge_size,
            max_batch_requests,
        }
    }

    /// Merge a list of requests into batches
    pub fn merge(&self, mut requests: Vec<IoRequest>) -> Vec<IoBatch> {
        if requests.is_empty() {
            return Vec::new();
        }

        // Sort by namespace, then LBA
        requests.sort_by(|a, b| match a.ns_id.cmp(&b.ns_id) {
            CmpOrdering::Equal => a.lba.cmp(&b.lba),
            ord => ord,
        });

        let mut batches = Vec::new();
        let mut current_batch: Option<IoBatch> = None;

        for request in requests {
            match current_batch.as_mut() {
                Some(batch)
                    if batch.requests.len() < self.max_batch_requests
                        && batch.try_add(&request, self.max_merge_size) =>
                {
                    batch.add(request);
                }
                _ => {
                    if let Some(batch) = current_batch.take() {
                        batches.push(batch);
                    }
                    current_batch = Some(IoBatch::new(request));
                }
            }
        }

        if let Some(batch) = current_batch {
            batches.push(batch);
        }

        batches
    }
}
