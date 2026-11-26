//! VirtIO-Block device emulation.
//!
//! Emulates a VirtIO block device (device type 2) backed by an in-memory
//! or host-file storage backend.
//!
//! The block device exposes a single virtqueue (queue 0) for read/write
//! requests. Each request consists of:
//!   1. A `VirtioBlkReq` header descriptor (read-only)
//!   2. One or more data descriptors (read-only for WRITE, writable for READ)
//!   3. A status byte descriptor (writable)

use crate::{
    VIRTIO_F_VERSION_1, VirtioDevice,
    queue::{VirtqAvail, VirtqDesc, VirtqUsed, Virtqueue},
};
use alloc::{boxed::Box, vec, vec::Vec};

/// Device type ID for VirtIO-Block.
pub const VIRTIO_BLK_DEVICE_TYPE: u32 = 2;

/// VirtIO-Block feature bits.
pub const VIRTIO_BLK_F_SIZE_MAX: u64 = 1 << 1;
pub const VIRTIO_BLK_F_SEG_MAX: u64 = 1 << 2;
pub const VIRTIO_BLK_F_BLK_SIZE: u64 = 1 << 6;

/// Block request types.
pub const VIRTIO_BLK_T_IN: u32 = 0; // Read
pub const VIRTIO_BLK_T_OUT: u32 = 1; // Write

/// Block request status codes.
pub const VIRTIO_BLK_S_OK: u8 = 0;
pub const VIRTIO_BLK_S_IOERR: u8 = 1;
pub const VIRTIO_BLK_S_UNSUPP: u8 = 2;

/// VirtIO-Block request header (16 bytes).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioBlkReq {
    pub req_type: u32,
    pub reserved: u32,
    pub sector: u64,
}

/// Backend storage trait: implemented by in-memory or file-backed stores.
pub trait BlockBackend: Send {
    /// Return the total number of 512-byte sectors.
    fn sector_count(&self) -> u64;
    /// Read `count` sectors starting at `sector` into `buf`.
    fn read_sectors(&mut self, sector: u64, count: u64, buf: &mut [u8]) -> bool;
    /// Write `count` sectors starting at `sector` from `buf`.
    fn write_sectors(&mut self, sector: u64, count: u64, buf: &[u8]) -> bool;
}

/// In-memory block backend for testing.
pub struct MemBackend {
    data: Vec<u8>,
}

impl MemBackend {
    /// Create a new in-memory backend with `sectors` × 512 bytes, zeroed.
    pub fn new(sectors: u64) -> Self {
        Self {
            data: vec![0u8; (sectors * 512) as usize],
        }
    }
}

impl BlockBackend for MemBackend {
    fn sector_count(&self) -> u64 {
        self.data.len() as u64 / 512
    }

    fn read_sectors(&mut self, sector: u64, count: u64, buf: &mut [u8]) -> bool {
        let start = (sector * 512) as usize;
        let end = start + (count * 512) as usize;
        if end > self.data.len() || buf.len() < end - start {
            return false;
        }
        buf[..end - start].copy_from_slice(&self.data[start..end]);
        true
    }

    fn write_sectors(&mut self, sector: u64, count: u64, buf: &[u8]) -> bool {
        let start = (sector * 512) as usize;
        let end = start + (count * 512) as usize;
        if end > self.data.len() || buf.len() < end - start {
            return false;
        }
        self.data[start..end].copy_from_slice(&buf[..end - start]);
        true
    }
}

/// Emulated VirtIO-Block device.
pub struct VirtioBlock {
    backend: Box<dyn BlockBackend>,
    device_status: u8,
    queue_sel: u32,
    /// Request virtqueue (queue 0).
    req_vq: Option<Virtqueue>,
    /// Pending queue setup GPAs.
    queue_desc_gpa: u64,
    queue_avail_gpa: u64,
    queue_used_gpa: u64,
    queue_num: u16,
    /// Guest-to-host address translation.
    guest_to_host: Option<Box<dyn Fn(u64) -> *mut u8 + Send>>,
}

impl VirtioBlock {
    pub fn new(backend: Box<dyn BlockBackend>) -> Self {
        Self {
            backend,
            device_status: 0,
            queue_sel: 0,
            req_vq: None,
            queue_desc_gpa: 0,
            queue_avail_gpa: 0,
            queue_used_gpa: 0,
            queue_num: 256,
            guest_to_host: None,
        }
    }

    /// Set the guest-to-host address translation function.
    pub fn set_guest_to_host(&mut self, f: Box<dyn Fn(u64) -> *mut u8 + Send>) {
        self.guest_to_host = Some(f);
    }

    /// Finalize virtqueue setup for the currently selected queue.
    pub fn activate_queue(&mut self) {
        let g2h = match &self.guest_to_host {
            Some(f) => f,
            None => return,
        };
        if self.queue_sel != 0 {
            return;
        }
        let desc = g2h(self.queue_desc_gpa) as *mut VirtqDesc;
        let avail = g2h(self.queue_avail_gpa) as *mut VirtqAvail;
        let used = g2h(self.queue_used_gpa) as *mut VirtqUsed;
        self.req_vq = Some(unsafe { Virtqueue::new(self.queue_num, desc, avail, used) });
    }

    /// Process all pending block requests from the request virtqueue.
    fn process_requests(&mut self) {
        let g2h = match &self.guest_to_host {
            Some(f) => f as *const _ as *const (dyn Fn(u64) -> *mut u8 + Send),
            None => return,
        };
        let req_vq = match &mut self.req_vq {
            Some(vq) => vq,
            None => return,
        };

        while let Some(head) = req_vq.pop_avail() {
            let segments = unsafe { req_vq.walk_chain(head, |gpa| (*g2h)(gpa)) };

            // Segment layout:
            //   [0]: VirtioBlkReq header (read-only, 16 bytes)
            //   [1..n-1]: data buffers (read-only for WRITE, writable for READ)
            //   [n]: status byte (writable, 1 byte)
            if segments.len() < 3 {
                req_vq.push_used(head as u32, 1);
                continue;
            }

            let (hdr_ptr, hdr_len, _) = segments[0];
            if hdr_len < core::mem::size_of::<VirtioBlkReq>() as u32 {
                req_vq.push_used(head as u32, 1);
                continue;
            }

            let hdr: VirtioBlkReq =
                unsafe { core::ptr::read_volatile(hdr_ptr as *const VirtioBlkReq) };

            let status_seg = segments.last().unwrap();
            let status_ptr = status_seg.0;

            let data_segs = &segments[1..segments.len() - 1];

            let status = match hdr.req_type {
                VIRTIO_BLK_T_IN => {
                    // READ: write data into guest writable buffers.
                    let mut sector = hdr.sector;
                    let mut ok = true;
                    for (ptr, len, writable) in data_segs {
                        if !writable {
                            ok = false;
                            break;
                        }
                        let count = (*len as u64).div_ceil(512);
                        let buf = unsafe { core::slice::from_raw_parts_mut(*ptr, *len as usize) };
                        if !self.backend.read_sectors(sector, count, buf) {
                            ok = false;
                            break;
                        }
                        sector += count;
                    }
                    if ok {
                        VIRTIO_BLK_S_OK
                    } else {
                        VIRTIO_BLK_S_IOERR
                    }
                }
                VIRTIO_BLK_T_OUT => {
                    // WRITE: read data from guest read-only buffers.
                    let mut sector = hdr.sector;
                    let mut ok = true;
                    for (ptr, len, writable) in data_segs {
                        if *writable {
                            ok = false;
                            break;
                        }
                        let count = (*len as u64).div_ceil(512);
                        let buf = unsafe { core::slice::from_raw_parts(*ptr, *len as usize) };
                        if !self.backend.write_sectors(sector, count, buf) {
                            ok = false;
                            break;
                        }
                        sector += count;
                    }
                    if ok {
                        VIRTIO_BLK_S_OK
                    } else {
                        VIRTIO_BLK_S_IOERR
                    }
                }
                _ => VIRTIO_BLK_S_UNSUPP,
            };

            unsafe {
                core::ptr::write_volatile(status_ptr, status);
            }

            // bytes_written = data bytes + 1 status byte.
            let data_bytes: u32 = data_segs.iter().map(|(_, len, _)| *len).sum();
            req_vq.push_used(head as u32, data_bytes + 1);
        }
    }
}

impl VirtioDevice for VirtioBlock {
    fn device_type(&self) -> u32 {
        VIRTIO_BLK_DEVICE_TYPE
    }

    fn features(&self) -> u64 {
        VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_BLK_SIZE
    }

    fn mmio_write(&mut self, offset: u32, val: u32) {
        match offset {
            // DeviceStatus
            0x070 => {
                self.device_status = val as u8;
            }
            // QueueSel
            0x030 => {
                self.queue_sel = val;
            }
            // QueueNum
            0x038 => {
                self.queue_num = val as u16;
            }
            // QueueDescLow
            0x080 => {
                self.queue_desc_gpa = (self.queue_desc_gpa & 0xFFFF_FFFF_0000_0000) | val as u64;
            }
            // QueueDescHigh
            0x084 => {
                self.queue_desc_gpa =
                    (self.queue_desc_gpa & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
            }
            // QueueAvailLow
            0x090 => {
                self.queue_avail_gpa = (self.queue_avail_gpa & 0xFFFF_FFFF_0000_0000) | val as u64;
            }
            // QueueAvailHigh
            0x094 => {
                self.queue_avail_gpa =
                    (self.queue_avail_gpa & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
            }
            // QueueUsedLow
            0x0A0 => {
                self.queue_used_gpa = (self.queue_used_gpa & 0xFFFF_FFFF_0000_0000) | val as u64;
            }
            // QueueUsedHigh
            0x0A4 => {
                self.queue_used_gpa =
                    (self.queue_used_gpa & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32);
            }
            // QueueReady
            0x044 => {
                if val == 1 {
                    self.activate_queue();
                }
            }
            _ => {}
        }
    }

    fn mmio_read(&self, offset: u32) -> u32 {
        match offset {
            // MagicValue
            0x000 => 0x74726976,
            // Version
            0x004 => 2,
            // DeviceID
            0x008 => self.device_type(),
            // VendorID
            0x00C => 0x1AF4,
            // DeviceFeatures
            0x010 => (self.features() & 0xFFFF_FFFF) as u32,
            // QueueNumMax
            0x034 => 256,
            // DeviceStatus
            0x070 => self.device_status as u32,
            // Config: capacity (sectors) low 32 bits
            0x100 => (self.backend.sector_count() & 0xFFFF_FFFF) as u32,
            // Config: capacity (sectors) high 32 bits
            0x104 => (self.backend.sector_count() >> 32) as u32,
            // Config: block size (512)
            0x114 => 512,
            _ => 0,
        }
    }

    fn notify(&mut self, queue_index: u32) {
        if queue_index == 0 {
            self.process_requests();
        }
    }
}
