//! Minimal runtime for embedded Redox OS
//!
//! Provides the essential runtime support for embedded systems.

use core::panic::PanicInfo;

/// Boot information passed from bootloader
#[derive(Debug, Clone)]
pub struct BootInfo {
    /// RAM start address
    pub ram_start: usize,
    /// RAM size in bytes
    pub ram_size: usize,
    /// Device tree address (for ARM/RISC-V)
    pub dtb_address: Option<usize>,
    /// Command line
    pub cmdline: Option<&'static str>,
    /// Boot time (if RTC available)
    pub boot_time: Option<u64>,
}

impl Default for BootInfo {
    fn default() -> Self {
        Self {
            ram_start: 0,
            ram_size: 0,
            dtb_address: None,
            cmdline: None,
            boot_time: None,
        }
    }
}

/// Simple heap allocator for embedded systems
pub mod heap {
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::UnsafeCell;
    use core::ptr::null_mut;

    /// Bump allocator for simple heap allocation
    pub struct BumpAllocator {
        heap_start: UnsafeCell<usize>,
        heap_end: usize,
        next: UnsafeCell<usize>,
    }

    unsafe impl Sync for BumpAllocator {}

    impl BumpAllocator {
        /// Create a new uninitialized allocator
        pub const fn new() -> Self {
            Self {
                heap_start: UnsafeCell::new(0),
                heap_end: 0,
                next: UnsafeCell::new(0),
            }
        }

        /// Initialize the allocator with a memory region
        ///
        /// # Safety
        ///
        /// The caller must ensure the memory region is valid and not used elsewhere.
        pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
            *self.heap_start.get() = heap_start;
            *((&self.heap_end) as *const usize as *mut usize) = heap_start + heap_size;
            *self.next.get() = heap_start;
        }

        /// Get the amount of heap used
        pub fn used(&self) -> usize {
            unsafe { *self.next.get() - *self.heap_start.get() }
        }

        /// Get the amount of heap free
        pub fn free(&self) -> usize {
            unsafe { self.heap_end - *self.next.get() }
        }
    }

    unsafe impl GlobalAlloc for BumpAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let next = self.next.get();
            let alloc_start = (*next + layout.align() - 1) & !(layout.align() - 1);
            let alloc_end = alloc_start.saturating_add(layout.size());

            if alloc_end > self.heap_end {
                return null_mut();
            }

            *next = alloc_end;
            alloc_start as *mut u8
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
            // Bump allocator doesn't support deallocation
        }
    }
}

/// Panic handler for embedded systems
#[cfg(feature = "panic-handler")]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print panic message if console is available
    #[cfg(feature = "uart")]
    {
        use core::fmt::Write;
        if let Some(console) = crate::drivers::uart::console() {
            let _ = writeln!(console, "\r\n!!! PANIC !!!");
            let _ = writeln!(console, "{}", info);
        }
    }

    // Halt the CPU
    loop {
        #[cfg(target_arch = "arm")]
        unsafe {
            core::arch::asm!("wfi");
        }
        #[cfg(target_arch = "riscv32")]
        unsafe {
            core::arch::asm!("wfi");
        }
        #[cfg(not(any(target_arch = "arm", target_arch = "riscv32")))]
        core::hint::spin_loop();
    }
}

/// System initialization
pub fn init_system(boot_info: &BootInfo) {
    // Initialize heap
    if boot_info.ram_size > 0 {
        // Reserve first 64KB for stack, use rest for heap
        let heap_start = boot_info.ram_start + 64 * 1024;
        let heap_size = boot_info.ram_size - 64 * 1024;

        // Would initialize global allocator here
        let _ = (heap_start, heap_size);
    }
}

/// System reset
pub fn system_reset() -> ! {
    #[cfg(feature = "armv7")]
    unsafe {
        // ARM: Write to AIRCR to trigger reset
        let aircr = 0xE000_ED0C as *mut u32;
        core::ptr::write_volatile(aircr, 0x05FA_0004);
    }

    #[cfg(feature = "riscv32")]
    unsafe {
        // RISC-V: Use WFI in a loop (no standard reset mechanism)
        loop {
            core::arch::asm!("wfi");
        }
    }

    #[cfg(not(any(feature = "armv7", feature = "riscv32")))]
    loop {
        core::hint::spin_loop();
    }
}

/// Power off (if supported)
pub fn power_off() -> ! {
    // Platform-specific power off
    // Most embedded systems don't support this
    loop {
        #[cfg(any(feature = "armv7", feature = "riscv32"))]
        unsafe {
            core::arch::asm!("wfi");
        }
        #[cfg(not(any(feature = "armv7", feature = "riscv32")))]
        core::hint::spin_loop();
    }
}

/// Enter low-power sleep mode
pub fn sleep() {
    #[cfg(feature = "armv7")]
    unsafe {
        core::arch::asm!("wfi");
    }

    #[cfg(feature = "riscv32")]
    unsafe {
        core::arch::asm!("wfi");
    }
}

/// Get system uptime in milliseconds
#[cfg(feature = "timer")]
pub fn uptime_ms() -> u64 {
    // Would use system timer
    0
}

/// CPU usage percentage (0-100)
pub fn cpu_usage() -> u8 {
    // Would track idle time
    0
}

/// Memory usage in bytes
pub fn memory_usage() -> usize {
    // Would report heap usage
    0
}
