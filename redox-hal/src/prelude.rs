//! Prelude module for convenient imports

pub use crate::error::{Error, Result};
pub use crate::time::{Duration, Instant, Rate};

#[cfg(feature = "gpio")]
pub use crate::gpio::{Edge, GpioPin, InputPin, Level, OutputPin, PinMode, Pull};

#[cfg(feature = "spi")]
pub use crate::spi::{SpiBus, SpiConfig, SpiMode};

#[cfg(feature = "i2c")]
pub use crate::i2c::{I2c, I2cAddress, I2cConfig, I2cSpeed};

#[cfg(feature = "uart")]
pub use crate::uart::{BaudRate, DataBits, Parity, StopBits, Uart, UartConfig};

#[cfg(feature = "timer")]
pub use crate::timer::{Delay, Timer, TimerConfig, TimerMode};

#[cfg(feature = "pwm")]
pub use crate::pwm::{Pwm, PwmConfig};

#[cfg(feature = "adc")]
pub use crate::adc::{Adc, AdcConfig};

#[cfg(feature = "dma")]
pub use crate::dma::{Dma, DmaChannel};

#[cfg(feature = "watchdog")]
pub use crate::watchdog::Watchdog;

#[cfg(feature = "rtc")]
pub use crate::rtc::Rtc;
