//! Xtensa architecture support (ESP32)
//!
//! This module provides Xtensa-specific implementations for ESP32 family.

/// Xtensa special register access
pub mod registers {
    /// Read PS (Processor Status) register
    #[inline]
    pub fn ps() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("rsr {}, ps", out(reg) value);
        }
        value
    }

    /// Write PS register
    #[inline]
    pub unsafe fn write_ps(value: u32) {
        core::arch::asm!("wsr {}, ps", in(reg) value);
        core::arch::asm!("rsync");
    }

    /// Read SAR (Shift Amount Register)
    #[inline]
    pub fn sar() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("rsr {}, sar", out(reg) value);
        }
        value
    }

    /// Read CCOUNT (Cycle Counter)
    #[inline]
    pub fn ccount() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("rsr {}, ccount", out(reg) value);
        }
        value
    }

    /// Read VECBASE (Vector Base Address)
    #[inline]
    pub fn vecbase() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("rsr {}, vecbase", out(reg) value);
        }
        value
    }

    /// Write VECBASE
    #[inline]
    pub unsafe fn write_vecbase(value: u32) {
        core::arch::asm!("wsr {}, vecbase", in(reg) value);
    }

    /// Read PRID (Processor ID) - identifies core on multi-core ESP32
    #[inline]
    pub fn prid() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("rsr {}, prid", out(reg) value);
        }
        value
    }
}

/// Interrupt control
pub mod interrupts {
    /// PS register bits
    const PS_INTLEVEL_MASK: u32 = 0x0F;
    const PS_EXCM: u32 = 1 << 4;
    const PS_UM: u32 = 1 << 5;

    /// Disable interrupts and return previous interrupt level
    #[inline]
    pub fn disable() -> u32 {
        let ps = super::registers::ps();
        let old_level = ps & PS_INTLEVEL_MASK;
        unsafe {
            // Set interrupt level to max (15)
            super::registers::write_ps((ps & !PS_INTLEVEL_MASK) | 15);
        }
        old_level
    }

    /// Restore interrupt level
    #[inline]
    pub fn restore(level: u32) {
        let ps = super::registers::ps();
        unsafe {
            super::registers::write_ps((ps & !PS_INTLEVEL_MASK) | (level & PS_INTLEVEL_MASK));
        }
    }

    /// Enable all interrupts
    #[inline]
    pub fn enable() {
        let ps = super::registers::ps();
        unsafe {
            super::registers::write_ps(ps & !PS_INTLEVEL_MASK);
        }
    }

    /// Get current interrupt level
    #[inline]
    pub fn level() -> u32 {
        super::registers::ps() & PS_INTLEVEL_MASK
    }

    /// Wait for interrupt
    #[inline]
    pub fn wait() {
        unsafe {
            core::arch::asm!("waiti 0");
        }
    }
}

/// Memory barriers
pub mod barriers {
    /// Memory fence
    #[inline]
    pub fn memw() {
        unsafe {
            core::arch::asm!("memw");
        }
    }

    /// Instruction sync
    #[inline]
    pub fn isync() {
        unsafe {
            core::arch::asm!("isync");
        }
    }

    /// Data sync
    #[inline]
    pub fn dsync() {
        unsafe {
            core::arch::asm!("dsync");
        }
    }

    /// Read sync
    #[inline]
    pub fn rsync() {
        unsafe {
            core::arch::asm!("rsync");
        }
    }

    /// Extended sync
    #[inline]
    pub fn esync() {
        unsafe {
            core::arch::asm!("esync");
        }
    }
}

/// Cache control
pub mod cache {
    /// Invalidate instruction cache line
    #[inline]
    pub unsafe fn invalidate_icache_line(addr: u32) {
        core::arch::asm!("ihi {}, 0", in(reg) addr);
    }

    /// Invalidate data cache line
    #[inline]
    pub unsafe fn invalidate_dcache_line(addr: u32) {
        core::arch::asm!("dhi {}, 0", in(reg) addr);
    }

    /// Write back data cache line
    #[inline]
    pub unsafe fn writeback_dcache_line(addr: u32) {
        core::arch::asm!("dhwb {}, 0", in(reg) addr);
    }

    /// Write back and invalidate data cache line
    #[inline]
    pub unsafe fn writeback_invalidate_dcache_line(addr: u32) {
        core::arch::asm!("dhwbi {}, 0", in(reg) addr);
    }
}

/// ESP32-specific peripherals base addresses
pub mod esp32 {
    /// GPIO base address
    pub const GPIO_BASE: usize = 0x3FF4_4000;
    /// SPI base addresses
    pub const SPI0_BASE: usize = 0x3FF4_2000;
    pub const SPI1_BASE: usize = 0x3FF4_2000;
    pub const SPI2_BASE: usize = 0x3FF6_4000;
    pub const SPI3_BASE: usize = 0x3FF6_5000;
    /// I2C base addresses
    pub const I2C0_BASE: usize = 0x3FF5_3000;
    pub const I2C1_BASE: usize = 0x3FF6_7000;
    /// UART base addresses
    pub const UART0_BASE: usize = 0x3FF4_0000;
    pub const UART1_BASE: usize = 0x3FF5_0000;
    pub const UART2_BASE: usize = 0x3FF6_E000;
    /// Timer base addresses
    pub const TIMER_GROUP0_BASE: usize = 0x3FF5_F000;
    pub const TIMER_GROUP1_BASE: usize = 0x3FF60000;
    /// RTC base address
    pub const RTC_BASE: usize = 0x3FF4_8000;
    /// WiFi base address
    pub const WIFI_BASE: usize = 0x3FF7_3000;
    /// Bluetooth base address
    pub const BT_BASE: usize = 0x3FF7_4000;
}

/// Delay using cycle counter
pub fn delay_cycles(cycles: u32) {
    let start = registers::ccount();
    while registers::ccount().wrapping_sub(start) < cycles {
        core::hint::spin_loop();
    }
}

/// Delay in microseconds
pub fn delay_us(us: u32, cpu_freq_mhz: u32) {
    let cycles = us * cpu_freq_mhz;
    delay_cycles(cycles);
}

/// Get the current core ID (0 or 1 for dual-core ESP32)
pub fn core_id() -> u32 {
    registers::prid() & 1
}
