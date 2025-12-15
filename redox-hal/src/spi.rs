//! SPI (Serial Peripheral Interface) HAL traits
//!
//! This module defines the SPI abstraction for serial communication.

use crate::error::Result;

/// SPI mode (clock polarity and phase)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiMode {
    /// CPOL=0, CPHA=0: Clock idle low, sample on rising edge
    Mode0,
    /// CPOL=0, CPHA=1: Clock idle low, sample on falling edge
    Mode1,
    /// CPOL=1, CPHA=0: Clock idle high, sample on falling edge
    Mode2,
    /// CPOL=1, CPHA=1: Clock idle high, sample on rising edge
    Mode3,
}

impl SpiMode {
    /// Get clock polarity
    pub fn polarity(&self) -> ClockPolarity {
        match self {
            SpiMode::Mode0 | SpiMode::Mode1 => ClockPolarity::IdleLow,
            SpiMode::Mode2 | SpiMode::Mode3 => ClockPolarity::IdleHigh,
        }
    }

    /// Get clock phase
    pub fn phase(&self) -> ClockPhase {
        match self {
            SpiMode::Mode0 | SpiMode::Mode2 => ClockPhase::CaptureOnFirstTransition,
            SpiMode::Mode1 | SpiMode::Mode3 => ClockPhase::CaptureOnSecondTransition,
        }
    }
}

/// Clock polarity (CPOL)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockPolarity {
    /// Clock is low when idle
    IdleLow,
    /// Clock is high when idle
    IdleHigh,
}

/// Clock phase (CPHA)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockPhase {
    /// Data sampled on first clock transition
    CaptureOnFirstTransition,
    /// Data sampled on second clock transition
    CaptureOnSecondTransition,
}

/// Bit order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    /// Most significant bit first
    MsbFirst,
    /// Least significant bit first
    LsbFirst,
}

/// SPI configuration
#[derive(Debug, Clone, Copy)]
pub struct SpiConfig {
    /// SPI mode
    pub mode: SpiMode,
    /// Bit order
    pub bit_order: BitOrder,
    /// Clock frequency in Hz
    pub frequency: u32,
    /// Word size in bits (typically 8 or 16)
    pub word_size: u8,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            mode: SpiMode::Mode0,
            bit_order: BitOrder::MsbFirst,
            frequency: 1_000_000, // 1 MHz
            word_size: 8,
        }
    }
}

/// SPI bus trait
pub trait SpiBus {
    /// Error type
    type Error;

    /// Configure the SPI bus
    fn configure(&mut self, config: SpiConfig) -> Result<(), Self::Error>;

    /// Transfer data (simultaneous read/write)
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error>;

    /// Transfer data in place
    fn transfer_in_place(&mut self, data: &mut [u8]) -> Result<(), Self::Error>;

    /// Write data without reading
    fn write(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Read data (sends zeros)
    fn read(&mut self, data: &mut [u8]) -> Result<(), Self::Error>;

    /// Flush any pending operations
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// SPI bus with chip select control
pub trait SpiBusWithCs: SpiBus {
    /// Assert chip select (active low)
    fn cs_assert(&mut self) -> Result<(), Self::Error>;

    /// Deassert chip select
    fn cs_deassert(&mut self) -> Result<(), Self::Error>;

    /// Execute a transaction with automatic CS control
    fn transaction<R>(&mut self, f: impl FnOnce(&mut Self) -> Result<R, Self::Error>) -> Result<R, Self::Error> {
        self.cs_assert()?;
        let result = f(self);
        self.cs_deassert()?;
        result
    }
}

/// SPI device (bus + specific chip select)
pub trait SpiDevice {
    /// Error type
    type Error;
    /// Bus type
    type Bus: SpiBus;

    /// Access the underlying bus (with CS asserted)
    fn transaction<R, F>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut Self::Bus) -> Result<R, <Self::Bus as SpiBus>::Error>;

    /// Write to the device
    fn write(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Read from the device
    fn read(&mut self, data: &mut [u8]) -> Result<(), Self::Error>;

    /// Transfer to/from the device
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error>;
}

/// SPI controller managing multiple buses
pub trait SpiController {
    /// Error type
    type Error;
    /// Bus type
    type Bus: SpiBus;

    /// Get an SPI bus
    fn bus(&mut self, bus_number: u8) -> Result<Self::Bus, Self::Error>;

    /// Get the number of available buses
    fn bus_count(&self) -> u8;
}

/// Async SPI bus trait
#[cfg(feature = "async")]
pub trait AsyncSpiBus {
    /// Error type
    type Error;

    /// Transfer data asynchronously
    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error>;

    /// Write data asynchronously
    async fn write(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Read data asynchronously
    async fn read(&mut self, data: &mut [u8]) -> Result<(), Self::Error>;

    /// Flush asynchronously
    async fn flush(&mut self) -> Result<(), Self::Error>;
}
