//! Common latency reduction types

use core::fmt;

/// Latency reduction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyMode {
    /// Disabled
    Off,
    /// Enabled (standard latency reduction)
    On,
    /// Boost mode (maximum latency reduction)
    Boost,
}

/// Latency statistics
#[derive(Debug, Clone, Copy)]
pub struct LatencyStats {
    /// Input-to-present latency (milliseconds)
    pub input_latency_ms: f32,
    /// Render latency (milliseconds)
    pub render_latency_ms: f32,
    /// Present latency (milliseconds)
    pub present_latency_ms: f32,
    /// Driver latency (milliseconds)
    pub driver_latency_ms: f32,
    /// OS queue latency (milliseconds)
    pub os_queue_latency_ms: f32,
    /// GPU render time (milliseconds)
    pub gpu_render_ms: f32,
}

impl LatencyStats {
    /// Create new latency stats
    pub fn new() -> Self {
        Self {
            input_latency_ms: 0.0,
            render_latency_ms: 0.0,
            present_latency_ms: 0.0,
            driver_latency_ms: 0.0,
            os_queue_latency_ms: 0.0,
            gpu_render_ms: 0.0,
        }
    }

    /// Get total end-to-end latency
    pub fn total_latency_ms(&self) -> f32 {
        self.input_latency_ms
    }

    /// Get PC latency (render + present + driver + OS)
    pub fn pc_latency_ms(&self) -> f32 {
        self.render_latency_ms
            + self.present_latency_ms
            + self.driver_latency_ms
            + self.os_queue_latency_ms
    }
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Latency error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatencyError {
    /// Feature not supported
    NotSupported,
    /// Initialization failed
    InitializationFailed,
    /// Invalid mode
    InvalidMode,
    /// Measurement failed
    MeasurementFailed,
}

impl fmt::Display for LatencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatencyError::NotSupported => write!(f, "Feature not supported"),
            LatencyError::InitializationFailed => write!(f, "Initialization failed"),
            LatencyError::InvalidMode => write!(f, "Invalid mode"),
            LatencyError::MeasurementFailed => write!(f, "Measurement failed"),
        }
    }
}

/// Latency marker for profiling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyMarker {
    /// Simulation start
    SimulationStart,
    /// Simulation end
    SimulationEnd,
    /// Render submit start
    RenderSubmitStart,
    /// Render submit end
    RenderSubmitEnd,
    /// Present start
    PresentStart,
    /// Present end
    PresentEnd,
    /// Input sample
    InputSample,
}
