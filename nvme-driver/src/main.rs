// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

//! High-Performance Async NVMe Driver Server
//!
//! This is a top-tier, low-latency NVMe driver that delivers maximum IOPS
//! and sequential throughput through:
//!
//! - Multi-Core/Multi-Queue I/O processing
//! - Asynchronous command submission and completion
//! - Zero-copy data transfer support
//! - Performance counter instrumentation
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    User Applications                             │
//! └──────────────────────────┬──────────────────────────────────────┘
//!                            │ nvme:N scheme requests
//! ┌──────────────────────────▼──────────────────────────────────────┐
//! │                    NVMe Driver Server                            │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │                  Request Router                          │    │
//! │  │  • CPU affinity-based queue selection                   │    │
//! │  │  • IO priority handling                                 │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! │                            │                                     │
//! │  ┌─────────────────────────▼───────────────────────────────┐    │
//! │  │              Per-CPU Queue Pairs                         │    │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │    │
//! │  │  │ SQ0/CQ0 │ │ SQ1/CQ1 │ │ SQ2/CQ2 │ │ SQ3/CQ3 │ ...    │    │
//! │  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘        │    │
//! │  └───────┼───────────┼───────────┼───────────┼─────────────┘    │
//! │          │           │           │           │                   │
//! │  ┌───────▼───────────▼───────────▼───────────▼─────────────┐    │
//! │  │              Completion Handler (IRQ/Poll)               │    │
//! │  │  • Per-queue interrupt threads                          │    │
//! │  │  • Polling mode for high-IOPS workloads                 │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! └──────────────────────────┬──────────────────────────────────────┘
//!                            │ PCIe / Memory-mapped I/O
//! ┌──────────────────────────▼──────────────────────────────────────┐
//! │                    NVMe Controller Hardware                      │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

#![feature(map_try_insert)]

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::bail;
use log::{debug, error, info, trace, warn};
use parking_lot::RwLock;
use redox_event::{EventFlags, EventQueue};
use redox_log;
use redox_rt::signal::block_all_signals;
use spin::Mutex;

mod benchmark;
mod io_scheduler;
mod queue;
mod scheme;
mod stats;

use crate::scheme::NvmeScheme;
use crate::stats::PerformanceStats;

/// Global performance statistics
static GLOBAL_STATS: PerformanceStats = PerformanceStats::new();

/// Driver configuration
#[derive(Debug, Clone)]
pub struct DriverConfig {
    /// Number of I/O queues (0 = auto-detect based on CPU count)
    pub num_queues: usize,
    /// Queue depth (commands per queue)
    pub queue_depth: u16,
    /// Enable polling mode instead of interrupts
    pub polling_mode: bool,
    /// Polling interval in microseconds
    pub poll_interval_us: u64,
    /// Enable zero-copy mode
    pub zero_copy: bool,
    /// Enable NUMA-aware queue allocation
    pub numa_aware: bool,
    /// Maximum concurrent commands per queue
    pub max_concurrent_cmds: u32,
    /// I/O scheduler type
    pub scheduler: IoSchedulerType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedulerType {
    /// No scheduling, direct submission
    None,
    /// Round-robin across queues
    RoundRobin,
    /// CPU affinity-based
    CpuAffinity,
    /// Priority-based
    Priority,
    /// Deadline-based
    Deadline,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            num_queues: 0, // Auto-detect
            queue_depth: 1024,
            polling_mode: false,
            poll_interval_us: 10,
            zero_copy: true,
            numa_aware: true,
            max_concurrent_cmds: 256,
            scheduler: IoSchedulerType::CpuAffinity,
        }
    }
}

fn main() {
    redox_log::init!();
    info!(
        "Starting high-performance NVMe driver v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Parse configuration from environment or defaults
    let config = parse_config();
    info!("Configuration: {:?}", config);

    // Open PCI device
    let pci_handle = unsafe {
        libredox::call::open(
            "pci:class=010802",
            libredox::flag::O_RDWR | libredox::flag::O_CLOEXEC,
        )
        .expect("nvme: failed to open pci")
    };

    // Read PCI configuration space
    let mut pci_config = [0u8; 4096];
    let pci_config_len =
        libredox::call::read(pci_handle, &mut pci_config).expect("nvme: failed to read pci config");

    info!("PCI config read {} bytes", pci_config_len);

    // Create scheme file
    let scheme_file = libredox::call::open(
        "nvme:",
        libredox::flag::O_RDWR | libredox::flag::O_CREAT | libredox::flag::O_CLOEXEC,
    )
    .expect("nvme: failed to create nvme scheme");

    // Initialize the NVMe scheme with multi-queue support
    let scheme = NvmeScheme::new(pci_handle, &pci_config[..pci_config_len], &config)
        .expect("nvme: failed to create scheme");

    let scheme_fd = scheme_file as RawFd;
    let scheme = Arc::new(RwLock::new(scheme));

    // Spawn per-queue interrupt/polling threads
    let num_queues = {
        let s = scheme.read();
        s.queues.len()
    };

    info!("Created {} I/O queue pairs", num_queues);

    let mut queue_threads = Vec::new();

    for queue_id in 0..num_queues {
        let scheme_clone = Arc::clone(&scheme);
        let config_clone = config.clone();

        let handle = thread::Builder::new()
            .name(format!("nvme-queue-{}", queue_id))
            .spawn(move || {
                queue_worker(queue_id, scheme_clone, config_clone);
            })
            .expect("nvme: failed to spawn queue worker thread");

        queue_threads.push(handle);
    }

    // Spawn statistics reporter thread
    #[cfg(feature = "performance-counters")]
    {
        thread::Builder::new()
            .name("nvme-stats".to_string())
            .spawn(move || {
                stats_reporter();
            })
            .expect("nvme: failed to spawn stats thread");
    }

    // Main scheme event loop
    let mut event_queue = EventQueue::new().expect("nvme: failed to create event queue");
    let scheme_token = 1;
    event_queue
        .subscribe(scheme_fd as usize, scheme_token, EventFlags::READ)
        .expect("nvme: failed to subscribe to scheme events");

    let scheme_for_event = Arc::clone(&scheme);

    info!("NVMe driver ready, entering event loop");

    for event_res in event_queue {
        let _ = event_res.expect("nvme: event queue error");

        // Handle scheme events
        let mut packets = Vec::with_capacity(64);

        loop {
            let mut packet = libredox::Packet::default();
            match syscall::read(scheme_fd, &mut packet) {
                Ok(0) => break,
                Ok(_) => packets.push(packet),
                Err(err) if err.errno == syscall::EAGAIN => break,
                Err(err) => {
                    error!("failed to read scheme: {}", err);
                    break;
                }
            }
        }

        // Batch process packets
        if !packets.is_empty() {
            let start = Instant::now();
            let count = packets.len();

            for mut packet in packets {
                scheme_for_event.write().handle(&mut packet);
                let _ = syscall::write(scheme_fd, &packet);
            }

            #[cfg(feature = "performance-counters")]
            {
                GLOBAL_STATS.record_batch(count, start.elapsed());
            }
        }
    }
}

/// Queue worker thread - handles completions for a specific queue
fn queue_worker(queue_id: usize, scheme: Arc<RwLock<NvmeScheme>>, config: DriverConfig) {
    if config.polling_mode {
        polling_worker(queue_id, scheme, config);
    } else {
        interrupt_worker(queue_id, scheme);
    }
}

/// Interrupt-based completion handling
fn interrupt_worker(queue_id: usize, scheme: Arc<RwLock<NvmeScheme>>) {
    let mut event_queue = EventQueue::new().expect("nvme: failed to create event queue");

    // Open IRQ file for this queue
    let irq_number = {
        let s = scheme.read();
        s.get_queue_irq(queue_id)
    };

    let irq_file =
        File::open(format!("irq:{}", irq_number)).expect("nvme: failed to open irq file");
    let irq_fd = irq_file.into_raw_fd();
    let irq_token = 1;

    event_queue
        .subscribe(irq_fd as usize, irq_token, EventFlags::READ)
        .expect("nvme: failed to subscribe to irq events");

    let scheme_clone = Arc::clone(&scheme);

    for event_res in event_queue {
        let _ = event_res.expect("nvme: queue event loop failed");

        let mut irq_buf = [0u8; 8];
        let bytes = syscall::read(irq_fd, &mut irq_buf).expect("nvme: failed to read irq file");

        if bytes == 8 {
            let start = Instant::now();
            let completions = scheme_clone.write().process_completions(queue_id);

            #[cfg(feature = "performance-counters")]
            if completions > 0 {
                GLOBAL_STATS.record_completions(completions, start.elapsed());
            }

            // Acknowledge interrupt
            let _ = syscall::write(irq_fd, &irq_buf);
        }
    }
}

/// Polling-based completion handling (for ultra-low latency)
fn polling_worker(queue_id: usize, scheme: Arc<RwLock<NvmeScheme>>, config: DriverConfig) {
    let poll_interval = Duration::from_micros(config.poll_interval_us);

    loop {
        let start = Instant::now();
        let completions = scheme.write().process_completions(queue_id);

        #[cfg(feature = "performance-counters")]
        if completions > 0 {
            GLOBAL_STATS.record_completions(completions, start.elapsed());
        }

        if completions == 0 {
            // No work, sleep briefly
            thread::sleep(poll_interval);
        }
    }
}

/// Statistics reporter thread
#[cfg(feature = "performance-counters")]
fn stats_reporter() {
    let interval = Duration::from_secs(10);

    loop {
        thread::sleep(interval);

        let stats = GLOBAL_STATS.snapshot();

        info!("NVMe Performance Stats:");
        info!("  Read IOPS:      {:>12}", stats.read_iops);
        info!("  Write IOPS:     {:>12}", stats.write_iops);
        info!("  Read MB/s:      {:>12.2}", stats.read_mbps);
        info!("  Write MB/s:     {:>12.2}", stats.write_mbps);
        info!("  Avg Latency:    {:>12?}", stats.avg_latency);
        info!("  P99 Latency:    {:>12?}", stats.p99_latency);
        info!("  Total Commands: {:>12}", stats.total_commands);
        info!("  Queue Depth:    {:>12}", stats.current_queue_depth);
    }
}

/// Parse configuration from environment
fn parse_config() -> DriverConfig {
    let mut config = DriverConfig::default();

    if let Ok(val) = std::env::var("NVME_NUM_QUEUES") {
        if let Ok(n) = val.parse() {
            config.num_queues = n;
        }
    }

    if let Ok(val) = std::env::var("NVME_QUEUE_DEPTH") {
        if let Ok(n) = val.parse() {
            config.queue_depth = n;
        }
    }

    if let Ok(val) = std::env::var("NVME_POLLING_MODE") {
        config.polling_mode = val == "1" || val.to_lowercase() == "true";
    }

    if let Ok(val) = std::env::var("NVME_POLL_INTERVAL_US") {
        if let Ok(n) = val.parse() {
            config.poll_interval_us = n;
        }
    }

    if let Ok(val) = std::env::var("NVME_ZERO_COPY") {
        config.zero_copy = val == "1" || val.to_lowercase() == "true";
    }

    if let Ok(val) = std::env::var("NVME_SCHEDULER") {
        config.scheduler = match val.to_lowercase().as_str() {
            "none" => IoSchedulerType::None,
            "roundrobin" => IoSchedulerType::RoundRobin,
            "cpu" | "cpuaffinity" => IoSchedulerType::CpuAffinity,
            "priority" => IoSchedulerType::Priority,
            "deadline" => IoSchedulerType::Deadline,
            _ => IoSchedulerType::CpuAffinity,
        };
    }

    config
}
