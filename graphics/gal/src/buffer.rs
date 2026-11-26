//! Buffer resource management
//!
//! This module provides GPU buffer abstractions for vertex, index,
//! uniform, and storage buffers.

use bitflags::bitflags;

use crate::{Error, Memory, MemoryType, Result};

bitflags! {
    /// Buffer usage flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BufferUsage: u32 {
        /// Buffer can be used as source for transfer operations
        const TRANSFER_SRC = 1 << 0;
        /// Buffer can be used as destination for transfer operations
        const TRANSFER_DST = 1 << 1;
        /// Buffer can be used as a uniform texel buffer
        const UNIFORM_TEXEL = 1 << 2;
        /// Buffer can be used as a storage texel buffer
        const STORAGE_TEXEL = 1 << 3;
        /// Buffer can be used as a uniform buffer
        const UNIFORM = 1 << 4;
        /// Buffer can be used as a storage buffer
        const STORAGE = 1 << 5;
        /// Buffer can be used as an index buffer
        const INDEX = 1 << 6;
        /// Buffer can be used as a vertex buffer
        const VERTEX = 1 << 7;
        /// Buffer can be used for indirect draw/dispatch commands
        const INDIRECT = 1 << 8;
    }
}

/// Buffer descriptor for creation
#[derive(Debug, Clone)]
pub struct BufferDescriptor {
    /// Size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: BufferUsage,
    /// Memory type requirements
    pub memory_type: MemoryType,
    /// Mapped at creation
    pub mapped_at_creation: bool,
    /// Debug label
    pub label: Option<&'static str>,
}

impl BufferDescriptor {
    /// Create a new buffer descriptor
    pub fn new(size: u64, usage: BufferUsage) -> Self {
        Self {
            size,
            usage,
            memory_type: MemoryType::DeviceLocal,
            mapped_at_creation: false,
            label: None,
        }
    }

    /// Set memory type
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = memory_type;
        self
    }

    /// Set mapped at creation
    pub fn mapped_at_creation(mut self, mapped: bool) -> Self {
        self.mapped_at_creation = mapped;
        self
    }

    /// Set debug label
    pub fn label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// Create a vertex buffer descriptor
    pub fn vertex(size: u64) -> Self {
        Self::new(size, BufferUsage::VERTEX | BufferUsage::TRANSFER_DST)
    }

    /// Create an index buffer descriptor
    pub fn index(size: u64) -> Self {
        Self::new(size, BufferUsage::INDEX | BufferUsage::TRANSFER_DST)
    }

    /// Create a uniform buffer descriptor
    pub fn uniform(size: u64) -> Self {
        Self::new(size, BufferUsage::UNIFORM | BufferUsage::TRANSFER_DST)
            .memory_type(MemoryType::HostVisible)
    }

    /// Create a storage buffer descriptor
    pub fn storage(size: u64) -> Self {
        Self::new(
            size,
            BufferUsage::STORAGE | BufferUsage::TRANSFER_DST | BufferUsage::TRANSFER_SRC,
        )
    }

    /// Create a staging buffer descriptor
    pub fn staging(size: u64) -> Self {
        Self::new(size, BufferUsage::TRANSFER_SRC)
            .memory_type(MemoryType::HostVisible)
            .mapped_at_creation(true)
    }
}

/// GPU buffer resource
pub trait Buffer: Send + Sync {
    /// Get buffer handle ID
    fn handle(&self) -> usize;

    /// Get buffer size in bytes
    fn size(&self) -> u64;

    /// Get buffer usage flags
    fn usage(&self) -> BufferUsage;

    /// Get associated memory
    fn memory(&self) -> Option<&dyn Memory>;

    /// Map buffer memory for CPU access
    fn map(&self) -> Result<*mut u8>;

    /// Unmap buffer memory
    fn unmap(&self);

    /// Flush mapped memory range (for non-coherent memory)
    fn flush(&self, offset: u64, size: u64) -> Result<()>;

    /// Invalidate mapped memory range (for non-coherent memory)
    fn invalidate(&self, offset: u64, size: u64) -> Result<()>;

    /// Write data to buffer (convenience method)
    fn write(&self, offset: u64, data: &[u8]) -> Result<()> {
        if offset + data.len() as u64 > self.size() {
            return Err(Error::InvalidParameter);
        }

        let ptr = self.map()?;
        unsafe {
            core::ptr::copy_nonoverlapping(data.as_ptr(), ptr.add(offset as usize), data.len());
        }
        self.flush(offset, data.len() as u64)?;
        self.unmap();

        Ok(())
    }

    /// Read data from buffer (convenience method)
    fn read(&self, offset: u64, data: &mut [u8]) -> Result<()> {
        if offset + data.len() as u64 > self.size() {
            return Err(Error::InvalidParameter);
        }

        self.invalidate(offset, data.len() as u64)?;
        let ptr = self.map()?;
        unsafe {
            core::ptr::copy_nonoverlapping(ptr.add(offset as usize), data.as_mut_ptr(), data.len());
        }
        self.unmap();

        Ok(())
    }
}

/// Buffer view for typed access
pub struct BufferView<'a, T> {
    buffer: &'a dyn Buffer,
    offset: u64,
    count: usize,
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Copy> BufferView<'a, T> {
    /// Create a new buffer view
    pub fn new(buffer: &'a dyn Buffer, offset: u64, count: usize) -> Result<Self> {
        let element_size = core::mem::size_of::<T>() as u64;
        let total_size = element_size * count as u64;

        if offset + total_size > buffer.size() {
            return Err(Error::InvalidParameter);
        }

        Ok(Self {
            buffer,
            offset,
            count,
            _marker: core::marker::PhantomData,
        })
    }

    /// Get element count
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get offset in bytes
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Get size in bytes
    pub fn size(&self) -> u64 {
        (core::mem::size_of::<T>() * self.count) as u64
    }
}
