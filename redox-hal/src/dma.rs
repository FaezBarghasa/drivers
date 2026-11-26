//! DMA (Direct Memory Access) HAL traits

use crate::error::Result;

/// DMA transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Peripheral to memory
    PeripheralToMemory,
    /// Memory to peripheral
    MemoryToPeripheral,
    /// Memory to memory
    MemoryToMemory,
}

/// DMA data size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDataSize {
    /// Byte (8-bit)
    Byte,
    /// Half-word (16-bit)
    HalfWord,
    /// Word (32-bit)
    Word,
}

/// DMA priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaPriority {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// DMA configuration
#[derive(Debug, Clone, Copy)]
pub struct DmaConfig {
    /// Transfer direction
    pub direction: DmaDirection,
    /// Source data size
    pub source_size: DmaDataSize,
    /// Destination data size
    pub dest_size: DmaDataSize,
    /// Increment source address
    pub source_increment: bool,
    /// Increment destination address
    pub dest_increment: bool,
    /// Circular mode (auto-restart)
    pub circular: bool,
    /// Priority level
    pub priority: DmaPriority,
}

impl Default for DmaConfig {
    fn default() -> Self {
        Self {
            direction: DmaDirection::PeripheralToMemory,
            source_size: DmaDataSize::Byte,
            dest_size: DmaDataSize::Byte,
            source_increment: false,
            dest_increment: true,
            circular: false,
            priority: DmaPriority::Medium,
        }
    }
}

/// DMA transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaStatus {
    /// Transfer not started
    Idle,
    /// Transfer in progress
    InProgress,
    /// Transfer complete
    Complete,
    /// Transfer error
    Error,
}

/// DMA channel trait
pub trait DmaChannel {
    /// Error type
    type Error;

    /// Configure the DMA channel
    fn configure(&mut self, config: DmaConfig) -> Result<(), Self::Error>;

    /// Start a transfer
    fn start(&mut self, src: usize, dst: usize, count: usize) -> Result<(), Self::Error>;

    /// Stop the transfer
    fn stop(&mut self) -> Result<(), Self::Error>;

    /// Get transfer status
    fn status(&self) -> DmaStatus;

    /// Get remaining transfer count
    fn remaining(&self) -> usize;

    /// Check if transfer is complete
    fn is_complete(&self) -> bool {
        self.status() == DmaStatus::Complete
    }

    /// Wait for transfer to complete
    fn wait(&mut self) -> Result<(), Self::Error>;

    /// Enable transfer complete interrupt
    fn enable_interrupt(&mut self);

    /// Disable interrupt
    fn disable_interrupt(&mut self);

    /// Clear interrupt flag
    fn clear_interrupt(&mut self);

    /// Set interrupt handler
    fn set_handler(&mut self, handler: fn());
}

/// DMA controller trait
pub trait Dma {
    /// Error type
    type Error;
    /// Channel type
    type Channel: DmaChannel;

    /// Get a DMA channel
    fn channel(&mut self, channel_number: u8) -> Result<Self::Channel, Self::Error>;

    /// Get the number of available channels
    fn channel_count(&self) -> u8;
}

/// Safe DMA buffer wrapper
pub struct DmaBuffer<T, const N: usize> {
    data: [T; N],
}

impl<T: Copy + Default, const N: usize> DmaBuffer<T, N> {
    /// Create a new DMA buffer
    pub fn new() -> Self {
        Self {
            data: [T::default(); N],
        }
    }

    /// Get the buffer address
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Get the buffer address (mutable)
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }

    /// Get the buffer length
    pub const fn len(&self) -> usize {
        N
    }

    /// Get a slice of the buffer
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Get a mutable slice of the buffer
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T: Copy + Default, const N: usize> Default for DmaBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
