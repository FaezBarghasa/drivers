//! RISC-V 32 architecture support
//!
//! This module provides RISC-V 32-bit specific implementations and utilities.

use core::arch::asm;

/// CSR (Control and Status Register) access
pub mod csr {
    use core::arch::asm;

    /// Read mstatus register
    #[inline]
    pub fn mstatus() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mstatus", out(reg) value);
        }
        value
    }

    /// Write mstatus register
    #[inline]
    pub unsafe fn write_mstatus(value: u32) {
        asm!("csrw mstatus, {}", in(reg) value);
    }

    /// Read mie (Machine Interrupt Enable) register
    #[inline]
    pub fn mie() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mie", out(reg) value);
        }
        value
    }

    /// Write mie register
    #[inline]
    pub unsafe fn write_mie(value: u32) {
        asm!("csrw mie, {}", in(reg) value);
    }

    /// Read mip (Machine Interrupt Pending) register
    #[inline]
    pub fn mip() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mip", out(reg) value);
        }
        value
    }

    /// Read mtvec (Machine Trap Vector) register
    #[inline]
    pub fn mtvec() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mtvec", out(reg) value);
        }
        value
    }

    /// Write mtvec register
    #[inline]
    pub unsafe fn write_mtvec(value: u32) {
        asm!("csrw mtvec, {}", in(reg) value);
    }

    /// Read mepc (Machine Exception PC) register
    #[inline]
    pub fn mepc() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mepc", out(reg) value);
        }
        value
    }

    /// Write mepc register
    #[inline]
    pub unsafe fn write_mepc(value: u32) {
        asm!("csrw mepc, {}", in(reg) value);
    }

    /// Read mcause register
    #[inline]
    pub fn mcause() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mcause", out(reg) value);
        }
        value
    }

    /// Read mtval register
    #[inline]
    pub fn mtval() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mtval", out(reg) value);
        }
        value
    }

    /// Read mhartid (Hardware Thread ID) register
    #[inline]
    pub fn mhartid() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, mhartid", out(reg) value);
        }
        value
    }

    /// Read cycle counter (lower 32 bits)
    #[inline]
    pub fn cycle() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, cycle", out(reg) value);
        }
        value
    }

    /// Read cycle counter (upper 32 bits)
    #[inline]
    pub fn cycleh() -> u32 {
        let value: u32;
        unsafe {
            asm!("csrr {}, cycleh", out(reg) value);
        }
        value
    }

    /// Read 64-bit cycle counter
    #[inline]
    pub fn cycle64() -> u64 {
        loop {
            let hi = cycleh();
            let lo = cycle();
            let hi2 = cycleh();
            if hi == hi2 {
                return ((hi as u64) << 32) | (lo as u64);
            }
        }
    }
}

/// Interrupt control
pub mod interrupts {
    use core::arch::asm;

    /// mstatus MIE bit
    const MSTATUS_MIE: u32 = 1 << 3;

    /// Disable interrupts and return previous state
    #[inline]
    pub fn disable() -> bool {
        let mstatus = super::csr::mstatus();
        let was_enabled = mstatus & MSTATUS_MIE != 0;
        unsafe {
            asm!("csrci mstatus, {}", const MSTATUS_MIE);
        }
        was_enabled
    }

    /// Enable interrupts
    #[inline]
    pub fn enable() {
        unsafe {
            asm!("csrsi mstatus, {}", const MSTATUS_MIE);
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
        super::csr::mstatus() & MSTATUS_MIE != 0
    }

    /// Wait for interrupt
    #[inline]
    pub fn wait() {
        unsafe {
            asm!("wfi");
        }
    }
}

/// Memory barriers
pub mod barriers {
    use core::arch::asm;

    /// Fence instruction (full memory barrier)
    #[inline]
    pub fn fence() {
        unsafe {
            asm!("fence");
        }
    }

    /// Fence.i instruction (instruction fence)
    #[inline]
    pub fn fence_i() {
        unsafe {
            asm!("fence.i");
        }
    }

    /// I/O fence
    #[inline]
    pub fn fence_io() {
        unsafe {
            asm!("fence iorw, iorw");
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
}

/// PLIC (Platform-Level Interrupt Controller) interface
pub mod plic {
    /// PLIC instance
    pub struct Plic {
        base: usize,
    }

    impl Plic {
        /// Create new PLIC interface
        pub const fn new(base: usize) -> Self {
            Self { base }
        }

        /// Set interrupt priority
        pub unsafe fn set_priority(&self, irq: u32, priority: u32) {
            let offset = (irq * 4) as usize;
            super::mmio::write32(self.base + offset, priority);
        }

        /// Get interrupt priority
        pub unsafe fn priority(&self, irq: u32) -> u32 {
            let offset = (irq * 4) as usize;
            super::mmio::read32(self.base + offset)
        }

        /// Enable interrupt for a hart/context
        pub unsafe fn enable(&self, context: u32, irq: u32) {
            let offset = 0x2000 + (context * 0x80 + irq / 32 * 4) as usize;
            let bit = 1 << (irq % 32);
            super::mmio::set_bits32(self.base + offset, bit);
        }

        /// Disable interrupt for a hart/context
        pub unsafe fn disable(&self, context: u32, irq: u32) {
            let offset = 0x2000 + (context * 0x80 + irq / 32 * 4) as usize;
            let bit = 1 << (irq % 32);
            super::mmio::clear_bits32(self.base + offset, bit);
        }

        /// Set priority threshold for a context
        pub unsafe fn set_threshold(&self, context: u32, threshold: u32) {
            let offset = 0x200000 + (context * 0x1000) as usize;
            super::mmio::write32(self.base + offset, threshold);
        }

        /// Claim interrupt
        pub unsafe fn claim(&self, context: u32) -> u32 {
            let offset = 0x200004 + (context * 0x1000) as usize;
            super::mmio::read32(self.base + offset)
        }

        /// Complete interrupt
        pub unsafe fn complete(&self, context: u32, irq: u32) {
            let offset = 0x200004 + (context * 0x1000) as usize;
            super::mmio::write32(self.base + offset, irq);
        }
    }
}

/// CLINT (Core Local Interruptor) interface
pub mod clint {
    /// CLINT instance
    pub struct Clint {
        base: usize,
    }

    impl Clint {
        /// Create new CLINT interface
        pub const fn new(base: usize) -> Self {
            Self { base }
        }

        /// Get mtime value
        pub fn mtime(&self) -> u64 {
            unsafe {
                let lo = super::mmio::read32(self.base + 0xBFF8);
                let hi = super::mmio::read32(self.base + 0xBFFC);
                ((hi as u64) << 32) | (lo as u64)
            }
        }

        /// Set mtimecmp for a hart
        pub unsafe fn set_mtimecmp(&self, hart: u32, value: u64) {
            let offset = 0x4000 + (hart * 8) as usize;
            // Write high word first to prevent spurious interrupt
            super::mmio::write32(self.base + offset + 4, u32::MAX);
            super::mmio::write32(self.base + offset, value as u32);
            super::mmio::write32(self.base + offset + 4, (value >> 32) as u32);
        }

        /// Get mtimecmp for a hart
        pub fn mtimecmp(&self, hart: u32) -> u64 {
            let offset = 0x4000 + (hart * 8) as usize;
            unsafe {
                let lo = super::mmio::read32(self.base + offset);
                let hi = super::mmio::read32(self.base + offset + 4);
                ((hi as u64) << 32) | (lo as u64)
            }
        }

        /// Trigger software interrupt for a hart
        pub unsafe fn trigger_soft_interrupt(&self, hart: u32) {
            let offset = (hart * 4) as usize;
            super::mmio::write32(self.base + offset, 1);
        }

        /// Clear software interrupt for a hart
        pub unsafe fn clear_soft_interrupt(&self, hart: u32) {
            let offset = (hart * 4) as usize;
            super::mmio::write32(self.base + offset, 0);
        }
    }
}

/// Delay using cycle counter
pub fn delay_cycles(cycles: u32) {
    let start = csr::cycle();
    while csr::cycle().wrapping_sub(start) < cycles {}
}

/// Delay in microseconds
pub fn delay_us(us: u32, cpu_freq_mhz: u32) {
    let cycles = us * cpu_freq_mhz;
    delay_cycles(cycles);
}
