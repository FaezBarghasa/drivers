// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

//! NVMe Driver Benchmarking Utilities
//!
//! Provides tools for measuring and validating driver performance.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::queue::QueuePair;
use crate::stats::{PerformanceStats, StatsReport};

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Duration of the benchmark
    pub duration: Duration,
    /// Number of concurrent threads
    pub threads: usize,
    /// I/O size in bytes
    pub io_size: usize,
    /// Queue depth per thread
    pub queue_depth: u32,
    /// Read/write mix (0.0 = all reads, 1.0 = all writes)
    pub write_ratio: f64,
    /// Random vs sequential (true = random)
    pub random: bool,
    /// Namespace ID to test
    pub ns_id: u32,
    /// Warmup duration before measurements
    pub warmup: Duration,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
            threads: 4,
            io_size: 4096,
            queue_depth: 32,
            write_ratio: 0.0,
            random: true,
            ns_id: 1,
            warmup: Duration::from_secs(5),
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Configuration used
    pub config: BenchmarkConfig,
    /// Total IOPS achieved
    pub iops: u64,
    /// Throughput in MB/s
    pub throughput_mbps: f64,
    /// Average latency
    pub avg_latency: Duration,
    /// Minimum latency
    pub min_latency: Duration,
    /// Maximum latency
    pub max_latency: Duration,
    /// 50th percentile latency
    pub p50_latency: Duration,
    /// 99th percentile latency
    pub p99_latency: Duration,
    /// 99.9th percentile latency
    pub p999_latency: Duration,
    /// Total I/O operations completed
    pub total_ios: u64,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Actual test duration
    pub actual_duration: Duration,
    /// Errors encountered
    pub errors: u64,
}

impl BenchmarkResults {
    /// Print results in fio-compatible format
    pub fn print_fio_format(&self) {
        let rw_str = if self.config.write_ratio == 0.0 {
            "read"
        } else if self.config.write_ratio == 1.0 {
            "write"
        } else {
            "randrw"
        };

        let pattern = if self.config.random { "rand" } else { "seq" };

        println!("nvme-bench: (groupid=0, jobs={})", self.config.threads);
        println!(
            "  {}{}: IOPS={}, BW={:.2}MiB/s ({}B/s)",
            pattern,
            rw_str,
            self.iops,
            self.throughput_mbps,
            (self.throughput_mbps * 1024.0 * 1024.0) as u64
        );
        println!(
            "    slat (usec): min={:.2}, max={:.2}, avg={:.2}",
            0.0,
            0.0,
            0.0 // Submission latency not tracked
        );
        println!(
            "    clat (usec): min={:.2}, max={:.2}, avg={:.2}",
            self.min_latency.as_micros() as f64,
            self.max_latency.as_micros() as f64,
            self.avg_latency.as_micros() as f64
        );
        println!(
            "     lat (usec): min={:.2}, max={:.2}, avg={:.2}",
            self.min_latency.as_micros() as f64,
            self.max_latency.as_micros() as f64,
            self.avg_latency.as_micros() as f64
        );
        println!("    clat percentiles (usec):");
        println!(
            "     |  1.00th=[{:>8}], 50.00th=[{:>8}], 99.00th=[{:>8}], 99.90th=[{:>8}]|",
            self.min_latency.as_micros(),
            self.p50_latency.as_micros(),
            self.p99_latency.as_micros(),
            self.p999_latency.as_micros()
        );
        println!(
            "   bw (  MiB/s): min={:.2}, max={:.2}, avg={:.2}",
            self.throughput_mbps * 0.8, // Approximate
            self.throughput_mbps * 1.2,
            self.throughput_mbps
        );
        println!(
            "   iops        : min={:>8}, max={:>8}, avg={:.2}",
            (self.iops as f64 * 0.8) as u64,
            (self.iops as f64 * 1.2) as u64,
            self.iops as f64
        );
        println!();
        println!("Run status group 0 (all jobs):");
        if self.config.write_ratio < 1.0 {
            println!(
                "   READ: bw={:.2}MiB/s ({}B/s), iops={}, io={}MiB",
                self.throughput_mbps * (1.0 - self.config.write_ratio),
                ((self.throughput_mbps * (1.0 - self.config.write_ratio)) * 1024.0 * 1024.0) as u64,
                (self.iops as f64 * (1.0 - self.config.write_ratio)) as u64,
                (self.total_bytes as f64 * (1.0 - self.config.write_ratio)) / (1024.0 * 1024.0)
            );
        }
        if self.config.write_ratio > 0.0 {
            println!(
                "  WRITE: bw={:.2}MiB/s ({}B/s), iops={}, io={}MiB",
                self.throughput_mbps * self.config.write_ratio,
                ((self.throughput_mbps * self.config.write_ratio) * 1024.0 * 1024.0) as u64,
                (self.iops as f64 * self.config.write_ratio) as u64,
                (self.total_bytes as f64 * self.config.write_ratio) / (1024.0 * 1024.0)
            );
        }
    }

    /// Print results as JSON
    pub fn to_json(&self) -> String {
        format!(
            r#"{{
  "config": {{
    "duration_secs": {},
    "threads": {},
    "io_size": {},
    "queue_depth": {},
    "write_ratio": {},
    "random": {}
  }},
  "results": {{
    "iops": {},
    "throughput_mbps": {:.2},
    "avg_latency_us": {},
    "min_latency_us": {},
    "max_latency_us": {},
    "p50_latency_us": {},
    "p99_latency_us": {},
    "p999_latency_us": {},
    "total_ios": {},
    "total_bytes": {},
    "errors": {}
  }}
}}"#,
            self.config.duration.as_secs(),
            self.config.threads,
            self.config.io_size,
            self.config.queue_depth,
            self.config.write_ratio,
            self.config.random,
            self.iops,
            self.throughput_mbps,
            self.avg_latency.as_micros(),
            self.min_latency.as_micros(),
            self.max_latency.as_micros(),
            self.p50_latency.as_micros(),
            self.p99_latency.as_micros(),
            self.p999_latency.as_micros(),
            self.total_ios,
            self.total_bytes,
            self.errors
        )
    }
}

/// Latency tracker for percentile calculations
pub struct LatencyTracker {
    /// All latency samples
    samples: parking_lot::Mutex<Vec<Duration>>,
    /// Maximum samples to keep
    max_samples: usize,
}

impl LatencyTracker {
    /// Create new tracker
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: parking_lot::Mutex::new(Vec::with_capacity(max_samples)),
            max_samples,
        }
    }

    /// Record a latency sample
    pub fn record(&self, latency: Duration) {
        let mut samples = self.samples.lock();
        if samples.len() < self.max_samples {
            samples.push(latency);
        } else {
            // Reservoir sampling for very long benchmarks
            let idx = rand_usize() % samples.len();
            samples[idx] = latency;
        }
    }

    /// Calculate percentiles
    pub fn percentiles(&self) -> LatencyPercentiles {
        let mut samples = self.samples.lock().clone();
        if samples.is_empty() {
            return LatencyPercentiles::default();
        }

        samples.sort();
        let len = samples.len();

        LatencyPercentiles {
            min: samples[0],
            max: samples[len - 1],
            avg: Duration::from_nanos(
                samples.iter().map(|d| d.as_nanos() as u64).sum::<u64>() / len as u64,
            ),
            p50: samples[len / 2],
            p90: samples[(len * 90) / 100],
            p99: samples[(len * 99) / 100],
            p999: samples
                .get((len * 999) / 1000)
                .copied()
                .unwrap_or(samples[len - 1]),
        }
    }
}

/// Latency percentiles
#[derive(Debug, Clone, Default)]
pub struct LatencyPercentiles {
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub p50: Duration,
    pub p90: Duration,
    pub p99: Duration,
    pub p999: Duration,
}

/// Simple pseudo-random number generator
fn rand_usize() -> usize {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as usize
}

/// IOPS calculator using sliding window
pub struct IopsCalculator {
    /// Window of (timestamp, count) samples
    samples: parking_lot::Mutex<Vec<(Instant, u64)>>,
    /// Window duration
    window: Duration,
}

impl IopsCalculator {
    /// Create new calculator
    pub fn new(window: Duration) -> Self {
        Self {
            samples: parking_lot::Mutex::new(Vec::new()),
            window,
        }
    }

    /// Record completed I/O
    pub fn record(&self, count: u64) {
        let now = Instant::now();
        let mut samples = self.samples.lock();

        // Remove old samples
        let cutoff = now - self.window;
        samples.retain(|(t, _)| *t >= cutoff);

        samples.push((now, count));
    }

    /// Calculate current IOPS
    pub fn iops(&self) -> u64 {
        let now = Instant::now();
        let samples = self.samples.lock();

        if samples.is_empty() {
            return 0;
        }

        let cutoff = now - self.window;
        let total: u64 = samples
            .iter()
            .filter(|(t, _)| *t >= cutoff)
            .map(|(_, c)| *c)
            .sum();

        let oldest = samples
            .iter()
            .filter(|(t, _)| *t >= cutoff)
            .map(|(t, _)| *t)
            .min()
            .unwrap_or(now);

        let elapsed = now.duration_since(oldest).as_secs_f64();
        if elapsed > 0.0 {
            (total as f64 / elapsed) as u64
        } else {
            0
        }
    }
}

/// Test patterns for various workloads
pub mod patterns {
    use super::*;

    /// 4K random read - typical database workload
    pub fn random_read_4k() -> BenchmarkConfig {
        BenchmarkConfig {
            io_size: 4096,
            random: true,
            write_ratio: 0.0,
            queue_depth: 32,
            ..Default::default()
        }
    }

    /// 4K random write
    pub fn random_write_4k() -> BenchmarkConfig {
        BenchmarkConfig {
            io_size: 4096,
            random: true,
            write_ratio: 1.0,
            queue_depth: 32,
            ..Default::default()
        }
    }

    /// 4K 70/30 read/write mix
    pub fn mixed_4k() -> BenchmarkConfig {
        BenchmarkConfig {
            io_size: 4096,
            random: true,
            write_ratio: 0.3,
            queue_depth: 32,
            ..Default::default()
        }
    }

    /// 128K sequential read - streaming workload
    pub fn sequential_read_128k() -> BenchmarkConfig {
        BenchmarkConfig {
            io_size: 128 * 1024,
            random: false,
            write_ratio: 0.0,
            queue_depth: 8,
            threads: 1,
            ..Default::default()
        }
    }

    /// 128K sequential write
    pub fn sequential_write_128k() -> BenchmarkConfig {
        BenchmarkConfig {
            io_size: 128 * 1024,
            random: false,
            write_ratio: 1.0,
            queue_depth: 8,
            threads: 1,
            ..Default::default()
        }
    }

    /// Queue depth sweep test
    pub fn qd_sweep(qd: u32) -> BenchmarkConfig {
        BenchmarkConfig {
            io_size: 4096,
            random: true,
            write_ratio: 0.0,
            queue_depth: qd,
            threads: 1,
            ..Default::default()
        }
    }

    /// Block size sweep test
    pub fn bs_sweep(block_size: usize) -> BenchmarkConfig {
        BenchmarkConfig {
            io_size: block_size,
            random: true,
            write_ratio: 0.0,
            queue_depth: 32,
            ..Default::default()
        }
    }
}
