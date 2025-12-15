//! Error types for HAL operations

use core::fmt;

/// HAL result type
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// HAL error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Peripheral not available
    NotAvailable,
    /// Invalid configuration
    InvalidConfig,
    /// Invalid parameter
    InvalidParameter,
    /// Resource busy
    Busy,
    /// Operation timeout
    Timeout,
    /// Buffer overflow
    Overflow,
    /// Buffer underflow
    Underflow,
    /// Data too large
    DataTooLarge,
    /// No acknowledge received (I2C)
    NoAcknowledge,
    /// Arbitration lost (I2C)
    ArbitrationLost,
    /// Bus error
    BusError,
    /// CRC error
    CrcError,
    /// Framing error (UART)
    FramingError,
    /// Parity error (UART)
    ParityError,
    /// Overrun error
    OverrunError,
    /// DMA error
    DmaError,
    /// Permission denied
    PermissionDenied,
    /// Not initialized
    NotInitialized,
    /// Already initialized
    AlreadyInitialized,
    /// Hardware failure
    HardwareFailure,
    /// Other error
    Other,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotAvailable => write!(f, "Peripheral not available"),
            Error::InvalidConfig => write!(f, "Invalid configuration"),
            Error::InvalidParameter => write!(f, "Invalid parameter"),
            Error::Busy => write!(f, "Resource busy"),
            Error::Timeout => write!(f, "Operation timeout"),
            Error::Overflow => write!(f, "Buffer overflow"),
            Error::Underflow => write!(f, "Buffer underflow"),
            Error::DataTooLarge => write!(f, "Data too large"),
            Error::NoAcknowledge => write!(f, "No acknowledge received"),
            Error::ArbitrationLost => write!(f, "Arbitration lost"),
            Error::BusError => write!(f, "Bus error"),
            Error::CrcError => write!(f, "CRC error"),
            Error::FramingError => write!(f, "Framing error"),
            Error::ParityError => write!(f, "Parity error"),
            Error::OverrunError => write!(f, "Overrun error"),
            Error::DmaError => write!(f, "DMA error"),
            Error::PermissionDenied => write!(f, "Permission denied"),
            Error::NotInitialized => write!(f, "Not initialized"),
            Error::AlreadyInitialized => write!(f, "Already initialized"),
            Error::HardwareFailure => write!(f, "Hardware failure"),
            Error::Other => write!(f, "Other error"),
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Error {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "{}",
            match self {
                Error::NotAvailable => "NotAvailable",
                Error::InvalidConfig => "InvalidConfig",
                Error::InvalidParameter => "InvalidParameter",
                Error::Busy => "Busy",
                Error::Timeout => "Timeout",
                Error::Overflow => "Overflow",
                Error::Underflow => "Underflow",
                Error::DataTooLarge => "DataTooLarge",
                Error::NoAcknowledge => "NoAcknowledge",
                Error::ArbitrationLost => "ArbitrationLost",
                Error::BusError => "BusError",
                Error::CrcError => "CrcError",
                Error::FramingError => "FramingError",
                Error::ParityError => "ParityError",
                Error::OverrunError => "OverrunError",
                Error::DmaError => "DmaError",
                Error::PermissionDenied => "PermissionDenied",
                Error::NotInitialized => "NotInitialized",
                Error::AlreadyInitialized => "AlreadyInitialized",
                Error::HardwareFailure => "HardwareFailure",
                Error::Other => "Other",
            }
        );
    }
}
