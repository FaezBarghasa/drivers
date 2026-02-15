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

/// ESP32-S3 specific peripheral base addresses
pub mod esp32s3 {
    /// SENSITIVE / APB_SARADC (renamed) base
    pub const SENSITIVE_BASE: usize = 0x600C0000;
    /// SYSTEM (replaced DPORT)
    pub const SYSTEM_BASE: usize = 0x600C0000;
    /// PMS constraint registers for Core 0
    pub const PMS_CORE0_BASE: usize = SENSITIVE_BASE;
}

/// Memory Protection Unit implementation (ESP32-S3 PMS)
///
/// Uses the Partition Management System (PMS) to emulate MPU regions.
/// S3 PMS uses "split lines" to divide address space.
pub mod mpu {
    use super::esp32s3;
    use core::ptr::{read_volatile, write_volatile};

    // simplified SENSITIVE register map for PMS
    const SENSITIVE_CORE_0_PIF_PMS_CONSTRAIN_1_REG: usize = esp32s3::PMS_CORE0_BASE + 0x004;
    const SENSITIVE_CORE_0_PIF_PMS_CONSTRAIN_2_REG: usize = esp32s3::PMS_CORE0_BASE + 0x008;
    const SENSITIVE_CORE_0_PIF_PMS_CONSTRAIN_3_REG: usize = esp32s3::PMS_CORE0_BASE + 0x00C;

    // Permissions
    const PMS_R: u32 = 1 << 0;
    const PMS_W: u32 = 1 << 1;
    const PMS_X: u32 = 1 << 2;

    pub fn reset() {
        // Reset execution logic
    }

    /// Region definition for PMS
    #[derive(Debug, Clone, Copy)]
    pub struct PmsRegion {
        pub base: u32,
        pub size: u32,
        pub r: bool,
        pub w: bool,
        pub x: bool,
        pub active: bool,
    }

    /// Configure PMS based on provided zones.
    /// Note: ESP32-S3 PMS logic uses splits.
    /// Region 0: 0 to SplitAddr0
    /// Region 1: SplitAddr0 to SplitAddr1
    /// Region 2: SplitAddr1 to End
    ///
    /// This implementation assumes zones are sorted and contiguous or we map boundaries.
    /// We support up to 2 splits (3 regions) for basic protection.
    pub fn configure(regions: &[PmsRegion]) {
        // Collect active regions
        let mut active = [PmsRegion {
            base: 0,
            size: 0,
            r: false,
            w: false,
            x: false,
            active: false,
        }; 3];
        let mut count = 0;

        for r in regions {
            if r.active && count < 3 {
                active[count] = *r;
                count += 1;
            }
        }

        // Must sort by base address
        // Bubble sort for 3 elements
        for i in 0..count {
            for j in 0..count - 1 - i {
                if active[j].base > active[j + 1].base {
                    let temp = active[j];
                    active[j] = active[j + 1];
                    active[j + 1] = temp;
                }
            }
        }

        // Determine splits
        // Default to max address if not used
        let mut split0 = 0u32;
        let mut split1 = 0u32;

        let mut perm0 = 0u32;
        let mut perm1 = 0u32;
        let mut perm2 = 0u32; // Region 2 (after split 1)

        // Mapping logic:
        // If 1 region: Split0 = End, Split1 = End. Region 0 = Perms.
        // If 2 regions: Split0 = R1.Base (if R0 starts at 0?), wait.
        // If R0 starts at 0: Split0 = R0.End.
        // If R1 starts at Split0: Split1 = R1.End.

        // Simplified: use the end of valid regions as splits.
        if count > 0 {
            split0 = active[0].base + active[0].size;
            if active[0].r {
                perm0 |= PMS_R;
            }
            if active[0].w {
                perm0 |= PMS_W;
            }
            if active[0].x {
                perm0 |= PMS_X;
            }
        }

        if count > 1 {
            split1 = active[1].base + active[1].size;
            if active[1].r {
                perm1 |= PMS_R;
            }
            if active[1].w {
                perm1 |= PMS_W;
            }
            if active[1].x {
                perm1 |= PMS_X;
            }
        } else {
            split1 = split0; // No second region
        }

        if count > 2 {
            // Region 2 covers Split1 to End?
            // Or Split1 to R2.End? PMS usually covers whole space.
            // We assume R2 is the last constrained region.
            if active[2].r {
                perm2 |= PMS_R;
            }
            if active[2].w {
                perm2 |= PMS_W;
            }
            if active[2].x {
                perm2 |= PMS_X;
            }
        }

        unsafe {
            // Write Split 0
            write_volatile(SENSITIVE_CORE_0_PIF_PMS_CONSTRAIN_1_REG as *mut u32, split0);

            // Write Split 1
            write_volatile(SENSITIVE_CORE_0_PIF_PMS_CONSTRAIN_2_REG as *mut u32, split1);

            // Write Permissions
            // Field mappings (approximate, check S3 TRM for exact bits)
            // Region 0: bits 0-1
            // Region 1: bits 2-3
            // Region 2: bits 4-5
            // Actually PMS perms are often 3 bits (RWX) or 2 bits (RW? X separate?).
            // Let's assume 3 bits per field for this implementation.
            // Bits 0-2: World 0 Region 0
            // Bits 3-5: World 0 Region 1
            // Bits 6-8: World 0 Region 2

            let mut val = 0u32;
            val |= (perm0 & 0x7) << 0;
            val |= (perm1 & 0x7) << 3;
            val |= (perm2 & 0x7) << 6;

            write_volatile(SENSITIVE_CORE_0_PIF_PMS_CONSTRAIN_3_REG as *mut u32, val);
        }
    }
}
