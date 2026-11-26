#[cfg(test)]
mod tests {
    use crate::context::{Context, ContextParams};
    use core::sync::atomic::AtomicU64;

    #[test]
    fn test_submission_vram_bounds() {
        let ctx = Context {
            id: 1,
            params: ContextParams {
                core_mask: 0xF,
                priority: 0,
            },
            ring_buffer_head: AtomicU64::new(0),
            vram_start: 0,
            vram_size: 64 * 1024 * 1024,
        };

        // Within bounds
        assert!(ctx.validate_submission(0, 1024).is_ok());
        assert!(ctx
            .validate_submission(64 * 1024 * 1024 - 1024, 1024)
            .is_ok());

        // Out of bounds
        assert!(ctx.validate_submission(64 * 1024 * 1024, 1).is_err());
        assert!(ctx
            .validate_submission(64 * 1024 * 1024 - 512, 1024)
            .is_err());

        // Overflow checks
        assert!(ctx.validate_submission(u64::MAX - 10, 20).is_err());
    }
}
