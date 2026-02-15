use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct ContextParams {
    pub core_mask: u64,
    pub priority: u8,
}

impl Default for ContextParams {
    fn default() -> Self {
        Self {
            core_mask: 0xFFFF_FFFF_FFFF_FFFF, // All cores
            priority: 0,
        }
    }
}

pub struct Context {
    pub id: u32,
    pub params: ContextParams,
    pub ring_buffer_head: AtomicU64,
}

impl Context {
    pub fn new(id: u32, params: ContextParams) -> Self {
        Self {
            id,
            params,
            ring_buffer_head: AtomicU64::new(0),
        }
    }

    pub fn apply_mask(&self) {
        log::debug!(
            "Context {}: Applying core mask {:x}",
            self.id,
            self.params.core_mask
        );
        // In real HW, write to register (e.g., EXECLIST_SUBMIT_PORT or GUC_CONTEXT_POLICY)
    }
}
