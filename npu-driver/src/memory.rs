//! NPU Memory Management
//!
//! Zero-copy buffer management for NPU operations.

/// Buffer usage flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    /// Read-only input
    ReadOnly,
    /// Write-only output
    WriteOnly,
    /// Read-write
    ReadWrite,
    /// Constant (immutable after creation)
    Constant,
    /// Scratch/temporary
    Scratch,
}

/// NPU buffer handle
#[derive(Debug, Clone)]
pub struct NpuBuffer {
    pub handle: u32,
    pub size: usize,
    pub usage: BufferUsage,
    pub ptr: *mut u8,
}

impl NpuBuffer {
    /// Create a buffer descriptor
    pub fn new(handle: u32, size: usize, usage: BufferUsage) -> Self {
        Self {
            handle,
            size,
            usage,
            ptr: std::ptr::null_mut(),
        }
    }

    /// Get buffer data as slice (if mapped)
    pub fn as_slice(&self) -> Option<&[u8]> {
        if self.ptr.is_null() {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(self.ptr, self.size) })
        }
    }

    /// Get buffer data as mutable slice (if mapped and writable)
    pub fn as_slice_mut(&mut self) -> Option<&mut [u8]> {
        if self.ptr.is_null()
            || self.usage == BufferUsage::ReadOnly
            || self.usage == BufferUsage::Constant
        {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) })
        }
    }

    /// Check if buffer is mapped
    pub fn is_mapped(&self) -> bool {
        !self.ptr.is_null()
    }
}

/// Memory layout for tensors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLayout {
    /// Row-major (C-style)
    RowMajor,
    /// Column-major (Fortran-style)
    ColumnMajor,
    /// Tiled for NPU
    Tiled { tile_h: u32, tile_w: u32 },
    /// Blocked for matrix ops
    Blocked { block_size: u32 },
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub allocation_count: u32,
    pub peak_usage: u64,
}
