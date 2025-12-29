//! NVMe Queue Management
//!
//! Submission Queue (SQ) and Completion Queue (CQ) implementation

use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::Arc;

/// Queue pair (SQ + CQ)
pub struct QueuePair {
    /// Queue ID
    pub id: u16,
    /// Submission queue
    pub sq: SubmissionQueue,
    /// Completion queue
    pub cq: CompletionQueue,
}

/// Submission Queue
pub struct SubmissionQueue {
    /// Queue entries
    entries: Vec<SubmissionQueueEntry>,
    /// Queue depth
    depth: u16,
    /// Head pointer (hardware updates)
    head: AtomicU16,
    /// Tail pointer (software updates)
    tail: AtomicU16,
    /// Doorbell register address
    doorbell: usize,
}

/// Completion Queue
pub struct CompletionQueue {
    /// Queue entries
    entries: Vec<CompletionQueueEntry>,
    /// Queue depth
    depth: u16,
    /// Head pointer (software updates)
    head: AtomicU16,
    /// Tail pointer (hardware updates)
    tail: AtomicU16,
    /// Phase tag
    phase: AtomicU16,
}

/// Submission Queue Entry (64 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SubmissionQueueEntry {
    pub opcode: u8,
    pub flags: u8,
    pub command_id: u16,
    pub nsid: u32,
    pub reserved: [u32; 2],
    pub metadata: u64,
    pub prp1: u64,
    pub prp2: u64,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

/// Completion Queue Entry (16 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CompletionQueueEntry {
    pub dword0: u32,
    pub dword1: u32,
    pub sq_head: u16,
    pub sq_id: u16,
    pub command_id: u16,
    pub status: u16,
}

impl QueuePair {
    /// Create new queue pair
    pub fn new(id: u16, depth: u16, sq_doorbell: usize, cq_doorbell: usize) -> Self {
        log::info!("Creating queue pair {} with depth {}", id, depth);

        Self {
            id,
            sq: SubmissionQueue::new(depth, sq_doorbell),
            cq: CompletionQueue::new(depth),
        }
    }

    /// Submit command
    pub fn submit(&self, entry: SubmissionQueueEntry) -> Result<u16, &'static str> {
        self.sq.submit(entry)
    }

    /// Poll for completions
    pub fn poll(&self) -> Option<CompletionQueueEntry> {
        self.cq.poll()
    }
}

impl SubmissionQueue {
    fn new(depth: u16, doorbell: usize) -> Self {
        let entries = vec![unsafe { std::mem::zeroed() }; depth as usize];

        Self {
            entries,
            depth,
            head: AtomicU16::new(0),
            tail: AtomicU16::new(0),
            doorbell,
        }
    }

    fn submit(&self, entry: SubmissionQueueEntry) -> Result<u16, &'static str> {
        let tail = self.tail.load(Ordering::Acquire);
        let next_tail = (tail + 1) % self.depth;
        let head = self.head.load(Ordering::Acquire);

        // Check if queue is full
        if next_tail == head {
            return Err("Queue full");
        }

        // Write entry
        unsafe {
            let ptr = self.entries.as_ptr() as *mut SubmissionQueueEntry;
            ptr.add(tail as usize).write_volatile(entry);
        }

        // Update tail
        self.tail.store(next_tail, Ordering::Release);

        // Ring doorbell
        unsafe {
            std::ptr::write_volatile(self.doorbell as *mut u32, next_tail as u32);
        }

        Ok(entry.command_id)
    }
}

impl CompletionQueue {
    fn new(depth: u16) -> Self {
        let entries = vec![unsafe { std::mem::zeroed() }; depth as usize];

        Self {
            entries,
            depth,
            head: AtomicU16::new(0),
            tail: AtomicU16::new(0),
            phase: AtomicU16::new(1),
        }
    }

    fn poll(&self) -> Option<CompletionQueueEntry> {
        let head = self.head.load(Ordering::Acquire);
        let phase = self.phase.load(Ordering::Acquire);

        // Read entry
        let entry = unsafe {
            let ptr = self.entries.as_ptr();
            ptr.add(head as usize).read_volatile()
        };

        // Check phase bit
        let entry_phase = (entry.status >> 15) & 1;
        if entry_phase != phase {
            return None; // No new completion
        }

        // Update head
        let next_head = (head + 1) % self.depth;
        self.head.store(next_head, Ordering::Release);

        // Flip phase if wrapped
        if next_head == 0 {
            self.phase.store(1 - phase, Ordering::Release);
        }

        Some(entry)
    }
}

/// NVMe opcodes
pub mod opcodes {
    pub const FLUSH: u8 = 0x00;
    pub const WRITE: u8 = 0x01;
    pub const READ: u8 = 0x02;
    pub const WRITE_UNCORRECTABLE: u8 = 0x04;
    pub const COMPARE: u8 = 0x05;
    pub const WRITE_ZEROES: u8 = 0x08;
    pub const DATASET_MANAGEMENT: u8 = 0x09;
}
