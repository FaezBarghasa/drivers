//! NPU Command Queue
//!
//! Async command submission and execution for NPU operations.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, RwLock};

/// Command types
#[derive(Debug, Clone)]
pub enum CommandType {
    /// Copy data to device
    CopyToDevice { src: usize, dst: u32, size: usize },
    /// Copy data from device
    CopyFromDevice { src: u32, dst: usize, size: usize },
    /// Execute inference
    Inference {
        model_id: u32,
        input: u32,
        output: u32,
    },
    /// Matrix multiply
    MatMul {
        a: u32,
        b: u32,
        c: u32,
        m: u32,
        n: u32,
        k: u32,
    },
    /// Convolution
    Conv2d {
        input: u32,
        kernel: u32,
        output: u32,
    },
    /// Activation function
    Activation { buffer: u32, func: ActivationFunc },
    /// Sync/barrier
    Barrier,
}

/// Activation functions
#[derive(Debug, Clone, Copy)]
pub enum ActivationFunc {
    ReLU,
    Sigmoid,
    Tanh,
    GeLU,
    SiLU,
    Softmax,
}

/// Command status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Pending,
    Running,
    Completed,
    Failed(u32),
}

/// NPU command
#[derive(Debug, Clone)]
pub struct Command {
    pub id: u64,
    pub cmd_type: CommandType,
    pub priority: u32,
    pub status: CommandStatus,
}

impl Command {
    pub fn new(cmd_type: CommandType) -> Self {
        Self {
            id: 0,
            cmd_type,
            priority: 0,
            status: CommandStatus::Pending,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// Command queue for async execution
pub struct CommandQueue {
    /// Pending commands
    pending: Mutex<VecDeque<Command>>,
    /// In-flight commands
    in_flight: RwLock<Vec<Command>>,
    /// Completed commands (for status lookup)
    completed: RwLock<Vec<(u64, CommandStatus)>>,
    /// Next command ID
    next_id: AtomicU64,
    /// Queue capacity
    capacity: usize,
    /// Condition variable for waiters
    completion: Condvar,
}

impl CommandQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            pending: Mutex::new(VecDeque::with_capacity(capacity)),
            in_flight: RwLock::new(Vec::new()),
            completed: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(1),
            capacity,
            completion: Condvar::new(),
        }
    }

    /// Submit a command for execution
    pub fn submit(&self, mut cmd: Command) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        cmd.id = id;
        cmd.status = CommandStatus::Pending;

        let mut pending = self.pending.lock().unwrap();

        // Insert sorted by priority (higher priority first)
        let pos = pending
            .iter()
            .position(|c| c.priority < cmd.priority)
            .unwrap_or(pending.len());
        pending.insert(pos, cmd);

        id
    }

    /// Get the next pending command
    pub fn dequeue(&self) -> Option<Command> {
        let mut pending = self.pending.lock().unwrap();
        pending.pop_front().map(|mut cmd| {
            cmd.status = CommandStatus::Running;
            self.in_flight.write().unwrap().push(cmd.clone());
            cmd
        })
    }

    /// Mark a command as completed
    pub fn complete(&self, id: u64, status: CommandStatus) {
        // Remove from in-flight
        self.in_flight.write().unwrap().retain(|c| c.id != id);

        // Add to completed
        let mut completed = self.completed.write().unwrap();
        completed.push((id, status));

        // Keep only last 1000 completions
        if completed.len() > 1000 {
            completed.drain(0..500);
        }

        // Notify waiters
        self.completion.notify_all();
    }

    /// Wait for a command to complete
    pub fn wait(&self, id: u64) -> CommandStatus {
        // Check if already completed
        if let Some(status) = self.get_status(id) {
            if status != CommandStatus::Pending && status != CommandStatus::Running {
                return status;
            }
        }

        // Wait for completion
        let pending = self.pending.lock().unwrap();
        let _guard = self
            .completion
            .wait_while(pending, |_| {
                self.get_status(id)
                    .map(|s| s == CommandStatus::Pending || s == CommandStatus::Running)
                    .unwrap_or(false)
            })
            .unwrap();

        self.get_status(id).unwrap_or(CommandStatus::Failed(1))
    }

    /// Get command status
    pub fn get_status(&self, id: u64) -> Option<CommandStatus> {
        // Check completed
        if let Some((_, status)) = self
            .completed
            .read()
            .unwrap()
            .iter()
            .find(|(i, _)| *i == id)
        {
            return Some(*status);
        }

        // Check in-flight
        if self.in_flight.read().unwrap().iter().any(|c| c.id == id) {
            return Some(CommandStatus::Running);
        }

        // Check pending
        if self.pending.lock().unwrap().iter().any(|c| c.id == id) {
            return Some(CommandStatus::Pending);
        }

        None
    }

    /// Get queue depth
    pub fn pending_count(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

    /// Get in-flight count
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.read().unwrap().len()
    }
}
