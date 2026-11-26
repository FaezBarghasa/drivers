use std::sync::atomic::AtomicU64;

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
    pub vram_start: u64,
    pub vram_size: u64,
}

impl Context {
    pub fn new(id: u32, params: ContextParams, vram_start: u64, vram_size: u64) -> Self {
        Self {
            id,
            params,
            ring_buffer_head: AtomicU64::new(0),
            vram_start,
            vram_size,
        }
    }

    pub fn apply_mask(&self, device: &crate::device::IntelDevice) {
        log::debug!(
            "Context {}: Applying core mask {:x}",
            self.id,
            self.params.core_mask
        );
        // Write to GUC_CONTEXT_POLICY (placeholder 0x24C0)
        // We write the lower 32 bits of the mask to the register
        device.mmio.write32(0x24C0, self.params.core_mask as u32);
    }

    pub fn validate_submission(
        &self,
        batch_gtt_offset: u64,
        batch_len: u64,
    ) -> Result<(), &'static str> {
        let batch_end = batch_gtt_offset.checked_add(batch_len).ok_or("Overflow")?;
        let vram_end = self
            .vram_start
            .checked_add(self.vram_size)
            .ok_or("Overflow")?;

        if batch_gtt_offset >= self.vram_start && batch_end <= vram_end {
            Ok(())
        } else {
            Err("Out of VRAM bounds")
        }
    }
}
