//! RTC (Real-Time Clock) HAL traits

use crate::error::Result;

/// Date and time structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DateTime {
    /// Year (2000-2099)
    pub year: u16,
    /// Month (1-12)
    pub month: u8,
    /// Day of month (1-31)
    pub day: u8,
    /// Day of week (0=Sunday, 6=Saturday)
    pub weekday: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
}

impl DateTime {
    /// Create a new date time
    pub const fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            weekday: 0,
            hour,
            minute,
            second,
        }
    }

    /// Check if the date time is valid
    pub fn is_valid(&self) -> bool {
        self.year >= 2000
            && self.year <= 2099
            && self.month >= 1
            && self.month <= 12
            && self.day >= 1
            && self.day <= 31
            && self.hour <= 23
            && self.minute <= 59
            && self.second <= 59
    }

    /// Convert to Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
    pub fn to_unix_timestamp(&self) -> u64 {
        // Simplified calculation
        let days = days_since_epoch(self.year, self.month, self.day);
        let seconds =
            days * 86400 + self.hour as u64 * 3600 + self.minute as u64 * 60 + self.second as u64;
        seconds
    }

    /// Create from Unix timestamp
    pub fn from_unix_timestamp(timestamp: u64) -> Self {
        let days = timestamp / 86400;
        let remaining = timestamp % 86400;
        let hour = (remaining / 3600) as u8;
        let minute = ((remaining % 3600) / 60) as u8;
        let second = (remaining % 60) as u8;

        let (year, month, day) = date_from_days(days);

        Self {
            year,
            month,
            day,
            weekday: ((days + 4) % 7) as u8, // Jan 1, 1970 was Thursday (4)
            hour,
            minute,
            second,
        }
    }
}

/// Calculate days since Unix epoch
fn days_since_epoch(year: u16, month: u8, day: u8) -> u64 {
    let mut y = year as i32;
    let m = month as i32;

    // Adjust for months
    let a = (14 - m) / 12;
    y -= a;
    let m = m + 12 * a - 3;

    // Julian day number
    let jdn = day as i32 + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;

    // Unix epoch is Julian day 2440588
    (jdn - 2440588) as u64
}

/// Convert days since epoch to date
fn date_from_days(days: u64) -> (u16, u8, u8) {
    let jdn = days as i32 + 2440588;

    let a = jdn + 32044;
    let b = (4 * a + 3) / 1461;
    let c = a - (1461 * b / 4);
    let d = (4 * c + 3) / 1225;
    let e = c - (1225 * d / 4);
    let m = (5 * e + 2) / 153;

    let day = (e - (153 * m + 2) / 5 + 1) as u8;
    let month = (m + 3 - 12 * (m / 10)) as u8;
    let year = (b - 4716 + (m / 10)) as u16;

    (year, month, day)
}

/// Alarm match configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmMatch {
    /// Match seconds only
    Seconds,
    /// Match minutes and seconds
    MinutesSeconds,
    /// Match hour, minutes, seconds
    HourMinutesSeconds,
    /// Match day, hour, minutes, seconds
    DayHourMinutesSeconds,
    /// Match date and time
    Full,
}

/// RTC alarm
#[derive(Debug, Clone, Copy)]
pub struct Alarm {
    /// Alarm time
    pub time: DateTime,
    /// What parts to match
    pub match_config: AlarmMatch,
}

/// RTC trait
pub trait Rtc {
    /// Error type
    type Error;

    /// Initialize the RTC
    fn init(&mut self) -> Result<(), Self::Error>;

    /// Set the current date and time
    fn set_datetime(&mut self, datetime: DateTime) -> Result<(), Self::Error>;

    /// Get the current date and time
    fn datetime(&self) -> Result<DateTime, Self::Error>;

    /// Set alarm
    fn set_alarm(&mut self, alarm: Alarm) -> Result<(), Self::Error>;

    /// Clear alarm
    fn clear_alarm(&mut self);

    /// Check if alarm is triggered
    fn is_alarm_triggered(&self) -> bool;

    /// Enable alarm interrupt
    fn enable_alarm_interrupt(&mut self);

    /// Disable alarm interrupt
    fn disable_alarm_interrupt(&mut self);

    /// Set alarm handler
    fn set_alarm_handler(&mut self, handler: fn());
}

/// RTC with backup registers
pub trait RtcBackup: Rtc {
    /// Number of backup registers
    fn backup_register_count(&self) -> u8;

    /// Read a backup register
    fn read_backup(&self, register: u8) -> Result<u32, Self::Error>;

    /// Write a backup register
    fn write_backup(&mut self, register: u8, value: u32) -> Result<(), Self::Error>;
}

/// RTC with calibration
pub trait RtcCalibration: Rtc {
    /// Set calibration value (in ppm)
    fn set_calibration(&mut self, ppm: i16) -> Result<(), Self::Error>;

    /// Get calibration value
    fn calibration(&self) -> i16;
}

/// RTC with wakeup timer
pub trait RtcWakeup: Rtc {
    /// Enable periodic wakeup
    fn enable_wakeup(&mut self, period_ms: u32) -> Result<(), Self::Error>;

    /// Disable wakeup
    fn disable_wakeup(&mut self);

    /// Set wakeup handler
    fn set_wakeup_handler(&mut self, handler: fn());
}
