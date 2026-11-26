//! RedoxML - Native AI/ML Computation API
//!
//! Lightweight tensor library optimized for microkernel architectures

pub mod blas;
pub mod inference;
pub mod npu;
pub mod tensor;

pub use blas::*;
pub use inference::*;
pub use npu::*;
pub use tensor::*;

/// Initialize RedoxML
pub fn init() -> Result<(), &'static str> {
    log::info!("Initializing RedoxML");

    // Detect available backends
    let backends = detect_backends();
    log::info!("Available backends: {:?}", backends);

    // Initialize BLAS
    blas::init()?;

    // Initialize NPU/TPU if available
    if backends.contains(&Backend::NPU) {
        npu::init()?;
    }

    log::info!("RedoxML initialized successfully");
    Ok(())
}

/// Compute backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    CPU,
    GPU,
    NPU,
    TPU,
}

fn detect_backends() -> Vec<Backend> {
    let mut backends = vec![Backend::CPU];

    // Check for GPU
    if std::path::Path::new("/scheme/gal").exists() {
        backends.push(Backend::GPU);
    }

    // Check for NPU
    if std::path::Path::new("/scheme/npu").exists() {
        backends.push(Backend::NPU);
    }

    backends
}

/// Trait for zero-copy buffer sharing between graphics and AI engines
pub trait ZeroCopyBuffer {
    /// Get GPU-accessible address
    fn gpu_address(&self) -> Option<u64>;

    /// Get NPU-accessible address
    fn npu_address(&self) -> Option<u64>;

    /// Check if buffer supports zero-copy between GPU and NPU
    fn supports_zero_copy(&self) -> bool;
}

impl<T: TensorType> ZeroCopyBuffer for Tensor<T> {
    fn gpu_address(&self) -> Option<u64> {
        self.gpu_addr()
    }

    fn npu_address(&self) -> Option<u64> {
        self.npu_addr()
    }

    fn supports_zero_copy(&self) -> bool {
        self.is_shared() || self.gpu_addr().is_some()
    }
}

/// Allocate a zero-copy tensor for DLSS/FSR upscaling
pub fn alloc_upscale_buffer<T: TensorType>(
    width: usize,
    height: usize,
    channels: usize,
) -> Result<Tensor<T>, &'static str> {
    let shape = Shape::new(vec![height, width, channels]);
    Tensor::alloc_shared(shape)
}

/// Create tensor from GPU framebuffer for AI processing
pub fn tensor_from_framebuffer<T: TensorType>(
    gpu_addr: u64,
    width: usize,
    height: usize,
    channels: usize,
) -> Tensor<T> {
    let shape = Shape::new(vec![height, width, channels]);
    Tensor::from_gpu_buffer(shape, gpu_addr)
}
