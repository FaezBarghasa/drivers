//! Board and SoC definitions for all supported platforms
//!
//! This module contains comprehensive board information for ESP32, STM32, Teensy,
//! Raspberry Pi, Radxa, ODROID, Pine64, Orange Pi, NanoPi, and Banana Pi.

use crate::{Architecture, BoardInfo, PeripheralConfig};

// ============================================================
// ESP32 Family
// ============================================================

/// ESP32 (original, Xtensa dual-core)
#[cfg(feature = "esp32")]
pub const ESP32: BoardInfo = BoardInfo {
    name: "ESP32",
    variant: "ESP32-WROOM-32",
    arch: Architecture::Xtensa,
    cpu_freq: 240_000_000,
    ram_size: 520 * 1024,       // 520KB SRAM
    flash_size: 4 * 1024 * 1024, // 4MB typically
    peripherals: PeripheralConfig {
        gpio_base: 0x3FF4_4000,
        spi_base: 0x3FF4_2000,
        i2c_base: 0x3FF5_3000,
        uart_base: 0x3FF4_0000,
        timer_base: 0x3FF5_F000,
        intc_base: 0x3FF0_0000,
    },
};

/// ESP32-S3 (Xtensa dual-core, AI acceleration)
#[cfg(feature = "esp32-s3")]
pub const ESP32_S3: BoardInfo = BoardInfo {
    name: "ESP32-S3",
    variant: "ESP32-S3-WROOM-1",
    arch: Architecture::Xtensa,
    cpu_freq: 240_000_000,
    ram_size: 512 * 1024,
    flash_size: 8 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0x6000_4000,
        spi_base: 0x6000_3000,
        i2c_base: 0x6001_E000,
        uart_base: 0x6000_0000,
        timer_base: 0x6001_F000,
        intc_base: 0x600C_2000,
    },
};

/// ESP32-C3 (RISC-V single-core)
#[cfg(feature = "esp32-c3")]
pub const ESP32_C3: BoardInfo = BoardInfo {
    name: "ESP32-C3",
    variant: "ESP32-C3-MINI-1",
    arch: Architecture::RISCV32,
    cpu_freq: 160_000_000,
    ram_size: 400 * 1024,
    flash_size: 4 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0x6000_4000,
        spi_base: 0x6000_2000,
        i2c_base: 0x6001_E000,
        uart_base: 0x6000_0000,
        timer_base: 0x6001_F000,
        intc_base: 0x600C_2000,
    },
};

// ============================================================
// STM32 Family
// ============================================================

/// STM32F4 Discovery board
#[cfg(feature = "stm32f4")]
pub const STM32F407_DISCOVERY: BoardInfo = BoardInfo {
    name: "STM32F407 Discovery",
    variant: "STM32F407VG",
    arch: Architecture::ARMv7M,
    cpu_freq: 168_000_000,
    ram_size: 192 * 1024,      // 192KB
    flash_size: 1024 * 1024,   // 1MB
    peripherals: PeripheralConfig {
        gpio_base: 0x4002_0000,
        spi_base: 0x4001_3000,
        i2c_base: 0x4000_5400,
        uart_base: 0x4001_1000,
        timer_base: 0x4000_0000,
        intc_base: 0xE000_E000,
    },
};

/// STM32H7 Nucleo board
#[cfg(feature = "stm32h7")]
pub const STM32H743_NUCLEO: BoardInfo = BoardInfo {
    name: "NUCLEO-H743ZI",
    variant: "STM32H743ZI",
    arch: Architecture::ARMv7M,
    cpu_freq: 480_000_000,
    ram_size: 1024 * 1024,     // 1MB
    flash_size: 2 * 1024 * 1024, // 2MB
    peripherals: PeripheralConfig {
        gpio_base: 0x5802_0000,
        spi_base: 0x4001_3000,
        i2c_base: 0x4000_5400,
        uart_base: 0x4001_1000,
        timer_base: 0x4000_0000,
        intc_base: 0xE000_E000,
    },
};

/// STM32 Blue Pill (STM32F103C8)
#[cfg(feature = "stm32f1")]
pub const STM32_BLUE_PILL: BoardInfo = BoardInfo {
    name: "STM32 Blue Pill",
    variant: "STM32F103C8T6",
    arch: Architecture::ARMv7M,
    cpu_freq: 72_000_000,
    ram_size: 20 * 1024,       // 20KB
    flash_size: 64 * 1024,     // 64KB
    peripherals: PeripheralConfig {
        gpio_base: 0x4001_0800,
        spi_base: 0x4001_3000,
        i2c_base: 0x4000_5400,
        uart_base: 0x4001_3800,
        timer_base: 0x4000_0000,
        intc_base: 0xE000_E000,
    },
};

// ============================================================
// Teensy Family  
// ============================================================

/// Teensy 4.0
#[cfg(feature = "teensy-40")]
pub const TEENSY_40: BoardInfo = BoardInfo {
    name: "Teensy 4.0",
    variant: "IMXRT1062",
    arch: Architecture::ARMv7M,
    cpu_freq: 600_000_000,
    ram_size: 1024 * 1024,     // 1MB
    flash_size: 2 * 1024 * 1024, // 2MB
    peripherals: PeripheralConfig {
        gpio_base: 0x401B_8000,
        spi_base: 0x4039_4000,
        i2c_base: 0x403F_0000,
        uart_base: 0x4018_4000,
        timer_base: 0x401D_C000,
        intc_base: 0xE000_E000,
    },
};

/// Teensy 4.1
#[cfg(feature = "teensy-41")]
pub const TEENSY_41: BoardInfo = BoardInfo {
    name: "Teensy 4.1",
    variant: "IMXRT1062",
    arch: Architecture::ARMv7M,
    cpu_freq: 600_000_000,
    ram_size: 1024 * 1024,
    flash_size: 8 * 1024 * 1024, // 8MB
    peripherals: PeripheralConfig {
        gpio_base: 0x401B_8000,
        spi_base: 0x4039_4000,
        i2c_base: 0x403F_0000,
        uart_base: 0x4018_4000,
        timer_base: 0x401D_C000,
        intc_base: 0xE000_E000,
    },
};

/// Teensy 3.6
#[cfg(feature = "teensy-36")]
pub const TEENSY_36: BoardInfo = BoardInfo {
    name: "Teensy 3.6",
    variant: "MK66FX1M0",
    arch: Architecture::ARMv7M,
    cpu_freq: 180_000_000,
    ram_size: 256 * 1024,
    flash_size: 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0x400F_F000,
        spi_base: 0x4002_C000,
        i2c_base: 0x4006_6000,
        uart_base: 0x4006_A000,
        timer_base: 0x4003_7000,
        intc_base: 0xE000_E000,
    },
};

// ============================================================
// Raspberry Pi Family
// ============================================================

/// Raspberry Pi Zero W
#[cfg(feature = "rpi-zero-w")]
pub const RPI_ZERO_W: BoardInfo = BoardInfo {
    name: "Raspberry Pi Zero W",
    variant: "BCM2835",
    arch: Architecture::ARMv6,
    cpu_freq: 1_000_000_000,
    ram_size: 512 * 1024 * 1024,
    flash_size: 0, // SD card
    peripherals: PeripheralConfig {
        gpio_base: 0x2020_0000,
        spi_base: 0x2020_4000,
        i2c_base: 0x2020_5000,
        uart_base: 0x2020_1000,
        timer_base: 0x2000_3000,
        intc_base: 0x2000_B200,
    },
};

/// Raspberry Pi 4
#[cfg(feature = "rpi-4")]
pub const RPI_4: BoardInfo = BoardInfo {
    name: "Raspberry Pi 4",
    variant: "BCM2711",
    arch: Architecture::ARMv8,
    cpu_freq: 1_500_000_000,
    ram_size: 4 * 1024 * 1024 * 1024, // 4GB model
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0xFE20_0000,
        spi_base: 0xFE20_4000,
        i2c_base: 0xFE80_4000,
        uart_base: 0xFE20_1000,
        timer_base: 0xFE00_3000,
        intc_base: 0xFF84_1000,
    },
};

/// Raspberry Pi 5
#[cfg(feature = "rpi-5")]
pub const RPI_5: BoardInfo = BoardInfo {
    name: "Raspberry Pi 5",
    variant: "BCM2712",
    arch: Architecture::ARMv8,
    cpu_freq: 2_400_000_000,
    ram_size: 8 * 1024 * 1024 * 1024, // 8GB model
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0x1F00_0D00_0000,
        spi_base: 0x1F00_0D00_4000,
        i2c_base: 0x1F00_0D00_5000,
        uart_base: 0x1F00_0D00_1000,
        timer_base: 0x1F00_0D00_3000,
        intc_base: 0x1F00_0000_0000,
    },
};

/// Raspberry Pi Pico
#[cfg(feature = "rpi-pico")]
pub const RPI_PICO: BoardInfo = BoardInfo {
    name: "Raspberry Pi Pico",
    variant: "RP2040",
    arch: Architecture::ARMv7M,
    cpu_freq: 133_000_000,
    ram_size: 264 * 1024,
    flash_size: 2 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0x4001_4000,
        spi_base: 0x4003_C000,
        i2c_base: 0x4004_4000,
        uart_base: 0x4003_4000,
        timer_base: 0x4005_4000,
        intc_base: 0xE000_E000,
    },
};

// ============================================================
// Radxa Boards
// ============================================================

/// Radxa Rock 5B
#[cfg(feature = "radxa-rock-5b")]
pub const RADXA_ROCK_5B: BoardInfo = BoardInfo {
    name: "Radxa Rock 5B",
    variant: "RK3588",
    arch: Architecture::ARMv8,
    cpu_freq: 2_400_000_000,
    ram_size: 16 * 1024 * 1024 * 1024, // 16GB model
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0xFD8A_0000,
        spi_base: 0xFEB2_0000,
        i2c_base: 0xFEA9_0000,
        uart_base: 0xFEB5_0000,
        timer_base: 0xFEAE_0000,
        intc_base: 0xFE60_0000,
    },
};

/// Radxa Rock 4
#[cfg(feature = "radxa-rock-4")]
pub const RADXA_ROCK_4: BoardInfo = BoardInfo {
    name: "Radxa Rock 4",
    variant: "RK3399",
    arch: Architecture::ARMv8,
    cpu_freq: 1_800_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0xFF72_0000,
        spi_base: 0xFF1C_0000,
        i2c_base: 0xFF3C_0000,
        uart_base: 0xFF18_0000,
        timer_base: 0xFF85_0000,
        intc_base: 0xFEE0_0000,
    },
};

// ============================================================
// ODROID Boards
// ============================================================

/// ODROID-N2+
#[cfg(feature = "odroid-n2")]
pub const ODROID_N2: BoardInfo = BoardInfo {
    name: "ODROID-N2+",
    variant: "S922X",
    arch: Architecture::ARMv8,
    cpu_freq: 2_400_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0xFF63_4000,
        spi_base: 0xFFF D1_0000,
        i2c_base: 0xFFD1_E000,
        uart_base: 0xFF80_3000,
        timer_base: 0xFFD0_0000,
        intc_base: 0xFFC0_1000,
    },
};

/// ODROID-M1
#[cfg(feature = "odroid-m1")]
pub const ODROID_M1: BoardInfo = BoardInfo {
    name: "ODROID-M1",
    variant: "RK3568",
    arch: Architecture::ARMv8,
    cpu_freq: 2_000_000_000,
    ram_size: 8 * 1024 * 1024 * 1024,
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0xFDD6_0000,
        spi_base: 0xFE61_0000,
        i2c_base: 0xFE5A_0000,
        uart_base: 0xFE66_0000,
        timer_base: 0xFE5F_0000,
        intc_base: 0xFD40_0000,
    },
};

// ============================================================
// Pine64 Boards
// ============================================================

/// PinePhone Pro
#[cfg(feature = "pinephone-pro")]
pub const PINEPHONE_PRO: BoardInfo = BoardInfo {
    name: "PinePhone Pro",
    variant: "RK3399S",
    arch: Architecture::ARMv8,
    cpu_freq: 1_800_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 128 * 1024 * 1024 * 1024, // 128GB eMMC option
    peripherals: PeripheralConfig {
        gpio_base: 0xFF72_0000,
        spi_base: 0xFF1C_0000,
        i2c_base: 0xFF3C_0000,
        uart_base: 0xFF18_0000,
        timer_base: 0xFF85_0000,
        intc_base: 0xFEE0_0000,
    },
};

/// Pinebook Pro
#[cfg(feature = "pinebook-pro")]
pub const PINEBOOK_PRO: BoardInfo = BoardInfo {
    name: "Pinebook Pro",
    variant: "RK3399",
    arch: Architecture::ARMv8,
    cpu_freq: 1_800_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 64 * 1024 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0xFF72_0000,
        spi_base: 0xFF1C_0000,
        i2c_base: 0xFF3C_0000,
        uart_base: 0xFF18_0000,
        timer_base: 0xFF85_0000,
        intc_base: 0xFEE0_0000,
    },
};

/// Star64 (RISC-V)
#[cfg(feature = "star64")]
pub const STAR64: BoardInfo = BoardInfo {
    name: "Pine64 Star64",
    variant: "JH7110",
    arch: Architecture::RISCV64,
    cpu_freq: 1_500_000_000,
    ram_size: 8 * 1024 * 1024 * 1024,
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0x1304_0000,
        spi_base: 0x1007_0000,
        i2c_base: 0x1003_0000,
        uart_base: 0x1000_0000,
        timer_base: 0x1305_0000,
        intc_base: 0x0C00_0000,
    },
};

// ============================================================
// Orange Pi Boards
// ============================================================

/// Orange Pi 5
#[cfg(feature = "orange-pi-5")]
pub const ORANGE_PI_5: BoardInfo = BoardInfo {
    name: "Orange Pi 5",
    variant: "RK3588S",
    arch: Architecture::ARMv8,
    cpu_freq: 2_400_000_000,
    ram_size: 8 * 1024 * 1024 * 1024,
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0xFD8A_0000,
        spi_base: 0xFEB2_0000,
        i2c_base: 0xFEA9_0000,
        uart_base: 0xFEB5_0000,
        timer_base: 0xFEAE_0000,
        intc_base: 0xFE60_0000,
    },
};

/// Orange Pi Zero 2
#[cfg(feature = "orange-pi-zero2")]
pub const ORANGE_PI_ZERO2: BoardInfo = BoardInfo {
    name: "Orange Pi Zero 2",
    variant: "H616",
    arch: Architecture::ARMv8,
    cpu_freq: 1_500_000_000,
    ram_size: 1024 * 1024 * 1024,
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0x0300_B000,
        spi_base: 0x0502_6000,
        i2c_base: 0x0502_2000,
        uart_base: 0x0500_0000,
        timer_base: 0x0302_0C00,
        intc_base: 0x0302_1000,
    },
};

// ============================================================
// NanoPi Boards
// ============================================================

/// NanoPi R5S
#[cfg(feature = "nanopi-r5s")]
pub const NANOPI_R5S: BoardInfo = BoardInfo {
    name: "NanoPi R5S",
    variant: "RK3568",
    arch: Architecture::ARMv8,
    cpu_freq: 2_000_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 32 * 1024 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0xFDD6_0000,
        spi_base: 0xFE61_0000,
        i2c_base: 0xFE5A_0000,
        uart_base: 0xFE66_0000,
        timer_base: 0xFE5F_0000,
        intc_base: 0xFD40_0000,
    },
};

/// NanoPi R6S
#[cfg(feature = "nanopi-r6s")]
pub const NANOPI_R6S: BoardInfo = BoardInfo {
    name: "NanoPi R6S",
    variant: "RK3588S",
    arch: Architecture::ARMv8,
    cpu_freq: 2_400_000_000,
    ram_size: 8 * 1024 * 1024 * 1024,
    flash_size: 32 * 1024 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0xFD8A_0000,
        spi_base: 0xFEB2_0000,
        i2c_base: 0xFEA9_0000,
        uart_base: 0xFEB5_0000,
        timer_base: 0xFEAE_0000,
        intc_base: 0xFE60_0000,
    },
};

/// NanoPi R4S
#[cfg(feature = "nanopi-r4s")]
pub const NANOPI_R4S: BoardInfo = BoardInfo {
    name: "NanoPi R4S",
    variant: "RK3399",
    arch: Architecture::ARMv8,
    cpu_freq: 1_800_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 0,
    peripherals: PeripheralConfig {
        gpio_base: 0xFF72_0000,
        spi_base: 0xFF1C_0000,
        i2c_base: 0xFF3C_0000,
        uart_base: 0xFF18_0000,
        timer_base: 0xFF85_0000,
        intc_base: 0xFEE0_0000,
    },
};

// ============================================================
// Banana Pi Boards
// ============================================================

/// Banana Pi M5
#[cfg(feature = "banana-pi-m5")]
pub const BANANA_PI_M5: BoardInfo = BoardInfo {
    name: "Banana Pi M5",
    variant: "S905X3",
    arch: Architecture::ARMv8,
    cpu_freq: 2_000_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 16 * 1024 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0xFF63_4000,
        spi_base: 0xFFD1_3000,
        i2c_base: 0xFFD1_D000,
        uart_base: 0xFF80_3000,
        timer_base: 0xFFD0_0000,
        intc_base: 0xFFC0_1000,
    },
};

/// Banana Pi R3 (Router, MT7986)
#[cfg(feature = "banana-pi-r3")]
pub const BANANA_PI_R3: BoardInfo = BoardInfo {
    name: "Banana Pi R3",
    variant: "MT7986A",
    arch: Architecture::ARMv8,
    cpu_freq: 2_000_000_000,
    ram_size: 2 * 1024 * 1024 * 1024,
    flash_size: 8 * 1024 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0x1000_5000,
        spi_base: 0x1100_D000,
        i2c_base: 0x1100_7000,
        uart_base: 0x1100_2000,
        timer_base: 0x1000_8000,
        intc_base: 0x0C00_0000,
    },
};

/// Banana Pi R4 (Router, MT7988)
#[cfg(feature = "banana-pi-r4")]
pub const BANANA_PI_R4: BoardInfo = BoardInfo {
    name: "Banana Pi R4",
    variant: "MT7988A",
    arch: Architecture::ARMv8,
    cpu_freq: 1_800_000_000,
    ram_size: 4 * 1024 * 1024 * 1024,
    flash_size: 8 * 1024 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0x1000_5000,
        spi_base: 0x1100_D000,
        i2c_base: 0x1100_7000,
        uart_base: 0x1100_2000,
        timer_base: 0x1000_8000,
        intc_base: 0x0C00_0000,
    },
};

/// Banana Pi M7
#[cfg(feature = "banana-pi-m7")]
pub const BANANA_PI_M7: BoardInfo = BoardInfo {
    name: "Banana Pi M7",
    variant: "RK3588",
    arch: Architecture::ARMv8,
    cpu_freq: 2_400_000_000,
    ram_size: 32 * 1024 * 1024 * 1024, // 32GB model
    flash_size: 64 * 1024 * 1024 * 1024,
    peripherals: PeripheralConfig {
        gpio_base: 0xFD8A_0000,
        spi_base: 0xFEB2_0000,
        i2c_base: 0xFEA9_0000,
        uart_base: 0xFEB5_0000,
        timer_base: 0xFEAE_0000,
        intc_base: 0xFE60_0000,
    },
};
