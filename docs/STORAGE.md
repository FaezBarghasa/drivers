# Storage Drivers

Storage in Redox is handled through a set of high-performance block drivers, focusing on low latency and high parallelism.

## NVMe (Modern Storage)

The NVMe driver is the primary high-performance storage interface in Redox.

### NVMe Features

- **Multi-Queue Architecture**: Implements independent submission and completion queues per CPU core to maximize throughput on multi-core systems.
- **Asynchronous I/O**: Full support for asynchronous command submission without blocking the calling thread.
- **ZNS (Zoned Namespaces)**: Support for next-generation Zoned NVMe devices (experimental).
- **Direct Attach (Redox NVMe Core)**: Optimized path for direct interaction between the kernel and the storage device, bypassing traditional block overhead.

## AHCI (SATA)

Standard interface for SATA SSDs and Hard Drives.

### AHCI Features

- Support for NCQ (Native Command Queuing).
- Hot-plug support.
- Reliable data transfer for legacy hardware.

## IDE

Legacy interface support for older hardware and older virtual machine configurations.

## VirtIO-Block

Optimized block storage for virtualized environments.

- Support for multi-queue virtio-blk.
- High-performance data path via shared ring buffers.

## Partition & File System Integration

- **Partition Library**: Standardized handling of GPT and MBR partition tables.
- **Lived**: A daemon that manages physical volumes and provides a unified block device interface to file systems (RedoxFS, etc.).
