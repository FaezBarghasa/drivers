# NPU & AI Drivers

As machine learning workloads become more common, Redox OS is implementing native support for AI acceleration hardware.

## NPU Driver (`drivers/npu-driver`)

The NPU (Neural Processing Unit) driver provides a low-level interface for AI accelerators.

### Features

- **Native Command Queues**: Direct submission of neural network graph kernels to the hardware.
- **Shared Memory Tensors**: Zero-copy tensor data passing between userspace applications and the NPU hardware.
- **Support for Industry Standards**: Initial support for various TPU and NPU IP cores.

## AI Acceleration Layer (`drivers/ai`)

This layer provides higher-level abstractions for AI workloads.

- Integration with graphics drivers for GPGPU-based acceleration (via Vulkan/DXVK).
- Standardized API for neural network inference.

## Planned Work

- Support for Intel NPU (Gen 14+).
- AMD Ryzen AI support.
- Native TPU drivers for specialized edge hardware.
