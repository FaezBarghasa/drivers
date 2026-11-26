//! Latency boost mode (generic optimizations)

use crate::common::{LatencyError, LatencyMode};

/// Boost context
pub struct BoostContext {
    /// Current mode
    mode: LatencyMode,
    /// CPU priority boost enabled
    cpu_priority_boost: bool,
    /// GPU clock boost enabled
    gpu_clock_boost: bool,
}

impl BoostContext {
    /// Create boost context
    pub fn new() -> Result<Self, LatencyError> {
        log::info!("Creating latency boost context");

        Ok(Self {
            mode: LatencyMode::Off,
            cpu_priority_boost: false,
            gpu_clock_boost: false,
        })
    }

    /// Set boost mode
    pub fn set_mode(&mut self, mode: LatencyMode) -> Result<(), LatencyError> {
        log::info!("Latency boost mode: {:?} -> {:?}", self.mode, mode);
        self.mode = mode;

        match mode {
            LatencyMode::Off => {
                self.cpu_priority_boost = false;
                self.gpu_clock_boost = false;
                log::debug!("All boost features disabled");
            }
            LatencyMode::On => {
                self.cpu_priority_boost = true;
                self.gpu_clock_boost = false;
                log::debug!("CPU priority boost enabled");
            }
            LatencyMode::Boost => {
                self.cpu_priority_boost = true;
                self.gpu_clock_boost = true;
                log::debug!("CPU priority + GPU clock boost enabled");
            }
        }

        Ok(())
    }

    /// Enable CPU priority boost
    pub fn set_cpu_priority_boost(&mut self, enabled: bool) {
        self.cpu_priority_boost = enabled;
        log::debug!(
            "CPU priority boost: {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    /// Enable GPU clock boost
    pub fn set_gpu_clock_boost(&mut self, enabled: bool) {
        self.gpu_clock_boost = enabled;
        log::debug!(
            "GPU clock boost: {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }
}

impl Default for BoostContext {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            mode: LatencyMode::Off,
            cpu_priority_boost: false,
            gpu_clock_boost: false,
        })
    }
}
