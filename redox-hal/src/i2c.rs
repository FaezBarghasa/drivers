//! I2C (Inter-Integrated Circuit) HAL traits
//!
//! This module defines the I2C abstraction for two-wire serial communication.

use crate::error::Result;

/// I2C address (7-bit or 10-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cAddress {
    /// 7-bit address (most common)
    SevenBit(u8),
    /// 10-bit address
    TenBit(u16),
}

impl I2cAddress {
    /// Create a 7-bit address
    pub fn seven_bit(addr: u8) -> Self {
        debug_assert!(addr < 128, "7-bit address must be < 128");
        I2cAddress::SevenBit(addr)
    }

    /// Create a 10-bit address
    pub fn ten_bit(addr: u16) -> Self {
        debug_assert!(addr < 1024, "10-bit address must be < 1024");
        I2cAddress::TenBit(addr)
    }

    /// Get the raw address value
    pub fn raw(&self) -> u16 {
        match self {
            I2cAddress::SevenBit(addr) => *addr as u16,
            I2cAddress::TenBit(addr) => *addr,
        }
    }
}

impl From<u8> for I2cAddress {
    fn from(addr: u8) -> Self {
        I2cAddress::SevenBit(addr)
    }
}

/// I2C speed mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cSpeed {
    /// Standard mode (100 kHz)
    Standard,
    /// Fast mode (400 kHz)
    Fast,
    /// Fast mode plus (1 MHz)
    FastPlus,
    /// High speed mode (3.4 MHz)  
    HighSpeed,
    /// Custom frequency
    Custom(u32),
}

impl I2cSpeed {
    /// Get the frequency in Hz
    pub fn frequency_hz(&self) -> u32 {
        match self {
            I2cSpeed::Standard => 100_000,
            I2cSpeed::Fast => 400_000,
            I2cSpeed::FastPlus => 1_000_000,
            I2cSpeed::HighSpeed => 3_400_000,
            I2cSpeed::Custom(freq) => *freq,
        }
    }
}

/// I2C configuration
#[derive(Debug, Clone, Copy)]
pub struct I2cConfig {
    /// Bus speed
    pub speed: I2cSpeed,
    /// Enable clock stretching
    pub clock_stretching: bool,
    /// Enable 10-bit addressing
    pub ten_bit_addressing: bool,
}

impl Default for I2cConfig {
    fn default() -> Self {
        Self {
            speed: I2cSpeed::Standard,
            clock_stretching: true,
            ten_bit_addressing: false,
        }
    }
}

/// I2C operation for transactions
#[derive(Debug)]
pub enum I2cOperation<'a> {
    /// Read data from device
    Read(&'a mut [u8]),
    /// Write data to device
    Write(&'a [u8]),
}

/// I2C bus trait
pub trait I2c {
    /// Error type
    type Error;

    /// Configure the I2C bus
    fn configure(&mut self, config: I2cConfig) -> Result<(), Self::Error>;

    /// Write data to a device
    fn write(&mut self, address: I2cAddress, data: &[u8]) -> Result<(), Self::Error>;

    /// Read data from a device
    fn read(&mut self, address: I2cAddress, buffer: &mut [u8]) -> Result<(), Self::Error>;

    /// Write then read (combined operation)
    fn write_read(
        &mut self,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Self::Error>;

    /// Execute a transaction with multiple operations
    fn transaction(
        &mut self,
        address: I2cAddress,
        operations: &mut [I2cOperation<'_>],
    ) -> Result<(), Self::Error>;

    /// Write a single byte to a register
    fn write_register(&mut self, address: I2cAddress, register: u8, value: u8) -> Result<(), Self::Error> {
        self.write(address, &[register, value])
    }

    /// Read a single byte from a register
    fn read_register(&mut self, address: I2cAddress, register: u8) -> Result<u8, Self::Error> {
        let mut buffer = [0u8];
        self.write_read(address, &[register], &mut buffer)?;
        Ok(buffer[0])
    }

    /// Write multiple bytes to a register
    fn write_registers(&mut self, address: I2cAddress, register: u8, data: &[u8]) -> Result<(), Self::Error> {
        // Would need to combine register and data
        let mut buf = alloc::vec![register];
        buf.extend_from_slice(data);
        self.write(address, &buf)
    }

    /// Read multiple bytes from a register
    fn read_registers(&mut self, address: I2cAddress, register: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.write_read(address, &[register], buffer)
    }
}

/// I2C bus with scanning capability
pub trait I2cScanner: I2c {
    /// Scan for devices on the bus
    fn scan(&mut self) -> alloc::vec::Vec<I2cAddress>;

    /// Check if a device is present at the given address
    fn device_present(&mut self, address: I2cAddress) -> bool;
}

/// I2C controller managing multiple buses
pub trait I2cController {
    /// Error type
    type Error;
    /// Bus type
    type Bus: I2c;

    /// Get an I2C bus
    fn bus(&mut self, bus_number: u8) -> Result<Self::Bus, Self::Error>;

    /// Get the number of available buses
    fn bus_count(&self) -> u8;
}

/// Async I2C trait
#[cfg(feature = "async")]
pub trait AsyncI2c {
    /// Error type
    type Error;

    /// Write data asynchronously
    async fn write(&mut self, address: I2cAddress, data: &[u8]) -> Result<(), Self::Error>;

    /// Read data asynchronously
    async fn read(&mut self, address: I2cAddress, buffer: &mut [u8]) -> Result<(), Self::Error>;

    /// Write then read asynchronously
    async fn write_read(
        &mut self,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Self::Error>;
}

use alloc::vec;
