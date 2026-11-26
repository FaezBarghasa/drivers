// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

//! Performance statistics and monitoring
//!
//! Provides real-time IOPS, throughput, and latency metrics.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Global performance statistics instance
pub static GLOBAL_STATS: PerformanceStats = PerformanceStats::new();

/// Latency histogram bucket boundaries (in nanoseconds)
const LATENCY_BUCKETS: [u64; 16] = [
    1_000,      // 1 μs
    2_000,      // 2 μs
    4_000,      // 4 μs
    8_000,      // 8 μs
    16_000,     // 16 μs
    32_000,     // 32 μs
    64_000,     // 64 μs
    128_000,    // 128 μs
    256_000,    // 256 μs
    512_000,    // 512 μs
    1_000_000,  // 1 ms
    2_000_000,  // 2 ms
    4_000_000,  // 4 ms
    8_000_000,  // 8 ms
    16_000_000, // 16 ms
    u64::MAX,   // 16+ ms
];

/// Performance statistics collector
pub struct PerformanceStats {
    // Counters
    read_ops: AtomicU64,
    write_ops: AtomicU64,
    read_bytes: AtomicU64,
    write_bytes: AtomicU64,

    // Latency tracking
    total_latency_ns: AtomicU64,
    min_latency_ns: AtomicU64,
    max_latency_ns: AtomicU64,

    // Latency histogram buckets
    latency_histogram: [AtomicU64; 16],

    // Queue depth tracking
    current_queue_depth: AtomicU32,
    max_queue_depth: AtomicU32,

    // Error tracking
    errors: AtomicU64,
    timeouts: AtomicU64,

    // Rate calculation
    last_snapshot: Mutex<Option<StatsSnapshot>>,

    // Batch processing stats
    batch_count: AtomicU64,
    batch_total_size: AtomicU64,
    batch_latency_ns: AtomicU64,
}

/// Statistics snapshot at a point in time
#[derive(Clone)]
struct StatsSnapshot {
    time: Instant,
    read_ops: u64,
    write_ops: u64,
    read_bytes: u64,
    write_bytes: u64,
}

/// Human-readable stats report
#[derive(Debug, Clone)]
pub struct StatsReport {
    pub read_iops: u64,
    pub write_iops: u64,
    pub read_mbps: f64,
    pub write_mbps: f64,
    pub avg_latency: Duration,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub p50_latency: Duration,
    pub p99_latency: Duration,
    pub p999_latency: Duration,
    pub total_commands: u64,
    pub total_bytes: u64,
    pub current_queue_depth: u32,
    pub max_queue_depth: u32,
    pub errors: u64,
    pub timeouts: u64,
}

impl PerformanceStats {
    /// Create new stats collector
    pub const fn new() -> Self {
        Self {
            read_ops: AtomicU64::new(0),
            write_ops: AtomicU64::new(0),
            read_bytes: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            min_latency_ns: AtomicU64::new(u64::MAX),
            max_latency_ns: AtomicU64::new(0),
            latency_histogram: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            current_queue_depth: AtomicU32::new(0),
            max_queue_depth: AtomicU32::new(0),
            errors: AtomicU64::new(0),
            timeouts: AtomicU64::new(0),
            last_snapshot: Mutex::new(None),
            batch_count: AtomicU64::new(0),
            batch_total_size: AtomicU64::new(0),
            batch_latency_ns: AtomicU64::new(0),
        }
    }

    /// Record I/O submission
    pub fn record_io_submit(&self, bytes: usize, is_write: bool) {
        if is_write {
            self.write_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        } else {
            self.read_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        }

        // Update queue depth
        let depth = self.current_queue_depth.fetch_add(1, Ordering::Relaxed) + 1;

        // Update max queue depth
        let mut current_max = self.max_queue_depth.load(Ordering::Relaxed);
        while depth > current_max {
            match self.max_queue_depth.compare_exchange_weak(
                current_max,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    /// Record I/O completion
    pub fn record_io_complete(&self, bytes: usize, is_write: bool, latency: Duration) {
        if is_write {
            self.write_ops.fetch_add(1, Ordering::Relaxed);
        } else {
            self.read_ops.fetch_add(1, Ordering::Relaxed);
        }

        let latency_ns = latency.as_nanos() as u64;
        self.total_latency_ns
            .fetch_add(latency_ns, Ordering::Relaxed);

        // Update min latency
        let mut current_min = self.min_latency_ns.load(Ordering::Relaxed);
        while latency_ns < current_min {
            match self.min_latency_ns.compare_exchange_weak(
                current_min,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        // Update max latency
        let mut current_max = self.max_latency_ns.load(Ordering::Relaxed);
        while latency_ns > current_max {
            match self.max_latency_ns.compare_exchange_weak(
                current_max,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        // Update histogram
        for (i, &bucket) in LATENCY_BUCKETS.iter().enumerate() {
            if latency_ns <= bucket {
                self.latency_histogram[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }

        // Update queue depth
        self.current_queue_depth.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record batch processing
    pub fn record_batch(&self, count: usize, latency: Duration) {
        self.batch_count.fetch_add(1, Ordering::Relaxed);
        self.batch_total_size
            .fetch_add(count as u64, Ordering::Relaxed);
        self.batch_latency_ns
            .fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record completions batch
    pub fn record_completions(&self, count: usize, latency: Duration) {
        self.record_batch(count, latency);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a timeout
    pub fn record_timeout(&self) {
        self.timeouts.fetch_add(1, Ordering::Relaxed);
        self.current_queue_depth.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get statistics snapshot
    pub fn snapshot(&self) -> StatsReport {
        let now = Instant::now();

        let read_ops = self.read_ops.load(Ordering::Relaxed);
        let write_ops = self.write_ops.load(Ordering::Relaxed);
        let read_bytes = self.read_bytes.load(Ordering::Relaxed);
        let write_bytes = self.write_bytes.load(Ordering::Relaxed);
        let total_ops = read_ops + write_ops;

        let total_latency_ns = self.total_latency_ns.load(Ordering::Relaxed);
        let min_latency_ns = self.min_latency_ns.load(Ordering::Relaxed);
        let max_latency_ns = self.max_latency_ns.load(Ordering::Relaxed);

        // Calculate rates
        let (read_iops, write_iops, read_mbps, write_mbps) = {
            let mut last = self.last_snapshot.lock();

            if let Some(prev) = last.as_ref() {
                let elapsed = now.duration_since(prev.time).as_secs_f64();
                if elapsed > 0.0 {
                    let delta_read_ops = read_ops.saturating_sub(prev.read_ops);
                    let delta_write_ops = write_ops.saturating_sub(prev.write_ops);
                    let delta_read_bytes = read_bytes.saturating_sub(prev.read_bytes);
                    let delta_write_bytes = write_bytes.saturating_sub(prev.write_bytes);

                    let read_iops = (delta_read_ops as f64 / elapsed) as u64;
                    let write_iops = (delta_write_ops as f64 / elapsed) as u64;
                    let read_mbps = (delta_read_bytes as f64 / elapsed) / (1024.0 * 1024.0);
                    let write_mbps = (delta_write_bytes as f64 / elapsed) / (1024.0 * 1024.0);

                    // Update snapshot
                    *last = Some(StatsSnapshot {
                        time: now,
                        read_ops,
                        write_ops,
                        read_bytes,
                        write_bytes,
                    });

                    (read_iops, write_iops, read_mbps, write_mbps)
                } else {
                    (0, 0, 0.0, 0.0)
                }
            } else {
                *last = Some(StatsSnapshot {
                    time: now,
                    read_ops,
                    write_ops,
                    read_bytes,
                    write_bytes,
                });
                (0, 0, 0.0, 0.0)
            }
        };

        // Calculate average latency
        let avg_latency = if total_ops > 0 {
            Duration::from_nanos(total_latency_ns / total_ops)
        } else {
            Duration::ZERO
        };

        // Calculate percentile latencies from histogram
        let (p50, p99, p999) = self.calculate_percentiles();

        StatsReport {
            read_iops,
            write_iops,
            read_mbps,
            write_mbps,
            avg_latency,
            min_latency: if min_latency_ns == u64::MAX {
                Duration::ZERO
            } else {
                Duration::from_nanos(min_latency_ns)
            },
            max_latency: Duration::from_nanos(max_latency_ns),
            p50_latency: p50,
            p99_latency: p99,
            p999_latency: p999,
            total_commands: total_ops,
            total_bytes: read_bytes + write_bytes,
            current_queue_depth: self.current_queue_depth.load(Ordering::Relaxed),
            max_queue_depth: self.max_queue_depth.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            timeouts: self.timeouts.load(Ordering::Relaxed),
        }
    }

    /// Calculate percentile latencies from histogram
    fn calculate_percentiles(&self) -> (Duration, Duration, Duration) {
        let mut total: u64 = 0;
        let mut histogram = [0u64; 16];

        for (i, bucket) in self.latency_histogram.iter().enumerate() {
            histogram[i] = bucket.load(Ordering::Relaxed);
            total += histogram[i];
        }

        if total == 0 {
            return (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        }

        let p50_threshold = total / 2;
        let p99_threshold = (total * 99) / 100;
        let p999_threshold = (total * 999) / 1000;

        let mut cumulative = 0u64;
        let mut p50 = Duration::ZERO;
        let mut p99 = Duration::ZERO;
        let mut p999 = Duration::ZERO;

        for (i, &count) in histogram.iter().enumerate() {
            cumulative += count;

            if p50.is_zero() && cumulative >= p50_threshold {
                p50 = Duration::from_nanos(LATENCY_BUCKETS[i]);
            }
            if p99.is_zero() && cumulative >= p99_threshold {
                p99 = Duration::from_nanos(LATENCY_BUCKETS[i]);
            }
            if p999.is_zero() && cumulative >= p999_threshold {
                p999 = Duration::from_nanos(LATENCY_BUCKETS[i]);
                break;
            }
        }

        (p50, p99, p999)
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.read_ops.store(0, Ordering::Relaxed);
        self.write_ops.store(0, Ordering::Relaxed);
        self.read_bytes.store(0, Ordering::Relaxed);
        self.write_bytes.store(0, Ordering::Relaxed);
        self.total_latency_ns.store(0, Ordering::Relaxed);
        self.min_latency_ns.store(u64::MAX, Ordering::Relaxed);
        self.max_latency_ns.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.timeouts.store(0, Ordering::Relaxed);

        for bucket in &self.latency_histogram {
            bucket.store(0, Ordering::Relaxed);
        }

        *self.last_snapshot.lock() = None;
    }

    /// Get latency histogram as human-readable format
    pub fn latency_histogram_report(&self) -> String {
        let mut report = String::new();
        report.push_str("Latency Histogram:\n");

        let labels = [
            "  ≤1μs",
            "  ≤2μs",
            "  ≤4μs",
            "  ≤8μs",
            " ≤16μs",
            " ≤32μs",
            " ≤64μs",
            "≤128μs",
            "≤256μs",
            "≤512μs",
            "  ≤1ms",
            "  ≤2ms",
            "  ≤4ms",
            "  ≤8ms",
            " ≤16ms",
            " >16ms",
        ];

        for (i, label) in labels.iter().enumerate() {
            let count = self.latency_histogram[i].load(Ordering::Relaxed);
            if count > 0 {
                report.push_str(&format!("  {}: {}\n", label, count));
            }
        }

        report
    }
}

/// Throughput tracker for real-time rate calculation
pub struct ThroughputTracker {
    /// Ring buffer of (timestamp, bytes) samples
    samples: Mutex<Vec<(Instant, u64)>>,
    /// Window duration for rate calculation  
    window: Duration,
    /// Maximum samples to keep
    max_samples: usize,
}

impl ThroughputTracker {
    /// Create new throughput tracker
    pub fn new(window: Duration, max_samples: usize) -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(max_samples)),
            window,
            max_samples,
        }
    }

    /// Record bytes transferred
    pub fn record(&self, bytes: u64) {
        let now = Instant::now();
        let mut samples = self.samples.lock();

        // Remove old samples
        let cutoff = now - self.window;
        samples.retain(|(t, _)| *t >= cutoff);

        // Add new sample
        if samples.len() < self.max_samples {
            samples.push((now, bytes));
        }
    }

    /// Calculate current rate in bytes/second
    pub fn rate(&self) -> f64 {
        let now = Instant::now();
        let samples = self.samples.lock();

        if samples.is_empty() {
            return 0.0;
        }

        let cutoff = now - self.window;
        let (total_bytes, oldest) = samples
            .iter()
            .filter(|(t, _)| *t >= cutoff)
            .fold((0u64, now), |(sum, oldest), (t, b)| {
                (sum + b, oldest.min(*t))
            });

        let elapsed = now.duration_since(oldest).as_secs_f64();
        if elapsed > 0.0 {
            total_bytes as f64 / elapsed
        } else {
            0.0
        }
    }
}
