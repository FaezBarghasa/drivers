//! VirtIO device emulation for the vmm: scheme.
//!
//! Provides:
//!   - `VirtioDevice` trait for emulated VirtIO devices
//!   - Split virtqueue implementation
//!   - VirtIO-Net device emulation
//!   - VirtIO-Block device emulation

#![no_std]
extern crate alloc;

pub mod block;
pub mod net;
pub mod queue;

// ─── VirtIO constants ─────────────────────────────────────────────────────────

pub const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_STATUS_FAILED: u8 = 128;

pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;

// ─── VirtIO device trait ──────────────────────────────────────────────────────

/// Trait implemented by all emulated VirtIO devices.
pub trait VirtioDevice: Send {
    /// Return the VirtIO device type ID.
    fn device_type(&self) -> u32;

    /// Return the device feature bits.
    fn features(&self) -> u64;

    /// Handle a MMIO write to the device configuration space.
    ///
    /// `offset` is relative to the device's MMIO base.
    fn mmio_write(&mut self, offset: u32, val: u32);

    /// Handle a MMIO read from the device configuration space.
    ///
    /// `offset` is relative to the device's MMIO base.
    fn mmio_read(&self, offset: u32) -> u32;

    /// Notify the device that virtqueue `queue_index` has new descriptors.
    fn notify(&mut self, queue_index: u32);
}
