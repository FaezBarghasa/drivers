//! MMIO register access helpers for Intel GPU driver.
//!
//! Provides safe wrappers around volatile MMIO reads and writes.
//! All accesses go through the device's MMIO base address.

use core::ptr::{read_volatile, write_volatile};

/// MMIO register accessor bound to a base virtual address.
pub struct MmioRegs {
    base: *mut u8,
    size: usize,
}

// SAFETY: The MMIO region is device memory; single-threaded access is enforced
// by the caller holding the device lock.
unsafe impl Send for MmioRegs {}
unsafe impl Sync for MmioRegs {}

impl MmioRegs {
    /// Create a new MMIO accessor.
    ///
    /// # Safety
    /// `base` must be a valid, mapped MMIO virtual address covering at least
    /// `size` bytes.
    pub unsafe fn new(base: *mut u8, size: usize) -> Self {
        Self { base, size }
    }

    /// Read a 32-bit register at `offset` bytes from the MMIO base.
    ///
    /// # Panics
    /// Panics if `offset + 4 > size`.
    pub fn read32(&self, offset: usize) -> u32 {
        assert!(offset + 4 <= self.size, "MMIO read out of range");
        unsafe { read_volatile(self.base.add(offset) as *const u32) }
    }

    /// Write a 32-bit register at `offset` bytes from the MMIO base.
    ///
    /// # Panics
    /// Panics if `offset + 4 > size`.
    pub fn write32(&self, offset: usize, val: u32) {
        assert!(offset + 4 <= self.size, "MMIO write out of range");
        unsafe { write_volatile(self.base.add(offset) as *mut u32, val) }
    }

    /// Read a 64-bit register at `offset` bytes from the MMIO base.
    ///
    /// # Panics
    /// Panics if `offset + 8 > size`.
    pub fn read64(&self, offset: usize) -> u64 {
        assert!(offset + 8 <= self.size, "MMIO read64 out of range");
        unsafe { read_volatile(self.base.add(offset) as *const u64) }
    }

    /// Write a 64-bit register at `offset` bytes from the MMIO base.
    ///
    /// # Panics
    /// Panics if `offset + 8 > size`.
    pub fn write64(&self, offset: usize, val: u64) {
        assert!(offset + 8 <= self.size, "MMIO write64 out of range");
        unsafe { write_volatile(self.base.add(offset) as *mut u64, val) }
    }

    /// Read-modify-write a 32-bit register: set bits in `set_mask`, clear bits
    /// in `clear_mask`.
    pub fn rmw32(&self, offset: usize, set_mask: u32, clear_mask: u32) {
        let val = self.read32(offset);
        self.write32(offset, (val & !clear_mask) | set_mask);
    }
}

// ─── Intel GPU register offsets ───────────────────────────────────────────────

/// GUC context policy register — controls EU (execution unit) core assignment.
pub const GUC_CONTEXT_POLICY: usize = 0x0000_C340;

/// Execution list submit port — used to submit work to specific EU slices.
pub const EXECLIST_SUBMIT_PORT: usize = 0x0002_230C;

/// Fence base register for VRAM partition 0 (repeated every 8 bytes per fence).
pub const FENCE_BASE: usize = 0x0010_0000;
pub const FENCE_STRIDE: usize = 8;
pub const FENCE_COUNT: usize = 16;

/// VRAM fence lower 32 bits: [31:12] = base >> 12, [2:1] = tile mode, [0] = valid.
pub const FENCE_VALID: u32 = 1 << 0;
pub const FENCE_TILE_XMAJOR: u32 = 1 << 1;
