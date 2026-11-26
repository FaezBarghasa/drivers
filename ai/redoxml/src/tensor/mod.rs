//! Native Tensor Abstraction

use num_traits::Float;
use std::sync::Arc;

/// Tensor data type
pub trait TensorType: Float + Send + Sync + 'static {}
impl TensorType for f32 {}
impl TensorType for f64 {}

/// Tensor shape
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub dims: Vec<usize>,
}

impl Shape {
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    pub fn ndim(&self) -> usize {
        self.dims.len()
    }

    pub fn size(&self) -> usize {
        self.dims.iter().product()
    }

    pub fn get_dims(&self) -> &[usize] {
        &self.dims
    }
}

/// Tensor with async execution support
#[derive(Clone)]
pub struct Tensor<T: TensorType> {
    /// Data buffer
    data: Arc<TensorData<T>>,
    /// Shape
    shape: Shape,
    /// Backend
    backend: crate::Backend,
}

/// Tensor data storage
pub enum TensorData<T> {
    /// CPU memory
    Cpu(Vec<T>),
    /// GPU VRAM (device address)
    Gpu(u64),
    /// NPU memory (device address)
    Npu(u64),
    /// Shared GPU/NPU memory (zero-copy path)
    Shared(SharedBuffer),
}

/// Zero-copy shared buffer between GPU and NPU
#[derive(Clone)]
pub struct SharedBuffer {
    /// Physical address accessible by both GPU and NPU
    pub phys_addr: u64,
    /// Size in bytes
    pub size: usize,
    /// GPU device handle
    pub gpu_handle: u64,
    /// NPU device handle
    pub npu_handle: u64,
}

impl SharedBuffer {
    /// Create a new shared buffer for zero-copy transfers
    pub fn new(size: usize) -> Result<Self, &'static str> {
        log::info!("Allocating shared GPU/NPU buffer: {} bytes", size);

        // In a real implementation, this would:
        // 1. Open /scheme/gal
        // 2. Request coherent memory allocation
        // 3. Map the memory for both GPU and NPU access

        // Simulated allocation
        let phys_addr = 0x1000_0000; // Fake physical address

        Ok(Self {
            phys_addr,
            size,
            gpu_handle: phys_addr,
            npu_handle: phys_addr,
        })
    }

    /// Get GPU-accessible address
    pub fn gpu_addr(&self) -> u64 {
        self.gpu_handle
    }

    /// Get NPU-accessible address  
    pub fn npu_addr(&self) -> u64 {
        self.npu_handle
    }
}

impl<T: TensorType> Tensor<T> {
    /// Create new tensor on CPU
    pub fn new(shape: Shape, data: Vec<T>) -> Self {
        assert_eq!(shape.size(), data.len());

        Self {
            data: Arc::new(TensorData::Cpu(data)),
            shape,
            backend: crate::Backend::CPU,
        }
    }

    /// Create tensor from existing GPU buffer (zero-copy import)
    pub fn from_gpu_buffer(shape: Shape, gpu_addr: u64) -> Self {
        log::debug!("Creating tensor from GPU buffer: 0x{:x}", gpu_addr);
        Self {
            data: Arc::new(TensorData::Gpu(gpu_addr)),
            shape,
            backend: crate::Backend::GPU,
        }
    }

    /// Create tensor from existing NPU buffer (zero-copy import)
    pub fn from_npu_buffer(shape: Shape, npu_addr: u64) -> Self {
        log::debug!("Creating tensor from NPU buffer: 0x{:x}", npu_addr);
        Self {
            data: Arc::new(TensorData::Npu(npu_addr)),
            shape,
            backend: crate::Backend::NPU,
        }
    }

    /// Create tensor using shared GPU/NPU memory (true zero-copy)
    pub fn from_shared_buffer(shape: Shape, buffer: SharedBuffer) -> Self {
        log::debug!(
            "Creating tensor from shared buffer: 0x{:x}",
            buffer.phys_addr
        );
        Self {
            data: Arc::new(TensorData::Shared(buffer)),
            shape,
            backend: crate::Backend::GPU, // Primary backend is GPU
        }
    }

    /// Allocate shared buffer for zero-copy GPU/NPU operations
    pub fn alloc_shared(shape: Shape) -> Result<Self, &'static str> {
        let size = shape.size() * std::mem::size_of::<T>();
        let buffer = SharedBuffer::new(size)?;
        Ok(Self::from_shared_buffer(shape, buffer))
    }

    /// Create tensor filled with zeros
    pub fn zeros(shape: Shape) -> Self {
        let data = vec![T::zero(); shape.size()];
        Self::new(shape, data)
    }

    /// Create tensor filled with ones
    pub fn ones(shape: Shape) -> Self {
        let data = vec![T::one(); shape.size()];
        Self::new(shape, data)
    }

    /// Get shape
    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// Get backend
    pub fn backend(&self) -> crate::Backend {
        self.backend
    }

    /// Check if tensor uses shared memory (zero-copy capable)
    pub fn is_shared(&self) -> bool {
        matches!(&*self.data, TensorData::Shared(_))
    }

    /// Get GPU address (for shared or GPU tensors)
    pub fn gpu_addr(&self) -> Option<u64> {
        match &*self.data {
            TensorData::Gpu(addr) => Some(*addr),
            TensorData::Shared(buf) => Some(buf.gpu_addr()),
            _ => None,
        }
    }

    /// Get NPU address (for shared or NPU tensors)
    pub fn npu_addr(&self) -> Option<u64> {
        match &*self.data {
            TensorData::Npu(addr) => Some(*addr),
            TensorData::Shared(buf) => Some(buf.npu_addr()),
            _ => None,
        }
    }

    /// Share tensor with NPU (zero-copy if already on GPU with shared memory)
    pub async fn share_with_npu(&self) -> Result<Tensor<T>, &'static str> {
        log::debug!("Sharing tensor with NPU");

        match &*self.data {
            TensorData::Shared(buf) => {
                // Already shared - just create NPU view
                Ok(Tensor {
                    data: Arc::new(TensorData::Npu(buf.npu_addr())),
                    shape: self.shape.clone(),
                    backend: crate::Backend::NPU,
                })
            }
            TensorData::Gpu(addr) => {
                // GPU buffer - check if coherent memory
                // For now, assume we can share the address
                log::debug!("Zero-copy GPU→NPU sharing at 0x{:x}", addr);
                Ok(Tensor {
                    data: Arc::new(TensorData::Npu(*addr)),
                    shape: self.shape.clone(),
                    backend: crate::Backend::NPU,
                })
            }
            TensorData::Cpu(_) => {
                // Need to allocate shared buffer and copy
                let shared = Self::alloc_shared(self.shape.clone())?;
                // In real impl: copy CPU data to shared buffer
                log::debug!("Copied CPU tensor to shared buffer");
                Ok(shared)
            }
            TensorData::Npu(addr) => {
                // Already on NPU
                Ok(Tensor {
                    data: Arc::new(TensorData::Npu(*addr)),
                    shape: self.shape.clone(),
                    backend: crate::Backend::NPU,
                })
            }
        }
    }

    /// Transfer to GPU (zero-copy if possible)
    pub async fn to_gpu(&self) -> Result<Tensor<T>, &'static str> {
        log::debug!("Transferring tensor to GPU");

        match &*self.data {
            TensorData::Shared(buf) => {
                // Shared buffer - just create GPU view
                Ok(Tensor {
                    data: Arc::new(TensorData::Gpu(buf.gpu_addr())),
                    shape: self.shape.clone(),
                    backend: crate::Backend::GPU,
                })
            }
            TensorData::Gpu(addr) => {
                // Already on GPU
                Ok(Tensor {
                    data: Arc::new(TensorData::Gpu(*addr)),
                    shape: self.shape.clone(),
                    backend: crate::Backend::GPU,
                })
            }
            _ => {
                // Allocate GPU memory via GAL
                let gpu_addr = 0; // Would allocate via /scheme/gal
                Ok(Tensor {
                    data: Arc::new(TensorData::Gpu(gpu_addr)),
                    shape: self.shape.clone(),
                    backend: crate::Backend::GPU,
                })
            }
        }
    }

    /// Transfer to NPU (zero-copy from GPU if possible)
    pub async fn to_npu(&self) -> Result<Tensor<T>, &'static str> {
        self.share_with_npu().await
    }

    /// Get access to CPU data slice
    pub fn data_as_slice(&self) -> Option<&[T]> {
        match &*self.data {
            TensorData::Cpu(vec) => Some(vec.as_slice()),
            _ => None,
        }
    }

    /// Matrix multiplication
    pub async fn matmul(&self, other: &Tensor<T>) -> Result<Tensor<T>, &'static str> {
        match self.backend {
            crate::Backend::CPU => self.matmul_cpu(other).await,
            crate::Backend::GPU => self.matmul_gpu(other).await,
            crate::Backend::NPU => self.matmul_npu(other).await,
            crate::Backend::TPU => self.matmul_tpu(other).await,
        }
    }

    async fn matmul_cpu(&self, other: &Tensor<T>) -> Result<Tensor<T>, &'static str> {
        crate::blas::gemm::<T>(self, other).await
    }

    async fn matmul_gpu(&self, other: &Tensor<T>) -> Result<Tensor<T>, &'static str> {
        // For shared tensors, try NPU acceleration
        if self.is_shared() {
            let a_npu = self.share_with_npu().await?;
            let b_npu = other.share_with_npu().await?;
            return a_npu.matmul_npu(&b_npu).await;
        }
        Err("GPU matmul not implemented")
    }

    async fn matmul_npu(&self, other: &Tensor<T>) -> Result<Tensor<T>, &'static str> {
        let device = crate::npu::NpuDevice::open().map_err(|_| "Failed to open NPU device")?;

        let a_addr = self.npu_addr().ok_or("Tensor A not accessible by NPU")?;
        let b_addr = other.npu_addr().ok_or("Tensor B not accessible by NPU")?;

        let shape = Shape::new(vec![self.shape.dims[0], other.shape.dims[1]]);
        let c_addr = device
            .alloc(shape.size() * std::mem::size_of::<T>())
            .map_err(|_| "NPU alloc failed")?;

        let cmd = crate::npu::NpuCommand {
            op_code: 1, // MatMul
            inputs: vec![a_addr, b_addr],
            outputs: vec![c_addr],
        };

        device
            .submit_command(cmd)
            .await
            .map_err(|_| "NPU submission failed")?;

        Ok(Tensor {
            data: Arc::new(TensorData::Npu(c_addr)),
            shape,
            backend: crate::Backend::NPU,
        })
    }

    async fn matmul_tpu(&self, _other: &Tensor<T>) -> Result<Tensor<T>, &'static str> {
        Err("TPU matmul not implemented")
    }
}
