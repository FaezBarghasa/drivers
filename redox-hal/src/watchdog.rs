//! Watchdog timer HAL traits

use crate::error::Result;
use crate::time::Duration;

/// Watchdog timer trait
pub trait Watchdog {
    /// Error type
    type Error;

    /// Configure the watchdog timeout
    fn configure(&mut self, timeout: Duration) -> Result<(), Self::Error>;

    /// Start the watchdog
    fn start(&mut self) -> Result<(), Self::Error>;

    /// Stop the watchdog (if supported)
    fn stop(&mut self) -> Result<(), Self::Error>;

    /// Feed/kick the watchdog (reset countdown)
    fn feed(&mut self) -> Result<(), Self::Error>;

    /// Check if watchdog is running
    fn is_running(&self) -> bool;

    /// Get the configured timeout
    fn timeout(&self) -> Duration;

    /// Get remaining time before reset
    fn remaining(&self) -> Duration;

    /// Check if last reset was caused by watchdog
    fn caused_last_reset(&self) -> bool;
}

/// Independent watchdog (cannot be stopped)
pub trait IndependentWatchdog: Watchdog {
    /// Set the prescaler
    fn set_prescaler(&mut self, prescaler: u8) -> Result<(), Self::Error>;

    /// Set the reload value
    fn set_reload(&mut self, reload: u16) -> Result<(), Self::Error>;
}

/// Window watchdog (must be fed within a time window)
pub trait WindowWatchdog: Watchdog {
    /// Set the window (earliest time to feed)
    fn set_window(&mut self, window: Duration) -> Result<(), Self::Error>;

    /// Get the window duration
    fn window(&self) -> Duration;

    /// Check if currently in the valid window
    fn in_window(&self) -> bool;
}

/// Watchdog with early warning interrupt
pub trait WatchdogEarlyWarning: Watchdog {
    /// Enable early warning interrupt
    fn enable_early_warning(&mut self, before_timeout: Duration) -> Result<(), Self::Error>;

    /// Disable early warning interrupt
    fn disable_early_warning(&mut self);

    /// Set early warning handler
    fn set_warning_handler(&mut self, handler: fn());

    /// Check if early warning interrupt pending
    fn is_warning_pending(&self) -> bool;

    /// Clear early warning interrupt
    fn clear_warning(&mut self);
}
