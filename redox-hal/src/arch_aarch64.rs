//! AArch64 (ARM 64-bit) architecture support
//!
//! This module provides AArch64-specific implementations for Raspberry Pi 3/4/5,
//! Rockchip RK35xx, and other 64-bit ARM platforms.

/// System register access
pub mod registers {
    /// Read MPIDR_EL1 (Multiprocessor Affinity Register)
    #[inline]
    pub fn mpidr_el1() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, mpidr_el1", out(reg) value);
        }
        value
    }

    /// Read MIDR_EL1 (Main ID Register)
    #[inline]
    pub fn midr_el1() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, midr_el1", out(reg) value);
        }
        value
    }

    /// Read CurrentEL
    #[inline]
    pub fn current_el() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, CurrentEL", out(reg) value);
        }
        (value >> 2) & 0x3
    }

    /// Read CNTFRQ_EL0 (Counter Frequency)
    #[inline]
    pub fn cntfrq_el0() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntfrq_el0", out(reg) value);
        }
        value
    }

    /// Read CNTVCT_EL0 (Virtual Count)
    #[inline]
    pub fn cntvct_el0() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) value);
        }
        value
    }

    /// Read CNTPCT_EL0 (Physical Count)
    #[inline]
    pub fn cntpct_el0() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntpct_el0", out(reg) value);
        }
        value
    }

    /// Read SCTLR_EL1 (System Control Register)
    #[inline]
    pub fn sctlr_el1() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, sctlr_el1", out(reg) value);
        }
        value
    }

    /// Write SCTLR_EL1
    #[inline]
    pub unsafe fn write_sctlr_el1(value: u64) {
        core::arch::asm!("msr sctlr_el1, {}", in(reg) value);
        core::arch::asm!("isb");
    }

    /// Read VBAR_EL1 (Vector Base Address Register)
    #[inline]
    pub fn vbar_el1() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, vbar_el1", out(reg) value);
        }
        value
    }

    /// Write VBAR_EL1
    #[inline]
    pub unsafe fn write_vbar_el1(value: u64) {
        core::arch::asm!("msr vbar_el1, {}", in(reg) value);
        core::arch::asm!("isb");
    }

    /// Read DAIF (Interrupt Mask Bits)
    #[inline]
    pub fn daif() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mrs {}, daif", out(reg) value);
        }
        value
    }
}

/// Interrupt control
pub mod interrupts {
    /// DAIF bits
    const DAIF_D: u64 = 1 << 9; // Debug
    const DAIF_A: u64 = 1 << 8; // SError
    const DAIF_I: u64 = 1 << 7; // IRQ
    const DAIF_F: u64 = 1 << 6; // FIQ

    /// Disable all interrupts and return previous state
    #[inline]
    pub fn disable() -> u64 {
        let daif = super::registers::daif();
        unsafe {
            core::arch::asm!("msr daifset, #0xf");
        }
        daif
    }

    /// Enable all interrupts
    #[inline]
    pub fn enable() {
        unsafe {
            core::arch::asm!("msr daifclr, #0xf");
        }
    }

    /// Enable IRQ only
    #[inline]
    pub fn enable_irq() {
        unsafe {
            core::arch::asm!("msr daifclr, #2"); // Clear I bit
        }
    }

    /// Disable IRQ only
    #[inline]
    pub fn disable_irq() {
        unsafe {
            core::arch::asm!("msr daifset, #2"); // Set I bit
        }
    }

    /// Restore interrupt state
    #[inline]
    pub fn restore(daif: u64) {
        unsafe {
            core::arch::asm!("msr daif, {}", in(reg) daif);
        }
    }

    /// Check if IRQ is enabled
    #[inline]
    pub fn is_irq_enabled() -> bool {
        super::registers::daif() & DAIF_I == 0
    }

    /// Wait for interrupt
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

    /// Send event
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
    pub fn dsb_sy() {
        unsafe {
            core::arch::asm!("dsb sy");
        }
    }

    /// Data synchronization barrier (inner shareable)
    #[inline]
    pub fn dsb_ish() {
        unsafe {
            core::arch::asm!("dsb ish");
        }
    }

    /// Data memory barrier
    #[inline]
    pub fn dmb_sy() {
        unsafe {
            core::arch::asm!("dmb sy");
        }
    }

    /// Data memory barrier (inner shareable)
    #[inline]
    pub fn dmb_ish() {
        unsafe {
            core::arch::asm!("dmb ish");
        }
    }

    /// Instruction synchronization barrier
    #[inline]
    pub fn isb() {
        unsafe {
            core::arch::asm!("isb");
        }
    }
}

/// Cache control
pub mod cache {
    /// Invalidate instruction cache (all)
    #[inline]
    pub unsafe fn invalidate_icache_all() {
        core::arch::asm!("ic iallu");
        super::barriers::dsb_sy();
        super::barriers::isb();
    }

    /// Invalidate data cache line by VA
    #[inline]
    pub unsafe fn invalidate_dcache_line(addr: u64) {
        core::arch::asm!("dc ivac, {}", in(reg) addr);
    }

    /// Clean data cache line by VA
    #[inline]
    pub unsafe fn clean_dcache_line(addr: u64) {
        core::arch::asm!("dc cvac, {}", in(reg) addr);
    }

    /// Clean and invalidate data cache line by VA
    #[inline]
    pub unsafe fn clean_invalidate_dcache_line(addr: u64) {
        core::arch::asm!("dc civac, {}", in(reg) addr);
    }

    /// Zero data cache line by VA
    #[inline]
    pub unsafe fn zero_dcache_line(addr: u64) {
        core::arch::asm!("dc zva, {}", in(reg) addr);
    }
}

/// GICv2/GICv3 interrupt controller
pub mod gic {
    /// GIC Distributor
    pub struct GicDistributor {
        base: usize,
    }

    impl GicDistributor {
        /// Create new distributor interface
        pub const fn new(base: usize) -> Self {
            Self { base }
        }

        /// Enable distributor
        pub unsafe fn enable(&self) {
            let ctlr = self.base;
            core::ptr::write_volatile(ctlr as *mut u32, 3); // Enable Group 0 and 1
        }

        /// Disable distributor
        pub unsafe fn disable(&self) {
            let ctlr = self.base;
            core::ptr::write_volatile(ctlr as *mut u32, 0);
        }

        /// Enable interrupt
        pub unsafe fn enable_interrupt(&self, irq: u32) {
            let offset = 0x100 + (irq / 32 * 4) as usize;
            let bit = 1u32 << (irq % 32);
            core::ptr::write_volatile((self.base + offset) as *mut u32, bit);
        }

        /// Disable interrupt
        pub unsafe fn disable_interrupt(&self, irq: u32) {
            let offset = 0x180 + (irq / 32 * 4) as usize;
            let bit = 1u32 << (irq % 32);
            core::ptr::write_volatile((self.base + offset) as *mut u32, bit);
        }

        /// Set interrupt priority
        pub unsafe fn set_priority(&self, irq: u32, priority: u8) {
            let offset = 0x400 + irq as usize;
            core::ptr::write_volatile((self.base + offset) as *mut u8, priority);
        }

        /// Set interrupt target CPU
        pub unsafe fn set_target(&self, irq: u32, target: u8) {
            let offset = 0x800 + irq as usize;
            core::ptr::write_volatile((self.base + offset) as *mut u8, target);
        }
    }

    /// GIC CPU Interface
    pub struct GicCpuInterface {
        base: usize,
    }

    impl GicCpuInterface {
        /// Create new CPU interface
        pub const fn new(base: usize) -> Self {
            Self { base }
        }

        /// Enable CPU interface
        pub unsafe fn enable(&self) {
            core::ptr::write_volatile(self.base as *mut u32, 3); // Enable Group 0 and 1
        }

        /// Set priority mask
        pub unsafe fn set_priority_mask(&self, mask: u8) {
            core::ptr::write_volatile((self.base + 0x4) as *mut u32, mask as u32);
        }

        /// Acknowledge interrupt
        pub unsafe fn acknowledge(&self) -> u32 {
            core::ptr::read_volatile((self.base + 0xC) as *const u32)
        }

        /// End of interrupt
        pub unsafe fn end_of_interrupt(&self, irq: u32) {
            core::ptr::write_volatile((self.base + 0x10) as *mut u32, irq);
        }
    }
}

/// Get core ID (affinity)
pub fn core_id() -> u32 {
    (registers::mpidr_el1() & 0xFF) as u32
}

/// Get timer frequency
pub fn timer_frequency() -> u64 {
    registers::cntfrq_el0()
}

/// Get current timer count
pub fn timer_count() -> u64 {
    registers::cntvct_el0()
}

/// Delay using generic timer
pub fn delay_cycles(cycles: u64) {
    let start = timer_count();
    while timer_count().wrapping_sub(start) < cycles {
        core::hint::spin_loop();
    }
}

/// Delay in microseconds
pub fn delay_us(us: u64) {
    let freq = timer_frequency();
    let cycles = (us * freq) / 1_000_000;
    delay_cycles(cycles);
}

/// Delay in milliseconds
pub fn delay_ms(ms: u64) {
    delay_us(ms * 1000);
}
