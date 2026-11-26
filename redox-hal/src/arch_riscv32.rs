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

/// Physical Memory Protection (PMP) implementation
pub mod pmp {
    use core::arch::asm;

    /// PMP configuration byte fields
    pub mod cfg {
        pub const R: u8 = 1 << 0;
        pub const W: u8 = 1 << 1;
        pub const X: u8 = 1 << 2;
        pub const A_OFF: u8 = 0 << 3;
        pub const A_TOR: u8 = 1 << 3;
        pub const A_NA4: u8 = 2 << 3;
        pub const A_NAPOT: u8 = 3 << 3;
        pub const L: u8 = 1 << 7;
    }

    /// Write pmpcfg CSRs
    ///
    /// pmpcfg0 controls pmp0..3
    /// pmpcfg2 controls pmp4..7 (on RV32)
    /// etc.
    #[inline]
    pub unsafe fn write_pmpcfg(idx: usize, val: u32) {
        match idx {
            0 => asm!("csrw pmpcfg0, {}", in(reg) val),
            1 => asm!("csrw pmpcfg1, {}", in(reg) val), // RV32: pmp4..7
            2 => asm!("csrw pmpcfg2, {}", in(reg) val), // RV32: pmp8..11
            3 => asm!("csrw pmpcfg3, {}", in(reg) val), // RV32: pmp12..15
            _ => panic!("invalid pmpcfg index"),
        }
    }

    /// Write pmpaddr CSR
    #[inline]
    pub unsafe fn write_pmpaddr(idx: usize, val: u32) {
        // CSR indices for pmpaddr are 0x3B0 + idx
        // We can't use variable CSR in asm with immediate.
        // Needs a match.
        match idx {
            0 => asm!("csrw pmpaddr0, {}", in(reg) val),
            1 => asm!("csrw pmpaddr1, {}", in(reg) val),
            2 => asm!("csrw pmpaddr2, {}", in(reg) val),
            3 => asm!("csrw pmpaddr3, {}", in(reg) val),
            4 => asm!("csrw pmpaddr4, {}", in(reg) val),
            5 => asm!("csrw pmpaddr5, {}", in(reg) val),
            6 => asm!("csrw pmpaddr6, {}", in(reg) val),
            7 => asm!("csrw pmpaddr7, {}", in(reg) val),
            8 => asm!("csrw pmpaddr8, {}", in(reg) val),
            9 => asm!("csrw pmpaddr9, {}", in(reg) val),
            10 => asm!("csrw pmpaddr10, {}", in(reg) val),
            11 => asm!("csrw pmpaddr11, {}", in(reg) val),
            12 => asm!("csrw pmpaddr12, {}", in(reg) val),
            13 => asm!("csrw pmpaddr13, {}", in(reg) val),
            14 => asm!("csrw pmpaddr14, {}", in(reg) val),
            15 => asm!("csrw pmpaddr15, {}", in(reg) val),
            _ => panic!("invalid pmpaddr index"),
        }
    }

    /// Calculate NAPOT address encoding
    /// Base must be aligned to size, size must be power of 2 >= 4.
    pub fn napot_encode(base: u32, size: u32) -> u32 {
        // NAPOT:
        // Bits are 1 until LSB 0.
        // e.g. 8 bytes: yyyy...y011
        // encoded = (base >> 2) | ((size/2 - 1) >> 1) ?
        // Spec:
        // NAPOT range size 2^(i+3) bytes
        // pmpaddr = (base >> 2) | (01...1) with i ones.
        //
        // Example size 4KB = 2^12. i=9.
        // pmpaddr = (base >> 2) | 0x1FF (9 ones).
        //
        // bitmask = (size >> 3) - 1.
        // pmpaddr = (base >> 2) | bitmask.
        if size < 4 {
            return 0;
        }
        let bitmask = (size >> 3) - 1;
        (base >> 2) | bitmask
    }

    /// Region definition for PMP
    #[derive(Debug, Clone, Copy)]
    pub struct PmpRegion {
        pub base: u32,
        pub size: u32,
        pub r: bool,
        pub w: bool,
        pub x: bool,
        pub active: bool,
    }

    /// Configure PMP with provided zones
    pub fn configure(regions: &[PmpRegion]) {
        // We assume zones map to PMP entries 0..N.
        // PMP priority is lowest index first (on match, use that rule).
        // Or highest? "Priority is highest number" if implemented?
        // Spec: "Matching PMP entry with lowest index takes priority."
        // except if locked?
        // Wait, "The highest numbered PMP entry that matches ... determines permissions."?
        // NO. "The lowest-numbered PMP entry that matches any byte of the access determines the permissions." (Privileged Spec).

        let mut pmpcfg = [0u8; 16];
        let mut pmpaddr = [0u32; 16];

        for (i, zone) in regions.iter().enumerate() {
            if i >= 16 {
                break;
            }
            if !zone.active {
                continue;
            }

            // Convert permissions
            let mut cfg = cfg::A_NAPOT; // Use NAPOT for simplicity (requires alignment)
                                        // TODO: Fallback to TOR if not aligned?
                                        // User requirement: "Flat memory model must correctly translate".
                                        // We assume aligned zones.

            if zone.r {
                cfg |= cfg::R;
            }
            if zone.w {
                cfg |= cfg::W;
            }
            if zone.x {
                cfg |= cfg::X;
            }

            pmpcfg[i] = cfg;
            pmpaddr[i] = napot_encode(zone.base, zone.size);
        }

        // Write to CSRs
        // Packing 4 configs into u32
        for i in 0..4 {
            let mut val = 0u32;
            val |= (pmpcfg[i * 4 + 0] as u32) << 0;
            val |= (pmpcfg[i * 4 + 1] as u32) << 8;
            val |= (pmpcfg[i * 4 + 2] as u32) << 16;
            val |= (pmpcfg[i * 4 + 3] as u32) << 24;
            unsafe {
                write_pmpcfg(i, val);
            }
        }

        for i in 0..16 {
            unsafe {
                write_pmpaddr(i, pmpaddr[i]);
            }
        }
    }
}
