//! UART (Universal Asynchronous Receiver/Transmitter) HAL traits
//!
//! This module defines the UART abstraction for serial communication.

use crate::error::Result;

/// Baud rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaudRate {
    Baud9600,
    Baud19200,
    Baud38400,
    Baud57600,
    Baud115200,
    Baud230400,
    Baud460800,
    Baud921600,
    Custom(u32),
}

impl BaudRate {
    /// Get the baud rate value
    pub fn value(&self) -> u32 {
        match self {
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115200,
            BaudRate::Baud230400 => 230400,
            BaudRate::Baud460800 => 460800,
            BaudRate::Baud921600 => 921600,
            BaudRate::Custom(rate) => *rate,
        }
    }
}

impl From<u32> for BaudRate {
    fn from(rate: u32) -> Self {
        match rate {
            9600 => BaudRate::Baud9600,
            19200 => BaudRate::Baud19200,
            38400 => BaudRate::Baud38400,
            57600 => BaudRate::Baud57600,
            115200 => BaudRate::Baud115200,
            230400 => BaudRate::Baud230400,
            460800 => BaudRate::Baud460800,
            921600 => BaudRate::Baud921600,
            _ => BaudRate::Custom(rate),
        }
    }
}

/// Data bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

impl DataBits {
    /// Get the number of data bits
    pub fn bits(&self) -> u8 {
        match self {
            DataBits::Five => 5,
            DataBits::Six => 6,
            DataBits::Seven => 7,
            DataBits::Eight => 8,
            DataBits::Nine => 9,
        }
    }
}

/// Parity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    /// No parity
    None,
    /// Even parity
    Even,
    /// Odd parity
    Odd,
    /// Mark parity (always 1)
    Mark,
    /// Space parity (always 0)
    Space,
}

/// Stop bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    /// 1 stop bit
    One,
    /// 1.5 stop bits
    OnePointFive,
    /// 2 stop bits
    Two,
}

/// Flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    /// No flow control
    None,
    /// Hardware flow control (RTS/CTS)
    Hardware,
    /// Software flow control (XON/XOFF)
    Software,
}

/// UART configuration
#[derive(Debug, Clone, Copy)]
pub struct UartConfig {
    /// Baud rate
    pub baud_rate: BaudRate,
    /// Data bits
    pub data_bits: DataBits,
    /// Parity
    pub parity: Parity,
    /// Stop bits
    pub stop_bits: StopBits,
    /// Flow control
    pub flow_control: FlowControl,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            baud_rate: BaudRate::Baud115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        }
    }
}

impl UartConfig {
    /// Create a simple 8N1 configuration at the given baud rate
    pub fn new_8n1(baud_rate: BaudRate) -> Self {
        Self {
            baud_rate,
            ..Default::default()
        }
    }
}

/// UART trait
pub trait Uart {
    /// Error type
    type Error;

    /// Configure the UART
    fn configure(&mut self, config: UartConfig) -> Result<(), Self::Error>;

    /// Write bytes
    fn write(&mut self, data: &[u8]) -> Result<usize, Self::Error>;

    /// Read bytes
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Write a single byte
    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.write(&[byte]).map(|_| ())
    }

    /// Read a single byte
    fn read_byte(&mut self) -> Result<u8, Self::Error>;

    /// Flush the transmit buffer
    fn flush(&mut self) -> Result<(), Self::Error>;

    /// Check if data is available to read
    fn is_rx_ready(&self) -> bool;

    /// Check if transmitter is ready
    fn is_tx_ready(&self) -> bool;

    /// Get the number of bytes available in RX buffer
    fn rx_available(&self) -> usize;

    /// Get the number of bytes free in TX buffer
    fn tx_free(&self) -> usize;
}

/// UART with interrupt support
pub trait UartInterrupt: Uart {
    /// Enable RX interrupt
    fn enable_rx_interrupt(&mut self);

    /// Disable RX interrupt
    fn disable_rx_interrupt(&mut self);

    /// Enable TX ready interrupt
    fn enable_tx_interrupt(&mut self);

    /// Disable TX ready interrupt
    fn disable_tx_interrupt(&mut self);

    /// Set RX interrupt handler
    fn set_rx_handler(&mut self, handler: fn(u8));

    /// Set TX ready interrupt handler
    fn set_tx_handler(&mut self, handler: fn());
}

/// Blocking UART read
pub trait UartBlocking: Uart {
    /// Read bytes, blocking until all bytes are received
    fn read_blocking(&mut self, buffer: &mut [u8]) -> Result<(), Self::Error>;

    /// Write bytes, blocking until all bytes are sent
    fn write_blocking(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Read a line (until newline or buffer full)
    fn read_line(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
}

/// UART controller managing multiple ports
pub trait UartController {
    /// Error type
    type Error;
    /// UART type
    type Uart: Uart;

    /// Get a UART port
    fn uart(&mut self, port_number: u8) -> Result<Self::Uart, Self::Error>;

    /// Get the number of available ports
    fn port_count(&self) -> u8;
}

/// Async UART trait
#[cfg(feature = "async")]
pub trait AsyncUart {
    /// Error type
    type Error;

    /// Write bytes asynchronously
    async fn write(&mut self, data: &[u8]) -> Result<usize, Self::Error>;

    /// Read bytes asynchronously
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Flush asynchronously
    async fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Console/debug UART (always available at boot)
pub trait ConsoleUart: Uart {
    /// Get the console UART
    fn console() -> &'static mut Self;

    /// Write string (convenience for debug output)
    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        self.write(s.as_bytes()).map(|_| ())
    }

    /// Write line with newline
    fn writeln(&mut self, s: &str) -> Result<(), Self::Error> {
        self.write_str(s)?;
        self.write(&[b'\r', b'\n']).map(|_| ())
    }
}
