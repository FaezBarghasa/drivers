//! Memory management abstractions
//!
//! This module provides GPU memory allocation and management.

use crate::{Error, Result};

/// Memory type for allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// GPU-local memory, fastest for GPU access
    DeviceLocal,
    /// CPU-visible memory, can be mapped
    HostVisible,
    /// CPU-visible and coherent (no flush/invalidate needed)
    HostCoherent,
    /// CPU-visible and cached
    HostCached,
    /// Memory suitable for upload (host to device)
    Upload,
    /// Memory suitable for readback (device to host)
    Readback,
}

impl MemoryType {
    /// Check if this memory type is host-visible
    pub fn is_host_visible(&self) -> bool {
        matches!(
            self,
            MemoryType::HostVisible
                | MemoryType::HostCoherent
                | MemoryType::HostCached
                | MemoryType::Upload
                | MemoryType::Readback
        )
    }

    /// Check if this memory type requires flush/invalidate
    pub fn needs_flush(&self) -> bool {
        matches!(self, MemoryType::HostVisible | MemoryType::HostCached)
    }
}

/// Memory allocation info
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Size in bytes
    pub size: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Alignment requirement
    pub alignment: u64,
    /// Whether the allocation is dedicated
    pub dedicated: bool,
}

impl AllocationInfo {
    /// Create new allocation info
    pub fn new(size: u64, memory_type: MemoryType) -> Self {
        Self {
            size,
            memory_type,
            alignment: 256, // Default alignment
            dedicated: false,
        }
    }

    /// Set alignment
    pub fn alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set as dedicated allocation
    pub fn dedicated(mut self) -> Self {
        self.dedicated = true;
        self
    }
}

/// GPU memory block
pub trait Memory: Send + Sync {
    /// Get memory handle ID
    fn handle(&self) -> usize;

    /// Get memory size in bytes
    fn size(&self) -> u64;

    /// Get memory type
    fn memory_type(&self) -> MemoryType;

    /// Map memory for CPU access
    fn map(&self, offset: u64, size: u64) -> Result<*mut u8>;

    /// Unmap memory
    fn unmap(&self);

    /// Flush mapped memory range (for non-coherent memory)
    fn flush(&self, offset: u64, size: u64) -> Result<()>;

    /// Invalidate mapped memory range (for non-coherent memory)
    fn invalidate(&self, offset: u64, size: u64) -> Result<()>;
}

/// Memory allocator for suballocations
pub trait MemoryAllocator: Send + Sync {
    /// Allocate memory
    fn allocate(&self, info: &AllocationInfo) -> Result<Allocation>;

    /// Free memory
    fn free(&self, allocation: Allocation);

    /// Get statistics
    fn stats(&self) -> AllocatorStats;
}

/// Memory allocation handle
#[derive(Debug, Clone)]
pub struct Allocation {
    /// Memory block handle
    pub memory_handle: usize,
    /// Offset within the memory block
    pub offset: u64,
    /// Size of allocation
    pub size: u64,
    /// Mapped pointer (if mapped)
    pub mapped_ptr: Option<*mut u8>,
}

impl Allocation {
    /// Get the mapped pointer with offset applied
    pub fn mapped_data(&self) -> Option<*mut u8> {
        self.mapped_ptr
            .map(|p| unsafe { p.add(self.offset as usize) })
    }
}

/// Allocator statistics
#[derive(Debug, Clone, Default)]
pub struct AllocatorStats {
    /// Total allocated bytes
    pub allocated_bytes: u64,
    /// Number of allocations
    pub allocation_count: u32,
    /// Total memory reserved
    pub reserved_bytes: u64,
    /// Number of memory blocks
    pub block_count: u32,
}

/// Simple linear allocator for staging buffers
pub struct LinearAllocator {
    memory_handle: usize,
    base_ptr: *mut u8,
    size: u64,
    offset: core::sync::atomic::AtomicU64,
}

unsafe impl Send for LinearAllocator {}
unsafe impl Sync for LinearAllocator {}

impl LinearAllocator {
    /// Create a new linear allocator
    pub fn new(memory_handle: usize, base_ptr: *mut u8, size: u64) -> Self {
        Self {
            memory_handle,
            base_ptr,
            size,
            offset: core::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Allocate aligned memory
    pub fn allocate(&self, size: u64, alignment: u64) -> Option<Allocation> {
        use core::sync::atomic::Ordering;

        loop {
            let current = self.offset.load(Ordering::Relaxed);
            let aligned = (current + alignment - 1) & !(alignment - 1);
            let new_offset = aligned + size;

            if new_offset > self.size {
                return None;
            }

            if self
                .offset
                .compare_exchange_weak(current, new_offset, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return Some(Allocation {
                    memory_handle: self.memory_handle,
                    offset: aligned,
                    size,
                    mapped_ptr: Some(self.base_ptr),
                });
            }
        }
    }

    /// Reset the allocator
    pub fn reset(&self) {
        self.offset.store(0, core::sync::atomic::Ordering::SeqCst);
    }

    /// Get current usage
    pub fn used(&self) -> u64 {
        self.offset.load(core::sync::atomic::Ordering::Relaxed)
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> u64 {
        self.size - self.used()
    }
}
