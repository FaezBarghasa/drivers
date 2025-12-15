//! Redox Hardware Abstraction Layer (HAL)
//!
//! This crate provides a universal, safe, Rust trait-based Hardware Abstraction Layer
//! for embedded peripherals in Redox OS. It enables portable driver development across
//! multiple architectures and boards.
//!
//! # Supported Platforms
//!
//! ## Microcontrollers
//! - **ESP32**: ESP32, ESP32-S2, ESP32-S3 (Xtensa), ESP32-C3/C6/H2 (RISC-V)
//! - **STM32**: Full STM32F/G/H/L/W series (ARM Cortex-M)
//! - **Teensy**: Teensy 3.x, 4.x, 4.1 (ARM Cortex-M)
//! - **RP2040/RP2350**: Raspberry Pi Pico (ARM Cortex-M)
//!
//! ## Single Board Computers
//! - **Raspberry Pi**: Zero, 1-5, CM4 (ARM)
//! - **Radxa**: Rock 3/4/5, Zero (Rockchip RK35xx)
//! - **ODROID**: C1-C4, N2, M1, XU4, H2/H3 (ARM/x86)
//! - **Pine64**: PinePhone, Pinebook Pro, Rock64, Star64 (ARM/RISC-V)
//! - **Orange Pi**: Zero/3/4/5, R1 series (Allwinner/Rockchip)
//! - **NanoPi**: NEO/M/R series (Allwinner/Rockchip)
//! - **Banana Pi**: M1-M7, R2-R4 series (Allwinner/MediaTek/Rockchip)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Application / Driver                          │
//! └──────────────────────────┬──────────────────────────────────────┘
//!                            │
//! ┌──────────────────────────▼──────────────────────────────────────┐
//! │                     redox-hal Traits                             │
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │
//! │  │  GPIO   │ │   SPI   │ │   I2C   │ │  UART   │ │  Timer  │    │
//! │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘    │
//! └───────┼───────────┼───────────┼───────────┼───────────┼─────────┘
//!         │           │           │           │           │
//! ┌───────┼───────────┼───────────┼───────────┼───────────┼─────────┐
//! │       ▼           ▼           ▼           ▼           ▼         │
//! │              Board Support Package (BSP)                         │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │  Hardware-specific implementations                      │    │
//! │  │  • ESP32 / STM32 / Teensy (Microcontrollers)           │    │
//! │  │  • Raspberry Pi / Radxa / ODROID (SBCs)                │    │
//! │  │  • Orange Pi / NanoPi / Banana Pi / Pine64             │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # HAL Traits
//!
//! The HAL defines the following core traits:
//!
//! - [`gpio::GpioPin`] - Digital input/output pins
//! - [`spi::SpiBus`] - SPI master interface
//! - [`i2c::I2c`] - I2C master interface
//! - [`uart::Uart`] - Serial UART interface
//! - [`timer::Timer`] - Hardware timers
//! - [`pwm::Pwm`] - PWM output
//! - [`adc::Adc`] - Analog to digital conversion
//! - [`dma::Dma`] - DMA transfer
//! - [`watchdog::Watchdog`] - Watchdog timer
//! - [`rtc::Rtc`] - Real-time clock
//!
//! # Usage
//!
//! ```ignore
//! use redox_hal::prelude::*;
//! use redox_hal::gpio::{GpioPin, PinMode, Level};
//!
//! fn blink<P: GpioPin>(pin: &mut P) -> Result<(), P::Error> {
//!     pin.set_mode(PinMode::Output)?;
//!     loop {
//!         pin.set_level(Level::High)?;
//!         delay_ms(500);
//!         pin.set_level(Level::Low)?;
//!         delay_ms(500);
//!     }
//! }
//! ```
//!
//! # Embedded Profile
//!
//! The HAL is designed to work with a minimal Redox OS embedded profile
//! suitable for IoT and networking devices:
//!
//! - Minimal kernel footprint (<512KB)
//! - No graphical interface
//! - Networking stack (TCP/IP, BBRv3)
//! - Secure boot support
//! - OTA update capability

#![no_std]
#![cfg_attr(feature = "async", feature(async_fn_in_trait))]

extern crate alloc;

// Core modules
pub mod error;
pub mod prelude;
pub mod time;

// Board definitions
pub mod boards;

// Peripheral modules
#[cfg(feature = "gpio")]
pub mod gpio;

#[cfg(feature = "spi")]
pub mod spi;

#[cfg(feature = "i2c")]
pub mod i2c;

#[cfg(feature = "uart")]
pub mod uart;

#[cfg(feature = "timer")]
pub mod timer;

#[cfg(feature = "pwm")]
pub mod pwm;

#[cfg(feature = "adc")]
pub mod adc;

#[cfg(feature = "dma")]
pub mod dma;

#[cfg(feature = "watchdog")]
pub mod watchdog;

#[cfg(feature = "rtc")]
pub mod rtc;

// Architecture-specific modules
#[cfg(any(feature = "armv6", feature = "armv7", feature = "armv8"))]
pub mod arch_armv7;

#[cfg(feature = "armv7m")]
pub mod arch_cortex_m;

#[cfg(any(feature = "riscv32", feature = "riscv64"))]
pub mod arch_riscv32;

#[cfg(feature = "xtensa")]
pub mod arch_xtensa;

#[cfg(feature = "aarch64")]
pub mod arch_aarch64;

// Re-exports
pub use error::{Error, Result};

/// HAL version
pub const HAL_VERSION: (u32, u32, u32) = (0, 1, 0);

/// Peripheral base address configuration
#[derive(Debug, Clone, Copy)]
pub struct PeripheralConfig {
    /// GPIO base address
    pub gpio_base: usize,
    /// SPI base address
    pub spi_base: usize,
    /// I2C base address
    pub i2c_base: usize,
    /// UART base address
    pub uart_base: usize,
    /// Timer base address
    pub timer_base: usize,
    /// Interrupt controller base address
    pub intc_base: usize,
}

/// Board information
#[derive(Debug, Clone)]
pub struct BoardInfo {
    /// Board name
    pub name: &'static str,
    /// Board variant
    pub variant: &'static str,
    /// CPU architecture
    pub arch: Architecture,
    /// CPU frequency in Hz
    pub cpu_freq: u32,
    /// RAM size in bytes
    pub ram_size: usize,
    /// Flash size in bytes
    pub flash_size: usize,
    /// Peripheral configuration
    pub peripherals: PeripheralConfig,
}

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// ARM v6 (Raspberry Pi Zero, Pi 1)
    ARMv6,
    /// ARM v7-A (BeagleBone, older Pi, Orange Pi)
    ARMv7,
    /// ARM v7-M (Cortex-M: STM32, Teensy, RP2040)
    ARMv7M,
    /// ARM v8 (32-bit mode)
    ARMv8,
    /// ARM v8 (64-bit: Pi 3/4/5, Rock5, NanoPi)
    AArch64,
    /// RISC-V 32-bit (ESP32-C3, SiFive HiFive1)
    RISCV32,
    /// RISC-V 64-bit (Star64, VisionFive)
    RISCV64,
    /// Xtensa (ESP32, ESP32-S2, ESP32-S3)
    Xtensa,
    /// x86_64 (ODROID-H2/H3)
    X86_64,
}

impl Architecture {
    /// Get the pointer width for this architecture
    pub const fn pointer_width(&self) -> u8 {
        match self {
            Architecture::ARMv6
            | Architecture::ARMv7
            | Architecture::ARMv7M
            | Architecture::ARMv8
            | Architecture::RISCV32
            | Architecture::Xtensa => 32,
            Architecture::AArch64 | Architecture::RISCV64 | Architecture::X86_64 => 64,
        }
    }

    /// Check if this is a 32-bit architecture
    pub const fn is_32bit(&self) -> bool {
        self.pointer_width() == 32
    }

    /// Check if this is an ARM architecture
    pub const fn is_arm(&self) -> bool {
        matches!(
            self,
            Architecture::ARMv6
                | Architecture::ARMv7
                | Architecture::ARMv7M
                | Architecture::ARMv8
                | Architecture::AArch64
        )
    }

    /// Check if this is a RISC-V architecture
    pub const fn is_riscv(&self) -> bool {
        matches!(self, Architecture::RISCV32 | Architecture::RISCV64)
    }

    /// Check if this is an Xtensa (ESP32) architecture
    pub const fn is_xtensa(&self) -> bool {
        matches!(self, Architecture::Xtensa)
    }

    /// Get architecture name
    pub const fn name(&self) -> &'static str {
        match self {
            Architecture::ARMv6 => "ARMv6",
            Architecture::ARMv7 => "ARMv7-A",
            Architecture::ARMv7M => "ARMv7-M",
            Architecture::ARMv8 => "ARMv8-A (32-bit)",
            Architecture::AArch64 => "AArch64",
            Architecture::RISCV32 => "RISC-V RV32",
            Architecture::RISCV64 => "RISC-V RV64",
            Architecture::Xtensa => "Xtensa LX6/LX7",
            Architecture::X86_64 => "x86_64",
        }
    }
}

/// Critical section guard
#[cfg(feature = "critical-section")]
pub mod critical_section {
    use core::cell::UnsafeCell;

    /// Critical section token proving interrupts are disabled
    #[derive(Debug)]
    pub struct CriticalSection {
        _private: (),
    }

    impl CriticalSection {
        /// Create a new critical section (unsafe - caller must ensure interrupts are disabled)
        ///
        /// # Safety
        ///
        /// Caller must ensure interrupts are disabled before calling this function.
        #[inline]
        pub unsafe fn new() -> Self {
            Self { _private: () }
        }
    }

    /// Execute code in a critical section with interrupts disabled
    #[inline]
    pub fn with<R>(f: impl FnOnce(&CriticalSection) -> R) -> R {
        // Architecture-specific interrupt disable
        #[cfg(feature = "armv7")]
        unsafe {
            let primask: u32;
            core::arch::asm!("mrs {}, primask", out(reg) primask);
            core::arch::asm!("cpsid i");
            let cs = CriticalSection::new();
            let result = f(&cs);
            if primask & 1 == 0 {
                core::arch::asm!("cpsie i");
            }
            result
        }

        #[cfg(feature = "riscv32")]
        unsafe {
            let mstatus: u32;
            core::arch::asm!("csrr {}, mstatus", out(reg) mstatus);
            core::arch::asm!("csrci mstatus, 8"); // Disable MIE
            let cs = CriticalSection::new();
            let result = f(&cs);
            if mstatus & 8 != 0 {
                core::arch::asm!("csrsi mstatus, 8"); // Enable MIE
            }
            result
        }

        #[cfg(not(any(feature = "armv7", feature = "riscv32")))]
        {
            // Fallback for other architectures
            let cs = unsafe { CriticalSection::new() };
            f(&cs)
        }
    }

    /// Mutex protected by critical section
    pub struct Mutex<T> {
        data: UnsafeCell<T>,
    }

    unsafe impl<T: Send> Sync for Mutex<T> {}
    unsafe impl<T: Send> Send for Mutex<T> {}

    impl<T> Mutex<T> {
        /// Create a new mutex
        pub const fn new(data: T) -> Self {
            Self {
                data: UnsafeCell::new(data),
            }
        }

        /// Access the data in a critical section
        pub fn lock<R>(&self, _cs: &CriticalSection, f: impl FnOnce(&mut T) -> R) -> R {
            f(unsafe { &mut *self.data.get() })
        }
    }
}
