//! ARM Cortex-M architecture support (STM32, Teensy, RP2040)
//!
//! This module provides Cortex-M specific implementations for ARMv7-M cores.

/// Cortex-M register access utilities
pub mod registers {
    /// Read PRIMASK (interrupt mask register)
    #[inline]
    pub fn primask() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("mrs {}, primask", out(reg) value);
        }
        value
    }

    /// Read BASEPRI (base priority mask)
    #[inline]
    pub fn basepri() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("mrs {}, basepri", out(reg) value);
        }
        value
    }

    /// Write BASEPRI
    #[inline]
    pub unsafe fn write_basepri(value: u32) {
        core::arch::asm!("msr basepri, {}", in(reg) value);
    }

    /// Read CONTROL register
    #[inline]
    pub fn control() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("mrs {}, control", out(reg) value);
        }
        value
    }

    /// Read PSP (Process Stack Pointer)
    #[inline]
    pub fn psp() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("mrs {}, psp", out(reg) value);
        }
        value
    }

    /// Read MSP (Main Stack Pointer)
    #[inline]
    pub fn msp() -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("mrs {}, msp", out(reg) value);
        }
        value
    }
}

/// Interrupt control
pub mod interrupts {
    /// Disable interrupts and return previous state
    #[inline]
    pub fn disable() -> bool {
        let primask: u32;
        unsafe {
            core::arch::asm!(
                "mrs {0}, primask",
                "cpsid i",
                out(reg) primask,
            );
        }
        primask & 1 == 0
    }

    /// Enable interrupts
    #[inline]
    pub fn enable() {
        unsafe {
            core::arch::asm!("cpsie i");
        }
    }

    /// Restore interrupt state
    #[inline]
    pub fn restore(enabled: bool) {
        if enabled {
            enable();
        }
    }

    /// Check if interrupts are enabled
    #[inline]
    pub fn is_enabled() -> bool {
        super::registers::primask() & 1 == 0
    }

    /// Wait for interrupt (low power)
    #[inline]
    pub fn wait() {
        unsafe {
            core::arch::asm!("wfi");
        }
    }

    /// Wait for event
    #[inline]
    pub fn wfe() {
        unsafe {
            core::arch::asm!("wfe");
        }
    }

    /// Send event (wake up other cores)
    #[inline]
    pub fn sev() {
        unsafe {
            core::arch::asm!("sev");
        }
    }
}

/// Memory barriers
pub mod barriers {
    /// Data synchronization barrier
    #[inline]
    pub fn dsb() {
        unsafe {
            core::arch::asm!("dsb sy");
        }
    }

    /// Data memory barrier
    #[inline]
    pub fn dmb() {
        unsafe {
            core::arch::asm!("dmb sy");
        }
    }

    /// Instruction synchronization barrier
    #[inline]
    pub fn isb() {
        unsafe {
            core::arch::asm!("isb sy");
        }
    }
}

/// NVIC (Nested Vectored Interrupt Controller)
pub mod nvic {
    /// NVIC base address
    const NVIC_BASE: usize = 0xE000_E100;

    /// Set Enable Register offset
    const ISER_OFFSET: usize = 0x000;
    /// Clear Enable Register offset
    const ICER_OFFSET: usize = 0x080;
    /// Set Pending Register offset
    const ISPR_OFFSET: usize = 0x100;
    /// Clear Pending Register offset
    const ICPR_OFFSET: usize = 0x180;
    /// Active Bit Register offset
    const IABR_OFFSET: usize = 0x200;
    /// Interrupt Priority Register offset
    const IPR_OFFSET: usize = 0x300;

    /// Enable an interrupt
    #[inline]
    pub unsafe fn enable(irq: u8) {
        let reg = NVIC_BASE + ISER_OFFSET + (irq as usize / 32) * 4;
        let bit = 1u32 << (irq % 32);
        core::ptr::write_volatile(reg as *mut u32, bit);
    }

    /// Disable an interrupt
    #[inline]
    pub unsafe fn disable(irq: u8) {
        let reg = NVIC_BASE + ICER_OFFSET + (irq as usize / 32) * 4;
        let bit = 1u32 << (irq % 32);
        core::ptr::write_volatile(reg as *mut u32, bit);
    }

    /// Check if interrupt is enabled
    #[inline]
    pub fn is_enabled(irq: u8) -> bool {
        let reg = NVIC_BASE + ISER_OFFSET + (irq as usize / 32) * 4;
        let bit = 1u32 << (irq % 32);
        unsafe { core::ptr::read_volatile(reg as *const u32) & bit != 0 }
    }

    /// Set interrupt pending
    #[inline]
    pub unsafe fn pend(irq: u8) {
        let reg = NVIC_BASE + ISPR_OFFSET + (irq as usize / 32) * 4;
        let bit = 1u32 << (irq % 32);
        core::ptr::write_volatile(reg as *mut u32, bit);
    }

    /// Clear interrupt pending
    #[inline]
    pub unsafe fn unpend(irq: u8) {
        let reg = NVIC_BASE + ICPR_OFFSET + (irq as usize / 32) * 4;
        let bit = 1u32 << (irq % 32);
        core::ptr::write_volatile(reg as *mut u32, bit);
    }

    /// Check if interrupt is pending
    #[inline]
    pub fn is_pending(irq: u8) -> bool {
        let reg = NVIC_BASE + ISPR_OFFSET + (irq as usize / 32) * 4;
        let bit = 1u32 << (irq % 32);
        unsafe { core::ptr::read_volatile(reg as *const u32) & bit != 0 }
    }

    /// Check if interrupt is active
    #[inline]
    pub fn is_active(irq: u8) -> bool {
        let reg = NVIC_BASE + IABR_OFFSET + (irq as usize / 32) * 4;
        let bit = 1u32 << (irq % 32);
        unsafe { core::ptr::read_volatile(reg as *const u32) & bit != 0 }
    }

    /// Set interrupt priority (0 = highest, 255 = lowest)
    #[inline]
    pub unsafe fn set_priority(irq: u8, priority: u8) {
        let reg = NVIC_BASE + IPR_OFFSET + irq as usize;
        core::ptr::write_volatile(reg as *mut u8, priority);
    }

    /// Get interrupt priority
    #[inline]
    pub fn priority(irq: u8) -> u8 {
        let reg = NVIC_BASE + IPR_OFFSET + irq as usize;
        unsafe { core::ptr::read_volatile(reg as *const u8) }
    }
}

/// SysTick timer
pub mod systick {
    /// SysTick base address
    const SYST_BASE: usize = 0xE000_E010;

    /// Control and Status Register
    const CSR: usize = SYST_BASE + 0x00;
    /// Reload Value Register
    const RVR: usize = SYST_BASE + 0x04;
    /// Current Value Register
    const CVR: usize = SYST_BASE + 0x08;
    /// Calibration Value Register
    const CALIB: usize = SYST_BASE + 0x0C;

    /// CSR bits
    const ENABLE: u32 = 1 << 0;
    const TICKINT: u32 = 1 << 1;
    const CLKSOURCE: u32 = 1 << 2;
    const COUNTFLAG: u32 = 1 << 16;

    /// Configure SysTick
    #[inline]
    pub unsafe fn configure(reload: u32, use_processor_clock: bool, enable_interrupt: bool) {
        // Stop SysTick
        core::ptr::write_volatile(CSR as *mut u32, 0);

        // Set reload value
        core::ptr::write_volatile(RVR as *mut u32, reload & 0x00FF_FFFF);

        // Clear current value
        core::ptr::write_volatile(CVR as *mut u32, 0);

        // Configure and enable
        let mut csr = ENABLE;
        if use_processor_clock {
            csr |= CLKSOURCE;
        }
        if enable_interrupt {
            csr |= TICKINT;
        }
        core::ptr::write_volatile(CSR as *mut u32, csr);
    }

    /// Get current counter value
    #[inline]
    pub fn current() -> u32 {
        unsafe { core::ptr::read_volatile(CVR as *const u32) & 0x00FF_FFFF }
    }

    /// Check if countflag is set (has counted to 0)
    #[inline]
    pub fn has_wrapped() -> bool {
        unsafe { core::ptr::read_volatile(CSR as *const u32) & COUNTFLAG != 0 }
    }

    /// Disable SysTick
    #[inline]
    pub unsafe fn disable() {
        core::ptr::write_volatile(CSR as *mut u32, 0);
    }
}

/// SCB (System Control Block)
pub mod scb {
    /// SCB base address
    const SCB_BASE: usize = 0xE000_ED00;

    /// CPUID
    const CPUID: usize = SCB_BASE + 0x00;
    /// Interrupt Control and State Register
    const ICSR: usize = SCB_BASE + 0x04;
    /// Vector Table Offset Register
    const VTOR: usize = SCB_BASE + 0x08;
    /// Application Interrupt and Reset Control Register
    const AIRCR: usize = SCB_BASE + 0x0C;
    /// System Control Register
    const SCR: usize = SCB_BASE + 0x10;

    /// Get CPUID
    #[inline]
    pub fn cpuid() -> u32 {
        unsafe { core::ptr::read_volatile(CPUID as *const u32) }
    }

    /// Set vector table offset
    #[inline]
    pub unsafe fn set_vtor(addr: u32) {
        core::ptr::write_volatile(VTOR as *mut u32, addr);
    }

    /// Get vector table offset
    #[inline]
    pub fn vtor() -> u32 {
        unsafe { core::ptr::read_volatile(VTOR as *const u32) }
    }

    /// System reset
    #[inline]
    pub unsafe fn system_reset() -> ! {
        super::barriers::dsb();
        // VECTKEY | SYSRESETREQ
        core::ptr::write_volatile(AIRCR as *mut u32, 0x05FA_0004);
        super::barriers::dsb();
        loop {
            core::arch::asm!("wfi");
        }
    }

    /// Enable sleep-on-exit
    #[inline]
    pub unsafe fn enable_sleep_on_exit() {
        let scr = core::ptr::read_volatile(SCR as *const u32);
        core::ptr::write_volatile(SCR as *mut u32, scr | (1 << 1));
    }

    /// Enable deep sleep
    #[inline]
    pub unsafe fn enable_deep_sleep() {
        let scr = core::ptr::read_volatile(SCR as *const u32);
        core::ptr::write_volatile(SCR as *mut u32, scr | (1 << 2));
    }
}

/// Delay using cycle counting
pub fn delay_cycles(cycles: u32) {
    // Use SysTick for precise delays
    let start = systick::current();
    let reload = unsafe { core::ptr::read_volatile(0xE000_E014 as *const u32) } & 0x00FF_FFFF;

    let mut elapsed = 0u32;
    let mut last = start;

    while elapsed < cycles {
        let current = systick::current();
        if current <= last {
            elapsed += last - current;
        } else {
            elapsed += last + (reload - current);
        }
        last = current;
    }
}

/// Delay in microseconds
pub fn delay_us(us: u32, cpu_freq_mhz: u32) {
    let cycles = us * cpu_freq_mhz;
    delay_cycles(cycles);
}
