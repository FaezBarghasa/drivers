//! ARMv7 architecture support
//!
//! This module provides ARMv7-specific implementations and utilities.

use core::arch::asm;

/// ARMv7 register access utilities
pub mod registers {
    use core::arch::asm;

    /// Read CPSR (Current Program Status Register)
    #[inline]
    pub fn cpsr() -> u32 {
        let value: u32;
        unsafe {
            asm!("mrs {}, cpsr", out(reg) value);
        }
        value
    }

    /// Read CPUID
    #[inline]
    pub fn cpuid() -> u32 {
        let value: u32;
        unsafe {
            asm!("mrc p15, 0, {}, c0, c0, 0", out(reg) value);
        }
        value
    }

    /// Read SCTLR (System Control Register)
    #[inline]
    pub fn sctlr() -> u32 {
        let value: u32;
        unsafe {
            asm!("mrc p15, 0, {}, c1, c0, 0", out(reg) value);
        }
        value
    }

    /// Write SCTLR
    #[inline]
    pub unsafe fn write_sctlr(value: u32) {
        asm!("mcr p15, 0, {}, c1, c0, 0", in(reg) value);
    }

    /// Read VBAR (Vector Base Address Register)
    #[inline]
    pub fn vbar() -> u32 {
        let value: u32;
        unsafe {
            asm!("mrc p15, 0, {}, c12, c0, 0", out(reg) value);
        }
        value
    }

    /// Write VBAR
    #[inline]
    pub unsafe fn write_vbar(value: u32) {
        asm!("mcr p15, 0, {}, c12, c0, 0", in(reg) value);
    }
}

/// Interrupt control
pub mod interrupts {
    use core::arch::asm;

    /// Disable interrupts and return previous state
    #[inline]
    pub fn disable() -> bool {
        let cpsr: u32;
        unsafe {
            asm!(
                "mrs {}, cpsr",
                "cpsid i",
                out(reg) cpsr,
            );
        }
        cpsr & (1 << 7) == 0 // Return true if interrupts were enabled
    }

    /// Enable interrupts
    #[inline]
    pub fn enable() {
        unsafe {
            asm!("cpsie i");
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
        let cpsr: u32;
        unsafe {
            asm!("mrs {}, cpsr", out(reg) cpsr);
        }
        cpsr & (1 << 7) == 0
    }

    /// Wait for interrupt (low power)
    #[inline]
    pub fn wait() {
        unsafe {
            asm!("wfi");
        }
    }
}

/// Cache control
pub mod cache {
    use core::arch::asm;

    /// Invalidate instruction cache
    #[inline]
    pub unsafe fn invalidate_icache() {
        asm!("mcr p15, 0, {0}, c7, c5, 0", in(reg) 0u32);
    }

    /// Invalidate data cache line by address
    #[inline]
    pub unsafe fn invalidate_dcache_line(addr: usize) {
        asm!("mcr p15, 0, {}, c7, c6, 1", in(reg) addr);
    }

    /// Clean data cache line by address
    #[inline]
    pub unsafe fn clean_dcache_line(addr: usize) {
        asm!("mcr p15, 0, {}, c7, c10, 1", in(reg) addr);
    }

    /// Clean and invalidate data cache line
    #[inline]
    pub unsafe fn clean_invalidate_dcache_line(addr: usize) {
        asm!("mcr p15, 0, {}, c7, c14, 1", in(reg) addr);
    }

    /// Data synchronization barrier
    #[inline]
    pub fn dsb() {
        unsafe {
            asm!("dsb");
        }
    }

    /// Data memory barrier
    #[inline]
    pub fn dmb() {
        unsafe {
            asm!("dmb");
        }
    }

    /// Instruction synchronization barrier
    #[inline]
    pub fn isb() {
        unsafe {
            asm!("isb");
        }
    }
}

/// Memory-mapped I/O utilities
pub mod mmio {
    use core::ptr::{read_volatile, write_volatile};

    /// Read 32-bit value from memory-mapped register
    #[inline]
    pub unsafe fn read32(addr: usize) -> u32 {
        read_volatile(addr as *const u32)
    }

    /// Write 32-bit value to memory-mapped register
    #[inline]
    pub unsafe fn write32(addr: usize, value: u32) {
        write_volatile(addr as *mut u32, value);
    }

    /// Read 16-bit value
    #[inline]
    pub unsafe fn read16(addr: usize) -> u16 {
        read_volatile(addr as *const u16)
    }

    /// Write 16-bit value
    #[inline]
    pub unsafe fn write16(addr: usize, value: u16) {
        write_volatile(addr as *mut u16, value);
    }

    /// Read 8-bit value
    #[inline]
    pub unsafe fn read8(addr: usize) -> u8 {
        read_volatile(addr as *const u8)
    }

    /// Write 8-bit value
    #[inline]
    pub unsafe fn write8(addr: usize, value: u8) {
        write_volatile(addr as *mut u8, value);
    }

    /// Set bits in a register
    #[inline]
    pub unsafe fn set_bits32(addr: usize, mask: u32) {
        let value = read32(addr);
        write32(addr, value | mask);
    }

    /// Clear bits in a register
    #[inline]
    pub unsafe fn clear_bits32(addr: usize, mask: u32) {
        let value = read32(addr);
        write32(addr, value & !mask);
    }

    /// Modify bits in a register
    #[inline]
    pub unsafe fn modify32(addr: usize, clear_mask: u32, set_mask: u32) {
        let value = read32(addr);
        write32(addr, (value & !clear_mask) | set_mask);
    }
}

/// GIC (Generic Interrupt Controller) interface
pub mod gic {
    /// GIC Distributor interface
    pub struct GicDistributor {
        base: usize,
    }

    impl GicDistributor {
        /// Create new distributor interface
        pub const fn new(base: usize) -> Self {
            Self { base }
        }

        /// Enable the distributor
        pub unsafe fn enable(&self) {
            let ctrl = self.base;
            super::mmio::set_bits32(ctrl, 1);
        }

        /// Disable the distributor
        pub unsafe fn disable(&self) {
            let ctrl = self.base;
            super::mmio::clear_bits32(ctrl, 1);
        }

        /// Enable an interrupt
        pub unsafe fn enable_interrupt(&self, irq: u32) {
            let offset = 0x100 + (irq / 32 * 4) as usize;
            let bit = 1 << (irq % 32);
            super::mmio::write32(self.base + offset, bit);
        }

        /// Disable an interrupt
        pub unsafe fn disable_interrupt(&self, irq: u32) {
            let offset = 0x180 + (irq / 32 * 4) as usize;
            let bit = 1 << (irq % 32);
            super::mmio::write32(self.base + offset, bit);
        }

        /// Set interrupt priority
        pub unsafe fn set_priority(&self, irq: u32, priority: u8) {
            let offset = 0x400 + irq as usize;
            super::mmio::write8(self.base + offset, priority);
        }

        /// Set interrupt target CPU
        pub unsafe fn set_target(&self, irq: u32, target: u8) {
            let offset = 0x800 + irq as usize;
            super::mmio::write8(self.base + offset, target);
        }
    }

    /// GIC CPU interface
    pub struct GicCpuInterface {
        base: usize,
    }

    impl GicCpuInterface {
        /// Create new CPU interface
        pub const fn new(base: usize) -> Self {
            Self { base }
        }

        /// Enable the CPU interface
        pub unsafe fn enable(&self) {
            let ctrl = self.base;
            super::mmio::set_bits32(ctrl, 1);
        }

        /// Set priority mask
        pub unsafe fn set_priority_mask(&self, mask: u8) {
            let pmr = self.base + 0x4;
            super::mmio::write32(pmr, mask as u32);
        }

        /// Acknowledge interrupt (get IRQ number)
        pub unsafe fn acknowledge(&self) -> u32 {
            let iar = self.base + 0xC;
            super::mmio::read32(iar)
        }

        /// End of interrupt
        pub unsafe fn end_of_interrupt(&self, irq: u32) {
            let eoir = self.base + 0x10;
            super::mmio::write32(eoir, irq);
        }
    }
}

/// Delay function using cycle counting
pub fn delay_cycles(cycles: u32) {
    for _ in 0..cycles {
        unsafe { asm!("nop") };
    }
}

/// Delay in microseconds (approximate, depends on CPU frequency)
pub fn delay_us(us: u32, cpu_freq_mhz: u32) {
    let cycles = us * cpu_freq_mhz;
    delay_cycles(cycles);
}
