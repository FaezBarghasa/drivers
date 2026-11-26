//! PWM (Pulse Width Modulation) HAL traits

use crate::error::Result;
use crate::time::Duration;

/// PWM polarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmPolarity {
    /// Active high (duty cycle = high time)
    ActiveHigh,
    /// Active low (duty cycle = low time)
    ActiveLow,
}

/// PWM alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmAlignment {
    /// Left-aligned (edge-aligned, rising edge at period start)
    Left,
    /// Right-aligned (edge-aligned, falling edge at period end)
    Right,
    /// Center-aligned (symmetric)
    Center,
}

/// PWM configuration
#[derive(Debug, Clone, Copy)]
pub struct PwmConfig {
    /// PWM frequency in Hz
    pub frequency: u32,
    /// Initial duty cycle (0-10000 for 0%-100.00%)
    pub duty_cycle: u16,
    /// Polarity
    pub polarity: PwmPolarity,
    /// Alignment
    pub alignment: PwmAlignment,
}

impl Default for PwmConfig {
    fn default() -> Self {
        Self {
            frequency: 1000,  // 1 kHz
            duty_cycle: 5000, // 50%
            polarity: PwmPolarity::ActiveHigh,
            alignment: PwmAlignment::Left,
        }
    }
}

/// PWM channel trait
pub trait Pwm {
    /// Error type
    type Error;

    /// Configure the PWM channel
    fn configure(&mut self, config: PwmConfig) -> Result<(), Self::Error>;

    /// Enable the PWM output
    fn enable(&mut self) -> Result<(), Self::Error>;

    /// Disable the PWM output
    fn disable(&mut self) -> Result<(), Self::Error>;

    /// Check if PWM is enabled
    fn is_enabled(&self) -> bool;

    /// Set the duty cycle (0-10000 for 0%-100.00%)
    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error>;

    /// Get the current duty cycle
    fn duty_cycle(&self) -> u16;

    /// Set the duty cycle as a percentage (0.0-100.0)
    fn set_duty_percent(&mut self, percent: f32) -> Result<(), Self::Error> {
        let duty = (percent * 100.0) as u16;
        self.set_duty_cycle(duty.min(10000))
    }

    /// Set the frequency in Hz
    fn set_frequency(&mut self, frequency: u32) -> Result<(), Self::Error>;

    /// Get the current frequency
    fn frequency(&self) -> u32;

    /// Set the period
    fn set_period(&mut self, period: Duration) -> Result<(), Self::Error> {
        let freq = 1_000_000_000 / period.as_nanos() as u32;
        self.set_frequency(freq)
    }

    /// Get the maximum duty cycle value
    fn max_duty_cycle(&self) -> u16 {
        10000
    }
}

/// PWM with complementary output
pub trait ComplementaryPwm: Pwm {
    /// Set dead time between complementary outputs
    fn set_dead_time(&mut self, dead_time: Duration) -> Result<(), Self::Error>;

    /// Enable complementary output
    fn enable_complementary(&mut self) -> Result<(), Self::Error>;

    /// Disable complementary output
    fn disable_complementary(&mut self) -> Result<(), Self::Error>;
}

/// PWM controller managing multiple channels
pub trait PwmController {
    /// Error type
    type Error;
    /// Channel type
    type Channel: Pwm;

    /// Get a PWM channel
    fn channel(&mut self, channel_number: u8) -> Result<Self::Channel, Self::Error>;

    /// Get the number of available channels
    fn channel_count(&self) -> u8;

    /// Set frequency for all channels
    fn set_global_frequency(&mut self, frequency: u32) -> Result<(), Self::Error>;
}
