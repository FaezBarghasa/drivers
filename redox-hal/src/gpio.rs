//! GPIO (General Purpose Input/Output) HAL traits
//!
//! This module defines the GPIO abstraction for digital I/O pins.

use crate::error::Result;

/// Pin mode configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMode {
    /// High-impedance input
    Input,
    /// Push-pull output
    Output,
    /// Open-drain output
    OpenDrain,
    /// Alternate function
    Alternate(u8),
    /// Analog mode (for ADC/DAC)
    Analog,
}

/// Pin pull configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pull {
    /// No pull-up or pull-down
    None,
    /// Pull-up resistor enabled
    Up,
    /// Pull-down resistor enabled
    Down,
}

/// Digital logic level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Logic low (0V)
    Low,
    /// Logic high (VCC)
    High,
}

impl Level {
    /// Convert from boolean
    pub fn from_bool(value: bool) -> Self {
        if value {
            Level::High
        } else {
            Level::Low
        }
    }

    /// Convert to boolean
    pub fn to_bool(self) -> bool {
        matches!(self, Level::High)
    }

    /// Toggle the level
    pub fn toggle(self) -> Self {
        match self {
            Level::Low => Level::High,
            Level::High => Level::Low,
        }
    }
}

impl From<bool> for Level {
    fn from(value: bool) -> Self {
        Level::from_bool(value)
    }
}

impl From<Level> for bool {
    fn from(level: Level) -> Self {
        level.to_bool()
    }
}

/// Edge detection for interrupts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Rising edge (low to high)
    Rising,
    /// Falling edge (high to low)
    Falling,
    /// Both edges
    Both,
}

/// Output drive strength
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveStrength {
    /// Lowest drive strength
    Low,
    /// Medium drive strength
    Medium,
    /// High drive strength
    High,
    /// Maximum drive strength
    Maximum,
}

/// Output slew rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlewRate {
    /// Slow edge transitions (for EMI reduction)
    Slow,
    /// Fast edge transitions
    Fast,
}

/// GPIO pin trait
pub trait GpioPin {
    /// Error type for GPIO operations
    type Error;

    /// Get the pin number
    fn pin_number(&self) -> u8;

    /// Set pin mode
    fn set_mode(&mut self, mode: PinMode) -> Result<(), Self::Error>;

    /// Get current pin mode
    fn mode(&self) -> PinMode;

    /// Set pull configuration
    fn set_pull(&mut self, pull: Pull) -> Result<(), Self::Error>;

    /// Read the current logic level
    fn read(&self) -> Result<Level, Self::Error>;

    /// Write a logic level (for output pins)
    fn write(&mut self, level: Level) -> Result<(), Self::Error>;

    /// Toggle the output state
    fn toggle(&mut self) -> Result<(), Self::Error> {
        let current = self.read()?;
        self.write(current.toggle())
    }

    /// Check if pin is high
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.read()? == Level::High)
    }

    /// Check if pin is low
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.read()? == Level::Low)
    }

    /// Set pin high
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.write(Level::High)
    }

    /// Set pin low
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.write(Level::Low)
    }
}

/// GPIO pin with interrupt support
pub trait InterruptPin: GpioPin {
    /// Enable interrupt on edge
    fn enable_interrupt(&mut self, edge: Edge) -> Result<(), Self::Error>;

    /// Disable interrupt
    fn disable_interrupt(&mut self) -> Result<(), Self::Error>;

    /// Check if interrupt is pending
    fn is_interrupt_pending(&self) -> bool;

    /// Clear interrupt pending flag
    fn clear_interrupt(&mut self);

    /// Set interrupt callback
    fn set_interrupt_handler(&mut self, handler: fn());
}

/// GPIO pin with configurable drive strength
pub trait ConfigurablePin: GpioPin {
    /// Set drive strength
    fn set_drive_strength(&mut self, strength: DriveStrength) -> Result<(), Self::Error>;

    /// Set slew rate
    fn set_slew_rate(&mut self, rate: SlewRate) -> Result<(), Self::Error>;
}

/// Input pin (type-state pattern)
pub trait InputPin {
    /// Error type
    type Error;

    /// Read the input level
    fn is_high(&self) -> Result<bool, Self::Error>;

    /// Read the input level
    fn is_low(&self) -> Result<bool, Self::Error>;
}

/// Output pin (type-state pattern)
pub trait OutputPin {
    /// Error type
    type Error;

    /// Set output high
    fn set_high(&mut self) -> Result<(), Self::Error>;

    /// Set output low
    fn set_low(&mut self) -> Result<(), Self::Error>;

    /// Set output level
    fn set_level(&mut self, level: Level) -> Result<(), Self::Error> {
        match level {
            Level::High => self.set_high(),
            Level::Low => self.set_low(),
        }
    }
}

/// Stateful output pin (can read back output state)
pub trait StatefulOutputPin: OutputPin {
    /// Check if output is set high
    fn is_set_high(&self) -> Result<bool, Self::Error>;

    /// Check if output is set low
    fn is_set_low(&self) -> Result<bool, Self::Error>;

    /// Toggle output state
    fn toggle(&mut self) -> Result<(), Self::Error>;
}

/// GPIO port (collection of pins)
pub trait GpioPort {
    /// Error type
    type Error;
    /// Pin type
    type Pin: GpioPin;

    /// Get a pin from this port
    fn pin(&mut self, number: u8) -> Result<Self::Pin, Self::Error>;

    /// Read all pins as a bitmask
    fn read_all(&self) -> Result<u32, Self::Error>;

    /// Write all pins from a bitmask
    fn write_all(&mut self, value: u32) -> Result<(), Self::Error>;

    /// Write only selected pins (using mask)
    fn write_masked(&mut self, value: u32, mask: u32) -> Result<(), Self::Error>;
}

/// GPIO controller for the entire chip
pub trait GpioController {
    /// Error type
    type Error;
    /// Port type
    type Port: GpioPort;

    /// Get a GPIO port
    fn port(&mut self, port_number: u8) -> Result<Self::Port, Self::Error>;

    /// Get total number of ports
    fn port_count(&self) -> u8;
}
