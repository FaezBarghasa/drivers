# Graphics Drivers

Redox OS features a modern graphics stack with a focus on high performance, hardware acceleration, and security via microkernel isolation.

## Graphics Abstraction Layer (GAL)

The **GAL** is the core interface for graphics in Redox. It provides a unified API for drivers to expose:

- VRAM management and shared memory buffers.
- Command buffer submission and validation.
- Secure DMA transfers.
- Display configuration and modesetting.

## Native GPU Drivers

### AMD (AMDGPU)

- **Status**: Native / Stable
- **Features**:
  - Asynchronous compute and graphics queues.
  - Support for modern GCN and RDNA architectures.
  - Power management support.
  - Integrated with the GAL for zero-copy buffer sharing.

### NVIDIA (Open-Kernel)

- **Status**: Native / Beta
- **Features**:
  - Based on the NVIDIA Open Kernel modules.
  - Support for Turing, Ampere, and newer architectures.
  - GSP (GPU System Processor) integration for firmware-managed scheduling.
  - Low-latency command submission.

### Intel (Xe / i915)

- **Status**: Native / Stable
- **Features**:
  - Support for Intel Core (Gen 11+) and Iris Xe graphics.
  - Efficient power state management.
  - Hardware-accelerated blitting and scaling.

### VirtIO-GPU

- **Status**: Stable
- **Features**:
  - Standardized interface for virtualized environments (QEMU/KVM).
  - Support for 2D/3D acceleration via host-side virglrenderer.

## Legacy & Fallback Drivers

### VESA / BGA

- **Status**: Stable / Fallback
- **Features**:
  - Basic framebuffer support used during boot or when no native driver is available.
  - Support for standard VBE modes.

## Advanced Features

### Upscaling (`drivers/graphics/upscaling`)

- **FSR-rs**: A Rust implementation of FidelityFX Super Resolution for high-performance upscaling.
- **DLSS-compat**: Compatibility layer for AI-based upscaling (experimental).

### Latency Management

- Standardized latency tracking per-frame to ensure 1000Hz+ response times in real-time scenarios.
