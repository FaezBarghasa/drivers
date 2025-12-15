//! VirtIO-GPU resource implementations
//!
//! This module provides buffer, image, and memory resources for VirtIO-GPU.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use gal::{
    Buffer, BufferDescriptor, BufferUsage, Error, Extent3D, Image, ImageDescriptor, ImageDimension,
    ImageFormat, ImageUsage, Memory, MemoryType, Result,
};

/// VirtIO buffer implementation
pub struct VirtioBuffer {
    handle: usize,
    resource_id: u32,
    size: u64,
    usage: BufferUsage,
    memory_type: MemoryType,
    data: spin::RwLock<Vec<u8>>,
}

impl VirtioBuffer {
    pub fn new(resource_id: u32, descriptor: &BufferDescriptor) -> Self {
        let data = if descriptor.mapped_at_creation {
            vec![0u8; descriptor.size as usize]
        } else {
            Vec::new()
        };

        Self {
            handle: resource_id as usize,
            resource_id,
            size: descriptor.size,
            usage: descriptor.usage,
            memory_type: descriptor.memory_type,
            data: spin::RwLock::new(data),
        }
    }

    /// Get the VirtIO resource ID
    pub fn resource_id(&self) -> u32 {
        self.resource_id
    }
}

impl Buffer for VirtioBuffer {
    fn handle(&self) -> usize {
        self.handle
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn usage(&self) -> BufferUsage {
        self.usage
    }

    fn memory(&self) -> Option<&dyn Memory> {
        None // Memory is embedded
    }

    fn map(&self) -> Result<*mut u8> {
        let mut data = self.data.write();
        if data.is_empty() {
            data.resize(self.size as usize, 0);
        }
        Ok(data.as_mut_ptr())
    }

    fn unmap(&self) {
        // Data stays allocated
    }

    fn flush(&self, _offset: u64, _size: u64) -> Result<()> {
        // In a real implementation, this would transfer data to host
        Ok(())
    }

    fn invalidate(&self, _offset: u64, _size: u64) -> Result<()> {
        // In a real implementation, this would transfer data from host
        Ok(())
    }
}

/// VirtIO image implementation
pub struct VirtioImage {
    handle: usize,
    resource_id: u32,
    dimension: ImageDimension,
    extent: Extent3D,
    format: ImageFormat,
    mip_levels: u32,
    array_layers: u32,
    sample_count: u32,
    usage: ImageUsage,
    data: spin::RwLock<Vec<u8>>,
}

impl VirtioImage {
    pub fn new(resource_id: u32, descriptor: &ImageDescriptor) -> Self {
        let bytes_per_pixel = descriptor.format.bytes_per_pixel().unwrap_or(4);
        let size = (descriptor.extent.width
            * descriptor.extent.height
            * descriptor.extent.depth
            * bytes_per_pixel) as usize;

        Self {
            handle: resource_id as usize,
            resource_id,
            dimension: descriptor.dimension,
            extent: descriptor.extent,
            format: descriptor.format,
            mip_levels: descriptor.mip_levels,
            array_layers: descriptor.array_layers,
            sample_count: descriptor.sample_count,
            usage: descriptor.usage,
            data: spin::RwLock::new(vec![0u8; size]),
        }
    }

    /// Get the VirtIO resource ID
    pub fn resource_id(&self) -> u32 {
        self.resource_id
    }

    /// Get a mutable pointer to the image data
    pub fn data_ptr(&self) -> *mut u8 {
        self.data.write().as_mut_ptr()
    }

    /// Get the image data size in bytes
    pub fn data_size(&self) -> usize {
        self.data.read().len()
    }
}

impl Image for VirtioImage {
    fn handle(&self) -> usize {
        self.handle
    }

    fn dimension(&self) -> ImageDimension {
        self.dimension
    }

    fn extent(&self) -> Extent3D {
        self.extent
    }

    fn format(&self) -> ImageFormat {
        self.format
    }

    fn mip_levels(&self) -> u32 {
        self.mip_levels
    }

    fn array_layers(&self) -> u32 {
        self.array_layers
    }

    fn sample_count(&self) -> u32 {
        self.sample_count
    }

    fn usage(&self) -> ImageUsage {
        self.usage
    }

    fn memory(&self) -> Option<&dyn Memory> {
        None // Memory is embedded
    }
}

/// VirtIO memory implementation
pub struct VirtioMemory {
    handle: usize,
    size: u64,
    memory_type: MemoryType,
    data: spin::RwLock<Vec<u8>>,
}

impl VirtioMemory {
    pub fn new(size: u64, memory_type: MemoryType) -> Self {
        static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

        Self {
            handle: NEXT_HANDLE.fetch_add(1, Ordering::SeqCst) as usize,
            size,
            memory_type,
            data: spin::RwLock::new(vec![0u8; size as usize]),
        }
    }
}

impl Memory for VirtioMemory {
    fn handle(&self) -> usize {
        self.handle
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn memory_type(&self) -> MemoryType {
        self.memory_type
    }

    fn map(&self, offset: u64, size: u64) -> Result<*mut u8> {
        if offset + size > self.size {
            return Err(Error::InvalidParameter);
        }

        let mut data = self.data.write();
        Ok(unsafe { data.as_mut_ptr().add(offset as usize) })
    }

    fn unmap(&self) {
        // Data stays allocated
    }

    fn flush(&self, _offset: u64, _size: u64) -> Result<()> {
        Ok(())
    }

    fn invalidate(&self, _offset: u64, _size: u64) -> Result<()> {
        Ok(())
    }
}
