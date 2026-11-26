//! Timer HAL traits
//!
//! This module defines hardware timer abstractions.

use crate::error::Result;
use crate::time::Duration;

/// Timer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    /// One-shot mode (fires once)
    OneShot,
    /// Periodic mode (fires repeatedly)
    Periodic,
    /// Free-running counter
    FreeRunning,
    /// Input capture mode
    InputCapture,
    /// Output compare mode
    OutputCompare,
}

/// Counter direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountDirection {
    /// Count up from 0
    Up,
    /// Count down from max
    Down,
    /// Count up then down
    UpDown,
}

/// Timer configuration
#[derive(Debug, Clone, Copy)]
pub struct TimerConfig {
    /// Timer mode
    pub mode: TimerMode,
    /// Count direction
    pub direction: CountDirection,
    /// Prescaler value
    pub prescaler: u32,
    /// Auto-reload value (period)
    pub period: u32,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            mode: TimerMode::Periodic,
            direction: CountDirection::Up,
            prescaler: 0,
            period: 0xFFFFFFFF,
        }
    }
}

/// Basic timer trait
pub trait Timer {
    /// Error type
    type Error;

    /// Configure the timer
    fn configure(&mut self, config: TimerConfig) -> Result<(), Self::Error>;

    /// Start the timer
    fn start(&mut self) -> Result<(), Self::Error>;

    /// Stop the timer
    fn stop(&mut self) -> Result<(), Self::Error>;

    /// Get the current counter value
    fn counter(&self) -> u32;

    /// Set the counter value
    fn set_counter(&mut self, value: u32);

    /// Reset the counter to zero
    fn reset(&mut self) {
        self.set_counter(0);
    }

    /// Check if timer is running
    fn is_running(&self) -> bool;

    /// Get the timer frequency in Hz
    fn frequency(&self) -> u32;

    /// Set the period
    fn set_period(&mut self, period: u32) -> Result<(), Self::Error>;

    /// Get the period
    fn period(&self) -> u32;
}

/// Timer with interrupt support
pub trait TimerInterrupt: Timer {
    /// Enable timer overflow/period interrupt
    fn enable_interrupt(&mut self);

    /// Disable timer interrupt
    fn disable_interrupt(&mut self);

    /// Check if interrupt is pending
    fn is_interrupt_pending(&self) -> bool;

    /// Clear interrupt pending flag
    fn clear_interrupt(&mut self);

    /// Set interrupt handler
    fn set_handler(&mut self, handler: fn());
}

/// Countdown timer
pub trait CountdownTimer: Timer {
    /// Start countdown from the given duration
    fn start_countdown(&mut self, duration: Duration) -> Result<(), Self::Error>;

    /// Check if countdown has elapsed
    fn has_elapsed(&self) -> bool;

    /// Wait for countdown to elapse (blocking)
    fn wait(&mut self) -> Result<(), Self::Error>;

    /// Get remaining time
    fn remaining(&self) -> Duration;
}

/// Delay provider using a timer
pub trait Delay {
    /// Delay for the specified duration
    fn delay(&mut self, duration: Duration);

    /// Delay for microseconds
    fn delay_us(&mut self, us: u32) {
        self.delay(Duration::from_micros(us as u64));
    }

    /// Delay for milliseconds
    fn delay_ms(&mut self, ms: u32) {
        self.delay(Duration::from_millis(ms as u64));
    }
}

/// High-resolution timestamp counter
pub trait Timestamp {
    /// Get current timestamp value
    fn timestamp(&self) -> u64;

    /// Get timestamp frequency in Hz
    fn timestamp_frequency(&self) -> u32;

    /// Convert timestamp to microseconds
    fn timestamp_to_us(&self, timestamp: u64) -> u64 {
        (timestamp * 1_000_000) / self.timestamp_frequency() as u64
    }

    /// Get elapsed time between two timestamps
    fn elapsed(&self, start: u64, end: u64) -> Duration {
        let ticks = end.wrapping_sub(start);
        let us = (ticks * 1_000_000) / self.timestamp_frequency() as u64;
        Duration::from_micros(us)
    }
}

/// System tick timer (for RTOS-like functionality)
pub trait SysTick {
    /// Error type
    type Error;

    /// Configure system tick with period in microseconds
    fn configure(&mut self, period_us: u32) -> Result<(), Self::Error>;

    /// Enable system tick
    fn enable(&mut self);

    /// Disable system tick
    fn disable(&mut self);

    /// Get current tick count
    fn ticks(&self) -> u64;

    /// Set tick handler
    fn set_handler(&mut self, handler: fn());
}

/// Capture/Compare unit
pub trait CaptureCompare: Timer {
    /// Number of capture/compare channels
    fn channel_count(&self) -> u8;

    /// Set compare value for a channel
    fn set_compare(&mut self, channel: u8, value: u32) -> Result<(), Self::Error>;

    /// Get compare value for a channel
    fn compare(&self, channel: u8) -> u32;

    /// Get captured value for a channel
    fn capture(&self, channel: u8) -> u32;

    /// Enable capture on a channel
    fn enable_capture(&mut self, channel: u8) -> Result<(), Self::Error>;

    /// Disable capture on a channel
    fn disable_capture(&mut self, channel: u8) -> Result<(), Self::Error>;

    /// Enable compare interrupt on a channel
    fn enable_compare_interrupt(&mut self, channel: u8);

    /// Disable compare interrupt on a channel
    fn disable_compare_interrupt(&mut self, channel: u8);
}

/// Timer controller managing multiple timers
pub trait TimerController {
    /// Error type
    type Error;
    /// Timer type
    type Timer: Timer;

    /// Get a timer
    fn timer(&mut self, timer_number: u8) -> Result<Self::Timer, Self::Error>;

    /// Get the number of available timers
    fn timer_count(&self) -> u8;
}
