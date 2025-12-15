//! Generic Board Support Package for Redox OS
//!
//! This crate provides BSP implementations for common embedded boards,
//! enabling Redox OS to run on various ARM and RISC-V platforms.
//!
//! # Supported Boards
//!
//! - **BeagleBone Black**: TI AM335x (ARMv7-A Cortex-A8)
//! - **Raspberry Pi Zero**: BCM2835 (ARMv6)
//! - **SiFive HiFive1**: FE310 (RISC-V RV32IMAC)
//!
//! # Minimal Embedded Profile
//!
//! This BSP is designed for the minimal Redox OS embedded profile:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Redox Embedded Profile                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
//! │  │  Networking │ │   Drivers   │ │  Filesys    │           │
//! │  │  TCP/IP     │ │   GPIO/SPI  │ │  RedoxFS    │           │
//! │  │  BBRv3      │ │   I2C/UART  │ │  FAT32      │           │
//! │  └─────────────┘ └─────────────┘ └─────────────┘           │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │                   Redox Kernel                       │   │
//! │  │  • Microkernel architecture                          │   │
//! │  │  • IPC-based drivers                                 │   │
//! │  │  • Memory isolation                                  │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │                   redox-hal                          │   │
//! │  │  Universal HAL traits (GPIO, SPI, I2C, UART, Timer) │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │              Board Support Package (BSP)             │   │
//! │  │  Hardware-specific implementations                   │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![no_std]

extern crate alloc;

pub mod board;
pub mod drivers;
pub mod net;
pub mod runtime;

// Re-export HAL traits
pub use redox_hal::prelude::*;

/// Board trait for common operations
pub trait Board {
    /// Initialize the board
    fn init() -> Self;

    /// Get board information
    fn info(&self) -> &'static BoardInfo;

    /// Get LED GPIO (if available)
    fn led(&mut self) -> Option<&mut dyn redox_hal::gpio::OutputPin<Error = redox_hal::Error>>;

    /// Get the console UART
    fn console(&mut self) -> &mut dyn redox_hal::uart::Uart<Error = redox_hal::Error>;

    /// Delay in milliseconds
    fn delay_ms(&self, ms: u32);
}

/// Board information
#[derive(Debug, Clone, Copy)]
pub struct BoardInfo {
    /// Board name
    pub name: &'static str,
    /// CPU description
    pub cpu: &'static str,
    /// RAM size in bytes
    pub ram_size: usize,
    /// Flash size in bytes
    pub flash_size: usize,
    /// CPU frequency in Hz
    pub cpu_freq: u32,
    /// Has Ethernet
    pub has_ethernet: bool,
    /// Has WiFi
    pub has_wifi: bool,
    /// Number of GPIO pins
    pub gpio_count: u8,
    /// Number of UART ports
    pub uart_count: u8,
    /// Number of SPI buses
    pub spi_count: u8,
    /// Number of I2C buses
    pub i2c_count: u8,
}

/// Networking profile type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkProfile {
    /// No networking
    None,
    /// Ethernet only
    Ethernet,
    /// WiFi only
    WiFi,
    /// Both Ethernet and WiFi
    Full,
}

/// Embedded system configuration
#[derive(Debug, Clone)]
pub struct EmbeddedConfig {
    /// Hostname
    pub hostname: &'static str,
    /// Network profile
    pub network_profile: NetworkProfile,
    /// Enable DHCP
    pub dhcp: bool,
    /// Static IP (if DHCP disabled)
    pub static_ip: Option<[u8; 4]>,
    /// Gateway
    pub gateway: Option<[u8; 4]>,
    /// DNS servers
    pub dns: Option<[u8; 4]>,
    /// Enable console on UART
    pub uart_console: bool,
    /// Console baud rate
    pub console_baud: u32,
    /// Enable watchdog
    pub watchdog: bool,
    /// Watchdog timeout in seconds
    pub watchdog_timeout: u32,
}

impl Default for EmbeddedConfig {
    fn default() -> Self {
        Self {
            hostname: "redox-embedded",
            network_profile: NetworkProfile::Ethernet,
            dhcp: true,
            static_ip: None,
            gateway: None,
            dns: None,
            uart_console: true,
            console_baud: 115200,
            watchdog: true,
            watchdog_timeout: 10,
        }
    }
}
