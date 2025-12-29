//! Frame pacing and synchronization

use crate::common::{LatencyError, LatencyStats};

/// Frame pacing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacingStrategy {
    /// VSync (wait for vertical blank)
    VSync,
    /// Immediate (no waiting)
    Immediate,
    /// Adaptive (dynamic based on load)
    Adaptive,
    /// Mailbox (replace queued frame)
    Mailbox,
}

/// Frame pacer
pub struct FramePacer {
    /// Pacing strategy
    strategy: PacingStrategy,
    /// Target frame time (microseconds)
    target_frame_time_us: u64,
    /// Maximum flip queue depth
    max_flip_queue_depth: u32,
    /// Current frame time (microseconds)
    current_frame_time_us: u64,
    /// Frame counter
    frame_count: u64,
}

impl FramePacer {
    /// Create new frame pacer
    pub fn new(strategy: PacingStrategy, target_fps: u32) -> Self {
        let target_frame_time_us = 1_000_000 / target_fps as u64;

        log::info!(
            "Creating frame pacer: {:?}, target: {} FPS ({} μs)",
            strategy,
            target_fps,
            target_frame_time_us
        );

        Self {
            strategy,
            target_frame_time_us,
            max_flip_queue_depth: 1, // Minimize latency
            current_frame_time_us: 0,
            frame_count: 0,
        }
    }

    /// Set pacing strategy
    pub fn set_strategy(&mut self, strategy: PacingStrategy) {
        log::info!(
            "Frame pacing strategy changed: {:?} -> {:?}",
            self.strategy,
            strategy
        );
        self.strategy = strategy;
    }

    /// Set target FPS
    pub fn set_target_fps(&mut self, fps: u32) {
        self.target_frame_time_us = 1_000_000 / fps as u64;
        log::debug!(
            "Target FPS set to {} ({} μs)",
            fps,
            self.target_frame_time_us
        );
    }

    /// Set maximum flip queue depth
    pub fn set_max_flip_queue_depth(&mut self, depth: u32) {
        self.max_flip_queue_depth = depth;
        log::debug!("Max flip queue depth set to {}", depth);
    }

    /// Begin frame
    pub fn begin_frame(&mut self) -> Result<(), LatencyError> {
        self.frame_count += 1;
        log::trace!("Frame {} begin", self.frame_count);
        Ok(())
    }

    /// End frame and pace
    pub fn end_frame(&mut self, frame_time_us: u64) -> Result<(), LatencyError> {
        self.current_frame_time_us = frame_time_us;

        match self.strategy {
            PacingStrategy::VSync => {
                // Wait for VBlank
                log::trace!("Frame {} end: VSync wait", self.frame_count);
            }
            PacingStrategy::Immediate => {
                // No waiting
                log::trace!("Frame {} end: Immediate present", self.frame_count);
            }
            PacingStrategy::Adaptive => {
                // Adaptive pacing based on frame time
                if frame_time_us < self.target_frame_time_us {
                    let wait_time = self.target_frame_time_us - frame_time_us;
                    log::trace!(
                        "Frame {} end: Adaptive wait {} μs",
                        self.frame_count,
                        wait_time
                    );
                }
            }
            PacingStrategy::Mailbox => {
                // Replace queued frame
                log::trace!("Frame {} end: Mailbox present", self.frame_count);
            }
        }

        Ok(())
    }

    /// Get current FPS
    pub fn current_fps(&self) -> f32 {
        if self.current_frame_time_us == 0 {
            0.0
        } else {
            1_000_000.0 / self.current_frame_time_us as f32
        }
    }

    /// Get frame statistics
    pub fn stats(&self) -> LatencyStats {
        let mut stats = LatencyStats::new();
        stats.render_latency_ms = self.current_frame_time_us as f32 / 1000.0;
        stats
    }
}

/// VSync mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VSyncMode {
    /// VSync off
    Off,
    /// VSync on
    On,
    /// Adaptive VSync (disable when below target FPS)
    Adaptive,
    /// Fast VSync (tear when above target FPS)
    Fast,
}
