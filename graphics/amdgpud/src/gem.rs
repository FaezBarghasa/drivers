//! GEM (Graphics Execution Manager) Memory Manager for AMD GPUs

use bitflags::bitflags;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// GEM buffer object
pub struct GemObject {
    /// Unique handle
    pub handle: u32,
    /// Size in bytes
    pub size: usize,
    /// Physical address (GPU)
    pub gpu_addr: u64,
    /// CPU mapping address
    pub cpu_addr: Option<usize>,
    /// Flags
    pub flags: GemFlags,
    /// Reference count
    pub refcount: u32,
}

bitflags! {
    /// GEM buffer flags
    pub struct GemFlags: u32 {
        /// Buffer is in VRAM
        const VRAM = 1 << 0;
        /// Buffer is in GTT (system memory)
        const GTT = 1 << 1;
        /// Buffer is CPU accessible
        const CPU_ACCESS = 1 << 2;
        /// Buffer is GPU accessible
        const GPU_ACCESS = 1 << 3;
        /// Buffer is shareable
        const SHAREABLE = 1 << 4;
        /// Buffer is tiled
        const TILED = 1 << 5;
    }
}

/// GEM memory manager
pub struct GemManager {
    /// Buffer objects
    objects: Mutex<HashMap<u32, Arc<GemObject>>>,
    /// Next handle
    next_handle: Mutex<u32>,
    /// VRAM allocator
    vram_allocator: Mutex<VramAllocator>,
    /// GTT allocator
    gtt_allocator: Mutex<GttAllocator>,
}

impl GemManager {
    /// Create new GEM manager
    pub fn new(vram_size: u64, gtt_size: u64) -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            next_handle: Mutex::new(1),
            vram_allocator: Mutex::new(VramAllocator::new(vram_size)),
            gtt_allocator: Mutex::new(GttAllocator::new(gtt_size)),
        }
    }

    /// Allocate GEM object
    pub fn alloc(&self, size: usize, flags: GemFlags) -> Result<u32, &'static str> {
        let handle = {
            let mut next = self.next_handle.lock().unwrap();
            let h = *next;
            *next += 1;
            h
        };

        // Allocate from VRAM or GTT
        let gpu_addr = if flags.contains(GemFlags::VRAM) {
            self.vram_allocator.lock().unwrap().alloc(size)?
        } else {
            self.gtt_allocator.lock().unwrap().alloc(size)?
        };

        let obj = Arc::new(GemObject {
            handle,
            size,
            gpu_addr,
            cpu_addr: None,
            flags,
            refcount: 1,
        });

        self.objects.lock().unwrap().insert(handle, obj);

        Ok(handle)
    }

    /// Free GEM object
    pub fn free(&self, handle: u32) -> Result<(), &'static str> {
        let obj = self
            .objects
            .lock()
            .unwrap()
            .remove(&handle)
            .ok_or("Invalid handle")?;

        // Free from allocator
        if obj.flags.contains(GemFlags::VRAM) {
            self.vram_allocator
                .lock()
                .unwrap()
                .free(obj.gpu_addr, obj.size);
        } else {
            self.gtt_allocator
                .lock()
                .unwrap()
                .free(obj.gpu_addr, obj.size);
        }

        Ok(())
    }

    /// Get GEM object
    pub fn get(&self, handle: u32) -> Option<Arc<GemObject>> {
        self.objects.lock().unwrap().get(&handle).cloned()
    }

    /// Map GEM object to CPU
    pub fn map(&self, handle: u32) -> Result<usize, &'static str> {
        let mut objects = self.objects.lock().unwrap();
        let obj = objects.get_mut(&handle).ok_or("Invalid handle")?;

        // TODO: Actual CPU mapping via BAR
        let cpu_addr = obj.gpu_addr as usize;
        Arc::get_mut(obj).unwrap().cpu_addr = Some(cpu_addr);

        Ok(cpu_addr)
    }
}

/// VRAM allocator (simple bump allocator)
struct VramAllocator {
    base: u64,
    size: u64,
    next: u64,
}

impl VramAllocator {
    fn new(size: u64) -> Self {
        Self {
            base: 0,
            size,
            next: 0,
        }
    }

    fn alloc(&mut self, size: usize) -> Result<u64, &'static str> {
        let aligned_size = (size + 4095) & !4095; // 4KB align

        if self.next + aligned_size as u64 > self.size {
            return Err("Out of VRAM");
        }

        let addr = self.base + self.next;
        self.next += aligned_size as u64;

        Ok(addr)
    }

    fn free(&mut self, _addr: u64, _size: usize) {
        // Simple allocator doesn't support free
    }
}

/// GTT allocator
struct GttAllocator {
    base: u64,
    size: u64,
    next: u64,
}

impl GttAllocator {
    fn new(size: u64) -> Self {
        Self {
            base: 0,
            size,
            next: 0,
        }
    }

    fn alloc(&mut self, size: usize) -> Result<u64, &'static str> {
        let aligned_size = (size + 4095) & !4095;

        if self.next + aligned_size as u64 > self.size {
            return Err("Out of GTT");
        }

        let addr = self.base + self.next;
        self.next += aligned_size as u64;

        Ok(addr)
    }

    fn free(&mut self, _addr: u64, _size: usize) {
        // Simple allocator doesn't support free
    }
}
