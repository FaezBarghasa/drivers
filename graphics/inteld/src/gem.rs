//! GEM (Graphics Execution Manager) for Intel GPUs

use bitflags::bitflags;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// GEM object
pub struct GemObject {
    pub handle: u32,
    pub size: usize,
    pub gtt_offset: u64,
    pub cpu_addr: Option<usize>,
    pub flags: GemFlags,
    pub tiling: TilingMode,
}

bitflags! {
    pub struct GemFlags: u32 {
        const STOLEN = 1 << 0;
        const CPU_ACCESS = 1 << 1;
        const GPU_ACCESS = 1 << 2;
        const SHAREABLE = 1 << 3;
        const PURGEABLE = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TilingMode {
    None,
    X,
    Y,
}

pub struct GemManager {
    objects: Mutex<HashMap<u32, Arc<GemObject>>>,
    next_handle: Mutex<u32>,
    gtt_allocator: Mutex<GttAllocator>,
}

impl GemManager {
    pub fn new(gtt_size: u64) -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            next_handle: Mutex::new(1),
            gtt_allocator: Mutex::new(GttAllocator::new(gtt_size)),
        }
    }

    pub fn alloc(&self, size: usize, flags: GemFlags) -> Result<u32, &'static str> {
        let handle = {
            let mut next = self.next_handle.lock().unwrap();
            let h = *next;
            *next += 1;
            h
        };

        let gtt_offset = self.gtt_allocator.lock().unwrap().alloc(size)?;

        let obj = Arc::new(GemObject {
            handle,
            size,
            gtt_offset,
            cpu_addr: None,
            flags,
            tiling: TilingMode::None,
        });

        self.objects.lock().unwrap().insert(handle, obj);

        Ok(handle)
    }

    pub fn free(&self, handle: u32) -> Result<(), &'static str> {
        let obj = self
            .objects
            .lock()
            .unwrap()
            .remove(&handle)
            .ok_or("Invalid handle")?;

        self.gtt_allocator
            .lock()
            .unwrap()
            .free(obj.gtt_offset, obj.size);

        Ok(())
    }

    pub fn get(&self, handle: u32) -> Option<Arc<GemObject>> {
        self.objects.lock().unwrap().get(&handle).cloned()
    }
}

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
            return Err("Out of GTT space");
        }

        let offset = self.base + self.next;
        self.next += aligned_size as u64;

        Ok(offset)
    }

    fn free(&mut self, _offset: u64, _size: usize) {}
}
