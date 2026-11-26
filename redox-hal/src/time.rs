//! Time types for HAL

/// Duration type for timing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    /// Duration in nanoseconds
    nanos: u64,
}

impl Duration {
    /// Zero duration
    pub const ZERO: Duration = Duration { nanos: 0 };

    /// Maximum duration
    pub const MAX: Duration = Duration { nanos: u64::MAX };

    /// Create from nanoseconds
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Create from microseconds
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            nanos: micros * 1_000,
        }
    }

    /// Create from milliseconds
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            nanos: millis * 1_000_000,
        }
    }

    /// Create from seconds
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            nanos: secs * 1_000_000_000,
        }
    }

    /// Get as nanoseconds
    pub const fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Get as microseconds
    pub const fn as_micros(&self) -> u64 {
        self.nanos / 1_000
    }

    /// Get as milliseconds
    pub const fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    /// Get as seconds
    pub const fn as_secs(&self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Get subsecond nanoseconds
    pub const fn subsec_nanos(&self) -> u32 {
        (self.nanos % 1_000_000_000) as u32
    }

    /// Check if duration is zero
    pub const fn is_zero(&self) -> bool {
        self.nanos == 0
    }

    /// Saturating addition
    pub const fn saturating_add(self, other: Duration) -> Duration {
        Duration {
            nanos: self.nanos.saturating_add(other.nanos),
        }
    }

    /// Saturating subtraction
    pub const fn saturating_sub(self, other: Duration) -> Duration {
        Duration {
            nanos: self.nanos.saturating_sub(other.nanos),
        }
    }

    /// Multiply by a scalar
    pub const fn saturating_mul(self, scalar: u32) -> Duration {
        Duration {
            nanos: self.nanos.saturating_mul(scalar as u64),
        }
    }
}

impl core::ops::Add for Duration {
    type Output = Duration;

    fn add(self, other: Duration) -> Duration {
        Duration {
            nanos: self.nanos + other.nanos,
        }
    }
}

impl core::ops::Sub for Duration {
    type Output = Duration;

    fn sub(self, other: Duration) -> Duration {
        Duration {
            nanos: self.nanos - other.nanos,
        }
    }
}

impl core::ops::Mul<u32> for Duration {
    type Output = Duration;

    fn mul(self, scalar: u32) -> Duration {
        Duration {
            nanos: self.nanos * scalar as u64,
        }
    }
}

impl core::ops::Div<u32> for Duration {
    type Output = Duration;

    fn div(self, divisor: u32) -> Duration {
        Duration {
            nanos: self.nanos / divisor as u64,
        }
    }
}

/// Instant type for measuring elapsed time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    /// Timestamp in ticks
    ticks: u64,
}

impl Instant {
    /// Create from raw ticks
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    /// Get raw ticks
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Calculate duration since another instant
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        Duration::from_nanos(self.ticks.saturating_sub(earlier.ticks))
    }

    /// Calculate elapsed time since this instant
    pub fn elapsed(&self, now: Instant) -> Duration {
        now.duration_since(*self)
    }

    /// Check if deadline has passed
    pub fn has_passed(&self, now: Instant) -> bool {
        now.ticks >= self.ticks
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, duration: Duration) -> Instant {
        Instant {
            ticks: self.ticks + duration.as_nanos(),
        }
    }
}

impl core::ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, duration: Duration) -> Instant {
        Instant {
            ticks: self.ticks - duration.as_nanos(),
        }
    }
}

impl core::ops::Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, other: Instant) -> Duration {
        self.duration_since(other)
    }
}

/// Rate/frequency type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rate {
    /// Rate in Hz
    hz: u32,
}

impl Rate {
    /// Create from Hz
    pub const fn from_hz(hz: u32) -> Self {
        Self { hz }
    }

    /// Create from kHz
    pub const fn from_khz(khz: u32) -> Self {
        Self { hz: khz * 1_000 }
    }

    /// Create from MHz
    pub const fn from_mhz(mhz: u32) -> Self {
        Self {
            hz: mhz * 1_000_000,
        }
    }

    /// Get as Hz
    pub const fn as_hz(&self) -> u32 {
        self.hz
    }

    /// Get as kHz
    pub const fn as_khz(&self) -> u32 {
        self.hz / 1_000
    }

    /// Get as MHz
    pub const fn as_mhz(&self) -> u32 {
        self.hz / 1_000_000
    }

    /// Convert to period duration
    pub const fn period(&self) -> Duration {
        if self.hz == 0 {
            Duration::MAX
        } else {
            Duration::from_nanos(1_000_000_000 / self.hz as u64)
        }
    }
}
