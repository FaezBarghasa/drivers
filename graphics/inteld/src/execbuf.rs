use crate::context::Context;
use crate::device::IntelDevice;
use std::sync::Arc;

pub struct ExecBuffer {
    pub batch_start_offset: u64,
    pub batch_len: u64,
}

pub fn submit(
    context: &Arc<Context>,
    device: &Arc<IntelDevice>,
    _execbuf: &ExecBuffer,
) -> Result<(), &'static str> {
    if context.params.core_mask == 0 {
        return Err("No GPU cores allocated to this context");
    }

    context.apply_mask(device);

    log::trace!(
        "Submitting batch for Context {} (Mask: {:x})",
        context.id,
        context.params.core_mask
    );

    Ok(())
}
