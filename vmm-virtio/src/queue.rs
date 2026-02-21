//! Split virtqueue implementation.
//!
//! A split virtqueue consists of three guest-memory regions:
//!   - **Descriptor table**: array of `VirtqDesc` (16 bytes each)
//!   - **Available ring**: driver → device notification of new descriptors
//!   - **Used ring**: device → driver notification of completed descriptors
//!
//! This implementation operates on host-virtual pointers to guest memory,
//! which the VMM maps before calling `Virtqueue::new`.

use alloc::vec::Vec;
use core::sync::atomic::{fence, Ordering};

/// A single virtqueue descriptor.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqDesc {
    /// Guest physical address of the buffer.
    pub addr: u64,
    /// Length of the buffer in bytes.
    pub len: u32,
    /// Descriptor flags: VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE, etc.
    pub flags: u16,
    /// Index of the next descriptor in the chain (if NEXT flag is set).
    pub next: u16,
}

pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;

/// Available ring header (driver → device).
#[repr(C)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; 0], // variable-length; access via raw pointer arithmetic
}

/// Used ring element.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

/// Used ring header (device → driver).
#[repr(C)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; 0], // variable-length
}

/// A split virtqueue backed by host-virtual pointers to guest memory.
pub struct Virtqueue {
    size: u16,
    desc_table: *mut VirtqDesc,
    avail_ring: *mut VirtqAvail,
    used_ring: *mut VirtqUsed,
    last_avail: u16,
}

// SAFETY: The VMM holds the VM lock while calling virtqueue methods.
unsafe impl Send for Virtqueue {}

impl Virtqueue {
    /// Create a new virtqueue from host-virtual pointers to the three guest regions.
    ///
    /// # Safety
    /// All three pointers must be valid host-virtual addresses mapping the
    /// corresponding guest-physical regions for at least `size` entries.
    pub unsafe fn new(
        size: u16,
        desc_table: *mut VirtqDesc,
        avail_ring: *mut VirtqAvail,
        used_ring: *mut VirtqUsed,
    ) -> Self {
        Self {
            size,
            desc_table,
            avail_ring,
            used_ring,
            last_avail: 0,
        }
    }

    /// Return the number of new descriptor chains available for processing.
    pub fn available_count(&self) -> u16 {
        let avail_idx = unsafe { (*self.avail_ring).idx };
        avail_idx.wrapping_sub(self.last_avail)
    }

    /// Pop the next available descriptor chain head index.
    ///
    /// Returns `None` if no new descriptors are available.
    pub fn pop_avail(&mut self) -> Option<u16> {
        if self.available_count() == 0 {
            return None;
        }
        let ring_ptr = self.avail_ring as *const u8;
        // ring[] starts at offset 4 from VirtqAvail base.
        let ring_entry_ptr =
            unsafe { ring_ptr.add(4 + (self.last_avail % self.size) as usize * 2) as *const u16 };
        let head = unsafe { core::ptr::read_volatile(ring_entry_ptr) };
        self.last_avail = self.last_avail.wrapping_add(1);
        Some(head)
    }

    /// Walk a descriptor chain starting at `head`, collecting all buffer segments.
    ///
    /// Returns a `Vec` of `(host_virt_addr, len, writable)` tuples.
    ///
    /// # Safety
    /// The descriptor table must be valid and the chain must not be cyclic.
    pub unsafe fn walk_chain(
        &self,
        head: u16,
        guest_to_host: impl Fn(u64) -> *mut u8,
    ) -> Vec<(*mut u8, u32, bool)> {
        let mut result = Vec::new();
        let mut idx = head;
        loop {
            let desc = unsafe { &*self.desc_table.add(idx as usize) };
            let host_ptr = guest_to_host(desc.addr);
            let writable = desc.flags & VIRTQ_DESC_F_WRITE != 0;
            result.push((host_ptr, desc.len, writable));
            if desc.flags & VIRTQ_DESC_F_NEXT == 0 {
                break;
            }
            idx = desc.next;
        }
        result
    }

    /// Push a used ring entry, notifying the driver that descriptor `id` is done.
    ///
    /// `bytes_written` is the number of bytes written to writable buffers.
    pub fn push_used(&mut self, id: u32, bytes_written: u32) {
        let used_ptr = self.used_ring as *mut u8;
        // used.idx is at offset 2; ring[] starts at offset 4.
        let used_idx = unsafe { core::ptr::read_volatile(used_ptr.add(2) as *const u16) };
        let slot = (used_idx % self.size) as usize;
        let elem_ptr = unsafe {
            used_ptr.add(4 + slot * core::mem::size_of::<VirtqUsedElem>()) as *mut VirtqUsedElem
        };
        unsafe {
            core::ptr::write_volatile(
                elem_ptr,
                VirtqUsedElem {
                    id,
                    len: bytes_written,
                },
            );
        }
        fence(Ordering::SeqCst);
        unsafe {
            core::ptr::write_volatile(used_ptr.add(2) as *mut u16, used_idx.wrapping_add(1));
        }
    }
}
