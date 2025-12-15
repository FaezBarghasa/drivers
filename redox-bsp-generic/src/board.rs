//! Board-specific implementations

use crate::BoardInfo;

/// BeagleBone Black board information
#[cfg(feature = "beaglebone-black")]
pub const BEAGLEBONE_BLACK: BoardInfo = BoardInfo {
    name: "BeagleBone Black",
    cpu: "TI AM335x (ARM Cortex-A8 @ 1GHz)",
    ram_size: 512 * 1024 * 1024,        // 512 MB
    flash_size: 4 * 1024 * 1024 * 1024, // 4 GB eMMC
    cpu_freq: 1_000_000_000,
    has_ethernet: true,
    has_wifi: false,
    gpio_count: 65,
    uart_count: 6,
    spi_count: 2,
    i2c_count: 3,
};

/// Raspberry Pi Zero board information
#[cfg(feature = "raspberry-pi-zero")]
pub const RASPBERRY_PI_ZERO: BoardInfo = BoardInfo {
    name: "Raspberry Pi Zero",
    cpu: "BCM2835 (ARM1176JZF-S @ 1GHz)",
    ram_size: 512 * 1024 * 1024, // 512 MB
    flash_size: 0,               // SD card
    cpu_freq: 1_000_000_000,
    has_ethernet: false,
    has_wifi: false, // Zero W has WiFi
    gpio_count: 40,
    uart_count: 1,
    spi_count: 2,
    i2c_count: 2,
};

/// SiFive HiFive1 board information
#[cfg(feature = "sifive-hifive1")]
pub const SIFIVE_HIFIVE1: BoardInfo = BoardInfo {
    name: "SiFive HiFive1 Rev B",
    cpu: "FE310-G002 (RV32IMAC @ 320MHz)",
    ram_size: 16 * 1024,         // 16 KB SRAM
    flash_size: 4 * 1024 * 1024, // 4 MB QSPI Flash
    cpu_freq: 320_000_000,
    has_ethernet: false,
    has_wifi: false,
    gpio_count: 19,
    uart_count: 2,
    spi_count: 3,
    i2c_count: 1,
};

/// Memory map for AM335x (BeagleBone Black)
#[cfg(feature = "am335x")]
pub mod am335x {
    /// L3 interconnect base
    pub const L3_BASE: usize = 0x4400_0000;
    /// L4 Wakeup base
    pub const L4_WKUP_BASE: usize = 0x4420_0000;
    /// L4 Peripheral base
    pub const L4_PER_BASE: usize = 0x4800_0000;
    /// L4 Fast base
    pub const L4_FAST_BASE: usize = 0x4A00_0000;

    /// GPIO0 base address
    pub const GPIO0_BASE: usize = 0x44E0_7000;
    /// GPIO1 base address
    pub const GPIO1_BASE: usize = 0x4804_C000;
    /// GPIO2 base address
    pub const GPIO2_BASE: usize = 0x481A_C000;
    /// GPIO3 base address
    pub const GPIO3_BASE: usize = 0x481A_E000;

    /// UART0 base address (console)
    pub const UART0_BASE: usize = 0x44E0_9000;
    /// UART1 base address
    pub const UART1_BASE: usize = 0x4802_2000;
    /// UART2 base address
    pub const UART2_BASE: usize = 0x4802_4000;

    /// SPI0 base address
    pub const SPI0_BASE: usize = 0x4803_0000;
    /// SPI1 base address
    pub const SPI1_BASE: usize = 0x481A_0000;

    /// I2C0 base address
    pub const I2C0_BASE: usize = 0x44E0_B000;
    /// I2C1 base address
    pub const I2C1_BASE: usize = 0x4802_A000;
    /// I2C2 base address
    pub const I2C2_BASE: usize = 0x4819_C000;

    /// Timer base addresses
    pub const DMTIMER0_BASE: usize = 0x44E0_5000;
    pub const DMTIMER1_BASE: usize = 0x44E3_1000;
    pub const DMTIMER2_BASE: usize = 0x4804_0000;

    /// Watchdog base
    pub const WDT1_BASE: usize = 0x44E3_5000;

    /// Ethernet (CPSW) base
    pub const CPSW_BASE: usize = 0x4A10_0000;
    /// MDIO base
    pub const MDIO_BASE: usize = 0x4A10_1000;

    /// Interrupt controller base
    pub const INTC_BASE: usize = 0x4820_0000;

    /// DDR base
    pub const DDR_BASE: usize = 0x8000_0000;
}

/// Memory map for BCM2835 (Raspberry Pi)
#[cfg(feature = "bcm2835")]
pub mod bcm2835 {
    /// Peripheral base (depends on Pi model)
    pub const PERIPHERAL_BASE: usize = 0x2000_0000;

    /// GPIO base
    pub const GPIO_BASE: usize = PERIPHERAL_BASE + 0x20_0000;
    /// UART0 base
    pub const UART0_BASE: usize = PERIPHERAL_BASE + 0x20_1000;
    /// SPI0 base
    pub const SPI0_BASE: usize = PERIPHERAL_BASE + 0x20_4000;
    /// BSC0 (I2C0) base
    pub const BSC0_BASE: usize = PERIPHERAL_BASE + 0x20_5000;
    /// BSC1 (I2C1) base
    pub const BSC1_BASE: usize = PERIPHERAL_BASE + 0x80_4000;

    /// AUX base (mini UART, SPI1, SPI2)
    pub const AUX_BASE: usize = PERIPHERAL_BASE + 0x21_5000;
    /// Mini UART base
    pub const MINI_UART_BASE: usize = AUX_BASE + 0x40;

    /// Interrupt controller
    pub const IRQ_BASE: usize = PERIPHERAL_BASE + 0xB_200;

    /// System timer
    pub const TIMER_BASE: usize = PERIPHERAL_BASE + 0x3000;

    /// ARM timer
    pub const ARM_TIMER_BASE: usize = PERIPHERAL_BASE + 0xB_400;

    /// Mailbox
    pub const MAILBOX_BASE: usize = PERIPHERAL_BASE + 0xB_880;

    /// Power management
    pub const PM_BASE: usize = PERIPHERAL_BASE + 0x10_0000;

    /// Watchdog
    pub const WATCHDOG_BASE: usize = PERIPHERAL_BASE + 0x10_001C;
}

/// Memory map for FE310 (SiFive HiFive1)
#[cfg(feature = "fe310")]
pub mod fe310 {
    /// Memory-mapped registers base
    pub const PERIPHERALS_BASE: usize = 0x1000_0000;

    /// CLINT base (Core Local Interruptor)
    pub const CLINT_BASE: usize = 0x0200_0000;
    /// PLIC base (Platform Level Interrupt Controller)
    pub const PLIC_BASE: usize = 0x0C00_0000;

    /// AON (Always-On) base
    pub const AON_BASE: usize = 0x1000_0000;
    /// PRCI (Power, Reset, Clock, Interrupt) base
    pub const PRCI_BASE: usize = 0x1000_8000;
    /// OTP base
    pub const OTP_BASE: usize = 0x1001_0000;

    /// GPIO base
    pub const GPIO_BASE: usize = 0x1001_2000;
    /// UART0 base
    pub const UART0_BASE: usize = 0x1001_3000;
    /// UART1 base
    pub const UART1_BASE: usize = 0x1002_3000;

    /// QSPI0 base (flash)
    pub const QSPI0_BASE: usize = 0x1001_4000;
    /// SPI1 base
    pub const SPI1_BASE: usize = 0x1002_4000;
    /// SPI2 base
    pub const SPI2_BASE: usize = 0x1003_4000;

    /// I2C base
    pub const I2C_BASE: usize = 0x1001_6000;

    /// PWM0 base
    pub const PWM0_BASE: usize = 0x1001_5000;
    /// PWM1 base
    pub const PWM1_BASE: usize = 0x1002_5000;
    /// PWM2 base
    pub const PWM2_BASE: usize = 0x1003_5000;

    /// Flash base
    pub const FLASH_BASE: usize = 0x2000_0000;
    /// SRAM base
    pub const SRAM_BASE: usize = 0x8000_0000;
}
