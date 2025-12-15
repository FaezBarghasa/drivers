# High-Performance Async NVMe Driver for Redox OS

A top-tier, low-latency NVMe driver designed to deliver maximum IOPS and sequential throughput, competitive with specialized storage solutions.

## Features

### Multi-Core/Multi-Queue Architecture

- Creates one I/O queue pair per CPU core for parallel I/O processing
- Lock-free submission and completion queue handling
- NUMA-aware queue allocation (when available)
- Per-queue interrupt or polling mode

### Asynchronous I/O

- Non-blocking command submission
- Completion-based notification model
- Zero-copy data transfer support via physical address passing
- Integration with optimized IPC for minimal overhead

### I/O Scheduling

Multiple scheduling policies for different workloads:

| Scheduler | Description | Best For |
|-----------|-------------|----------|
| **None** | Direct submission, no scheduling overhead | Highest IOPS |
| **RoundRobin** | Fair distribution across all queues | Mixed workloads |
| **CpuAffinity** | Routes I/O to CPU-local queue | NUMA systems |
| **Priority** | Separate queues for priority levels | QoS requirements |
| **Deadline** | EDF scheduling with timeout handling | Latency-sensitive apps |

### Performance Monitoring

Real-time statistics including:

- Read/Write IOPS
- Throughput (MB/s)
- Latency percentiles (avg, p50, p99, p999)
- Queue depth tracking
- Error and timeout counts
- Latency histogram

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    User Applications                             │
└──────────────────────────┬──────────────────────────────────────┘
                           │ nvme:N scheme requests
┌──────────────────────────▼──────────────────────────────────────┐
│                    NVMe Driver Server                            │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  Request Router                          │    │
│  │  • CPU affinity-based queue selection                   │    │
│  │  • IO priority handling                                 │    │
│  └─────────────────────────────────────────────────────────┘    │
│                            │                                     │
│  ┌─────────────────────────▼───────────────────────────────┐    │
│  │              Per-CPU Queue Pairs                         │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │    │
│  │  │ SQ0/CQ0 │ │ SQ1/CQ1 │ │ SQ2/CQ2 │ │ SQ3/CQ3 │ ...    │    │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘        │    │
│  └───────┼───────────┼───────────┼───────────┼─────────────┘    │
│          │           │           │           │                   │
│  ┌───────▼───────────▼───────────▼───────────▼─────────────┐    │
│  │              Completion Handler (IRQ/Poll)               │    │
│  │  • Per-queue interrupt threads                          │    │
│  │  • Polling mode for high-IOPS workloads                 │    │
│  └─────────────────────────────────────────────────────────┘    │
└──────────────────────────┬──────────────────────────────────────┘
                           │ PCIe / Memory-mapped I/O
┌──────────────────────────▼──────────────────────────────────────┐
│                    NVMe Controller Hardware                      │
└─────────────────────────────────────────────────────────────────┘
```

## Configuration

Configure via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `NVME_NUM_QUEUES` | Auto | Number of I/O queues (0 = per-CPU) |
| `NVME_QUEUE_DEPTH` | 1024 | Commands per queue |
| `NVME_POLLING_MODE` | false | Use polling instead of interrupts |
| `NVME_POLL_INTERVAL_US` | 10 | Polling interval in microseconds |
| `NVME_ZERO_COPY` | true | Enable zero-copy transfers |
| `NVME_SCHEDULER` | cpuaffinity | I/O scheduler type |

### Scheduler Types

- `none` - No scheduling
- `roundrobin` - Round-robin dispatch
- `cpuaffinity` - CPU affinity-based
- `priority` - Priority-based
- `deadline` - Deadline-based (EDF)

## Usage

### Opening a namespace

```
file:/nvme:1/
```

### Reading data

Standard read() syscall with offset:

```rust
let fd = open("nvme:1/", O_RDWR)?;
let mut buf = [0u8; 4096];
pread(fd, &mut buf, offset)?;
```

### Zero-copy mode

Pass physical address with LSB set:

```rust
let phys_addr = get_physical_buffer();
let request_addr = phys_addr | 1;  // Set flag bit
```

## Performance Targets

### Random 4K Read (QD32)

- Target: 1,000,000+ IOPS
- Latency: < 100μs (p99)

### Sequential 128K Read

- Target: 7,000+ MB/s
- Latency: < 1ms (p99)

### Mixed Workload (70/30 R/W)

- Target: 500,000+ IOPS
- Consistent low latency

## Building

```bash
cd /home/jrad/RustroverProjects/redoxos/drivers
cargo build -p nvme-driver --release
```

### Feature Flags

- `performance-counters` - Enable detailed statistics (default)
- `multi-queue` - Multi-queue support (default)
- `zero-copy` - Zero-copy transfers
- `nvme-mi` - NVMe Management Interface
- `io-uring-compat` - io_uring style interface

## Files

| File | Description |
|------|-------------|
| `main.rs` | Driver entry point and event loop |
| `scheme.rs` | NVMe scheme handler |
| `queue.rs` | Queue pair management |
| `stats.rs` | Performance statistics |
| `io_scheduler.rs` | I/O scheduling policies |
| `benchmark.rs` | Benchmarking utilities |

## Testing

### Benchmark Mode

```bash
NVME_BENCHMARK=1 ./nvme-driver
```

### Sample fio-compatible output

```
nvme-bench: (groupid=0, jobs=4)
  randread: IOPS=983425, BW=3841.51MiB/s
    clat (usec): min=5.23, max=1842.50, avg=32.45
    clat percentiles (usec):
     |  1.00th=[      10], 50.00th=[      28], 99.00th=[      98], 99.90th=[     234]|
```

## License

MIT License - Copyright (c) 2024 RedoxOS Contributors
