//! NVMe Controller Management

use super::queue::QueuePair;
use std::sync::Arc;

/// NVMe controller
pub struct Controller {
    /// Admin queue pair
    pub admin_queue: Arc<QueuePair>,
    /// I/O queue pairs (per-CPU)
    pub io_queues: Vec<Arc<QueuePair>>,
    /// Number of namespaces
    pub num_namespaces: u32,
}

impl Controller {
    /// Initialize controller
    pub fn new() -> Result<Self, &'static str> {
        log::info!("Initializing NVMe controller");

        // Create admin queue (ID 0)
        let admin_queue = Arc::new(QueuePair::new(0, 64, 0, 0));

        // Create I/O queues (one per CPU)
        let num_cpus = num_cpus::get();
        let mut io_queues = Vec::with_capacity(num_cpus);

        for i in 0..num_cpus {
            let queue = Arc::new(QueuePair::new((i + 1) as u16, 1024, 0, 0));
            io_queues.push(queue);
        }

        log::info!("Created {} I/O queues", num_cpus);

        Ok(Self {
            admin_queue,
            io_queues,
            num_namespaces: 1,
        })
    }

    /// Get I/O queue for current CPU
    pub fn current_io_queue(&self) -> &Arc<QueuePair> {
        let cpu_id = 0; // TODO: Get actual CPU ID
        &self.io_queues[cpu_id % self.io_queues.len()]
    }
}
