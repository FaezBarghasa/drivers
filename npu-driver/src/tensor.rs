//! Tensor Descriptors
//!
//! Tensor metadata for NPU operations.

/// Data types supported by NPU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Float32,
    Float16,
    BFloat16,
    Int32,
    Int16,
    Int8,
    UInt8,
    Bool,
}

impl DataType {
    /// Size in bytes
    pub fn size(&self) -> usize {
        match self {
            DataType::Float32 | DataType::Int32 => 4,
            DataType::Float16 | DataType::BFloat16 | DataType::Int16 => 2,
            DataType::Int8 | DataType::UInt8 | DataType::Bool => 1,
        }
    }

    /// Name string
    pub fn name(&self) -> &'static str {
        match self {
            DataType::Float32 => "float32",
            DataType::Float16 => "float16",
            DataType::BFloat16 => "bfloat16",
            DataType::Int32 => "int32",
            DataType::Int16 => "int16",
            DataType::Int8 => "int8",
            DataType::UInt8 => "uint8",
            DataType::Bool => "bool",
        }
    }
}

/// Tensor descriptor
#[derive(Debug, Clone)]
pub struct TensorDesc {
    /// Data type
    pub dtype: DataType,
    /// Shape (dimensions)
    pub shape: Vec<u32>,
    /// Strides (in elements)
    pub strides: Vec<u32>,
    /// Total size in bytes
    pub size_bytes: usize,
}

impl TensorDesc {
    pub fn new(dtype: DataType, shape: Vec<u32>) -> Self {
        let strides = Self::compute_strides(&shape);
        let num_elements: u32 = shape.iter().product();
        let size_bytes = num_elements as usize * dtype.size();

        Self {
            dtype,
            shape,
            strides,
            size_bytes,
        }
    }

    fn compute_strides(shape: &[u32]) -> Vec<u32> {
        let mut strides = vec![1u32; shape.len()];
        for i in (0..shape.len() - 1).rev() {
            strides[i] = strides[i + 1] * shape[i + 1];
        }
        strides
    }

    /// Number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Total number of elements
    pub fn numel(&self) -> usize {
        self.shape.iter().map(|&x| x as usize).product()
    }

    /// Check if shapes are compatible for broadcast
    pub fn is_broadcastable(&self, other: &TensorDesc) -> bool {
        let max_len = self.shape.len().max(other.shape.len());

        for i in 0..max_len {
            let a = self
                .shape
                .get(self.shape.len().saturating_sub(i + 1))
                .copied()
                .unwrap_or(1);
            let b = other
                .shape
                .get(other.shape.len().saturating_sub(i + 1))
                .copied()
                .unwrap_or(1);

            if a != b && a != 1 && b != 1 {
                return false;
            }
        }
        true
    }
}

/// Common tensor shapes
pub mod shapes {
    use super::*;

    pub fn scalar(dtype: DataType) -> TensorDesc {
        TensorDesc::new(dtype, vec![1])
    }

    pub fn vector(dtype: DataType, n: u32) -> TensorDesc {
        TensorDesc::new(dtype, vec![n])
    }

    pub fn matrix(dtype: DataType, m: u32, n: u32) -> TensorDesc {
        TensorDesc::new(dtype, vec![m, n])
    }

    pub fn image_nhwc(dtype: DataType, n: u32, h: u32, w: u32, c: u32) -> TensorDesc {
        TensorDesc::new(dtype, vec![n, h, w, c])
    }

    pub fn image_nchw(dtype: DataType, n: u32, c: u32, h: u32, w: u32) -> TensorDesc {
        TensorDesc::new(dtype, vec![n, c, h, w])
    }
}
