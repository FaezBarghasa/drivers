//! VirtIO-Net device emulation.
//!
//! Emulates a VirtIO network device (device type 1) with two virtqueues:
//!   - Queue 0: RX (device → driver): delivers packets to the guest
//!   - Queue 1: TX (driver → device): receives packets from the guest
//!
//! Received packets are delivered via a callback; transmitted packets are
//! forwarded to the host network stack via the `TxSink` trait.

use crate::{
    VIRTIO_F_VERSION_1, VirtioDevice,
    queue::{VirtqAvail, VirtqDesc, VirtqUsed, Virtqueue},
};
use alloc::{boxed::Box, vec::Vec};

/// Device type ID for VirtIO-Net.
pub const VIRTIO_NET_DEVICE_TYPE: u32 = 1;

/// VirtIO-Net feature bits.
pub const VIRTIO_NET_F_MAC: u64 = 1 << 5;
pub const VIRTIO_NET_F_STATUS: u64 = 1 << 16;
pub const VIRTIO_NET_F_MRG_RXBUF: u64 = 1 << 15;

/// VirtIO-Net MMIO register offsets.
const REG_MAGIC: u32 = 0x000;
const REG_VERSION: u32 = 0x004;
const REG_DEVICE_ID: u32 = 0x008;
const REG_VENDOR_ID: u32 = 0x00C;
const REG_DEVICE_FEATURES: u32 = 0x010;
const REG_QUEUE_SEL: u32 = 0x030;
const REG_QUEUE_NUM_MAX: u32 = 0x034;
const REG_DEVICE_STATUS: u32 = 0x070;
const REG_CONFIG_MAC_LO: u32 = 0x100;
const REG_CONFIG_MAC_HI: u32 = 0x104;

/// VirtIO-Net packet header (12 bytes).
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct VirtioNetHdr {
    pub flags: u8,
    pub gso_type: u8,
    pub hdr_len: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
    pub num_buffers: u16,
}

/// Sink for transmitted packets: forwards them to the host network stack.
pub trait TxSink: Send {
    fn send(&mut self, packet: &[u8]);
}

/// Emulated VirtIO-Net device.
pub struct VirtioNet {
    mac: [u8; 6],
    status: u16,
    tx_sink: Option<Box<dyn TxSink>>,
    /// Packets queued for delivery to the guest RX queue.
    rx_queue: Vec<Vec<u8>>,
    /// MMIO configuration register state.
    device_status: u8,
    queue_sel: u32,
    /// RX virtqueue (queue 0), set up by the driver via MMIO.
    rx_vq: Option<Virtqueue>,
    /// TX virtqueue (queue 1), set up by the driver via MMIO.
    tx_vq: Option<Virtqueue>,
    /// Pending queue setup: desc/avail/used GPA for the selected queue.
    queue_desc_gpa: u64,
    queue_avail_gpa: u64,
    queue_used_gpa: u64,
    queue_num: u16,
    /// Guest-to-host physical address translation function.
    /// Set by the VMM before calling notify().
    guest_to_host: Option<Box<dyn Fn(u64) -> *mut u8 + Send>>,
}

impl VirtioNet {
    /// Create a new VirtIO-Net device with the given MAC address.
    pub fn new(mac: [u8; 6]) -> Self {
        Self {
            mac,
            status: 1, // VIRTIO_NET_S_LINK_UP
            tx_sink: None,
            rx_queue: Vec::new(),
            device_status: 0,
            queue_sel: 0,
            rx_vq: None,
            tx_vq: None,
            queue_desc_gpa: 0,
            queue_avail_gpa: 0,
            queue_used_gpa: 0,
            queue_num: 256,
            guest_to_host: None,
        }
    }

    /// Attach a TX sink for forwarding guest-transmitted packets.
    pub fn set_tx_sink(&mut self, sink: Box<dyn TxSink>) {
        self.tx_sink = Some(sink);
    }

    /// Enqueue a packet for delivery to the guest RX virtqueue.
    pub fn inject_rx(&mut self, packet: Vec<u8>) {
        self.rx_queue.push(packet);
    }

    /// Set the guest-to-host address translation function.
    ///
    /// The VMM must call this before the first `notify()` so that virtqueue
    /// descriptor buffer addresses can be resolved to host-virtual pointers.
    pub fn set_guest_to_host(&mut self, f: Box<dyn Fn(u64) -> *mut u8 + Send>) {
        self.guest_to_host = Some(f);
    }

    /// Finalize virtqueue setup for the currently selected queue.
    ///
    /// Called by the VMM after the driver writes QueueReady (offset 0x044).
    pub fn activate_queue(&mut self) {
        let g2h = match &self.guest_to_host {
            Some(f) => f,
            None => return,
        };
        let desc = g2h(self.queue_desc_gpa) as *mut VirtqDesc;
        let avail = g2h(self.queue_avail_gpa) as *mut VirtqAvail;
        let used = g2h(self.queue_used_gpa) as *mut VirtqUsed;
        let vq = unsafe { Virtqueue::new(self.queue_num, desc, avail, used) };
        match self.queue_sel {
            0 => self.rx_vq = Some(vq),
            1 => self.tx_vq = Some(vq),
            _ => {}
        }
    }

    /// Process the TX virtqueue: drain all available descriptor chains and
    /// forward each packet to the TX sink.
    fn process_tx(&mut self) {
        let g2h = match &self.guest_to_host {
            Some(f) => f as *const _ as *const (dyn Fn(u64) -> *mut u8 + Send),
            None => return,
        };
        let tx_vq = match &mut self.tx_vq {
            Some(vq) => vq,
            None => return,
        };

        while let Some(head) = tx_vq.pop_avail() {
            // Walk the descriptor chain to collect all buffer segments.
            let segments = unsafe {
                tx_vq.walk_chain(head, |gpa| {
                    // SAFETY: g2h is valid for the lifetime of this call.
                    (*g2h)(gpa)
                })
            };

            // Assemble the packet (skip the VirtioNetHdr in the first segment).
            let mut packet: Vec<u8> = Vec::new();
            let hdr_size = core::mem::size_of::<VirtioNetHdr>();
            let mut skip = hdr_size;
            for (ptr, len, writable) in &segments {
                if *writable {
                    // TX descriptors should be read-only; skip writable ones.
                    continue;
                }
                let len = *len as usize;
                let data = unsafe { core::slice::from_raw_parts(*ptr, len) };
                if skip > 0 {
                    let advance = skip.min(len);
                    packet.extend_from_slice(&data[advance..]);
                    skip -= advance;
                } else {
                    packet.extend_from_slice(data);
                }
            }

            // Forward to the TX sink.
            if !packet.is_empty() {
                if let Some(ref mut sink) = self.tx_sink {
                    sink.send(&packet);
                }
            }

            // Return the descriptor chain to the used ring.
            tx_vq.push_used(head as u32, 0);
        }
    }

    /// Process the RX virtqueue: deliver queued packets to the guest.
    fn process_rx(&mut self) {
        let g2h = match &self.guest_to_host {
            Some(f) => f as *const _ as *const (dyn Fn(u64) -> *mut u8 + Send),
            None => return,
        };
        let rx_vq = match &mut self.rx_vq {
            Some(vq) => vq,
            None => return,
        };

        while let Some(packet) = self.rx_queue.first().cloned() {
            let head = match rx_vq.pop_avail() {
                Some(h) => h,
                None => break,
            };

            let segments = unsafe { rx_vq.walk_chain(head, |gpa| (*g2h)(gpa)) };

            // Write VirtioNetHdr + packet data into the writable guest buffers.
            let hdr = VirtioNetHdr::default();
            let hdr_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    &hdr as *const VirtioNetHdr as *const u8,
                    core::mem::size_of::<VirtioNetHdr>(),
                )
            };

            let mut src_iter = hdr_bytes.iter().chain(packet.iter());
            let mut bytes_written: u32 = 0;

            for (ptr, len, writable) in &segments {
                if !writable {
                    continue;
                }
                let len = *len as usize;
                let dst = unsafe { core::slice::from_raw_parts_mut(*ptr, len) };
                for byte in dst.iter_mut() {
                    match src_iter.next() {
                        Some(b) => {
                            *byte = *b;
                            bytes_written += 1;
                        }
                        None => break,
                    }
                }
            }

            rx_vq.push_used(head as u32, bytes_written);
            self.rx_queue.remove(0);
        }
    }
}

impl VirtioDevice for VirtioNet {
    fn device_type(&self) -> u32 {
        VIRTIO_NET_DEVICE_TYPE
    }

    fn features(&self) -> u64 {
        VIRTIO_F_VERSION_1 | VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS
    }

    fn mmio_write(&mut self, offset: u32, val: u32) {
        match offset {
            REG_DEVICE_STATUS => {
                self.device_status = val as u8;
            }
            REG_QUEUE_SEL => {
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
            REG_MAGIC => 0x74726976,
            REG_VERSION => 2,
            REG_DEVICE_ID => self.device_type(),
            REG_VENDOR_ID => 0x1AF4,
            REG_DEVICE_FEATURES => (self.features() & 0xFFFF_FFFF) as u32,
            REG_QUEUE_NUM_MAX => 256,
            REG_DEVICE_STATUS => self.device_status as u32,
            REG_CONFIG_MAC_LO => {
                u32::from_le_bytes([self.mac[0], self.mac[1], self.mac[2], self.mac[3]])
            }
            REG_CONFIG_MAC_HI => u32::from_le_bytes([
                self.mac[4],
                self.mac[5],
                (self.status & 0xFF) as u8,
                (self.status >> 8) as u8,
            ]),
            _ => 0,
        }
    }

    fn notify(&mut self, queue_index: u32) {
        match queue_index {
            0 => self.process_rx(),
            1 => self.process_tx(),
            _ => {}
        }
    }
}
