//! Common upscaling types and utilities

use alloc::string::String;
use core::fmt;

/// Upscaling backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpscalingBackend {
    /// AMD FidelityFX Super Resolution
    FSR,
    /// NVIDIA Deep Learning Super Sampling
    DLSS,
    /// Intel Xe Super Sampling
    XeSS,
}

impl fmt::Display for UpscalingBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpscalingBackend::FSR => write!(f, "AMD FSR"),
            UpscalingBackend::DLSS => write!(f, "NVIDIA DLSS"),
            UpscalingBackend::XeSS => write!(f, "Intel XeSS"),
        }
    }
}

/// Upscaling quality preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpscalingQuality {
    /// Ultra Performance (3x upscale)
    UltraPerformance,
    /// Performance (2x upscale)
    Performance,
    /// Balanced (1.7x upscale)
    Balanced,
    /// Quality (1.5x upscale)
    Quality,
    /// Ultra Quality (1.3x upscale)
    UltraQuality,
}

impl UpscalingQuality {
    /// Get the upscale factor for this quality preset
    pub fn scale_factor(&self) -> f32 {
        match self {
            UpscalingQuality::UltraPerformance => 3.0,
            UpscalingQuality::Performance => 2.0,
            UpscalingQuality::Balanced => 1.7,
            UpscalingQuality::Quality => 1.5,
            UpscalingQuality::UltraQuality => 1.3,
        }
    }

    /// Calculate render resolution from display resolution
    pub fn render_resolution(&self, display_width: u32, display_height: u32) -> (u32, u32) {
        let factor = self.scale_factor();
        (
            (display_width as f32 / factor) as u32,
            (display_height as f32 / factor) as u32,
        )
    }
}

/// Upscaling error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpscalingError {
    /// Backend not available
    BackendNotAvailable,
    /// Initialization failed
    InitializationFailed(String),
    /// Invalid parameters
    InvalidParameters,
    /// Resource creation failed
    ResourceCreationFailed(String),
    /// Upscaling failed
    UpscalingFailed(String),
}

impl fmt::Display for UpscalingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpscalingError::BackendNotAvailable => write!(f, "Backend not available"),
            UpscalingError::InitializationFailed(msg) => {
                write!(f, "Initialization failed: {}", msg)
            }
            UpscalingError::InvalidParameters => write!(f, "Invalid parameters"),
            UpscalingError::ResourceCreationFailed(msg) => {
                write!(f, "Resource creation failed: {}", msg)
            }
            UpscalingError::UpscalingFailed(msg) => write!(f, "Upscaling failed: {}", msg),
        }
    }
}

/// Upscaling context
pub struct UpscalingContext {
    /// Backend in use
    pub backend: UpscalingBackend,
    /// Quality preset
    pub quality: UpscalingQuality,
    /// Display resolution
    pub display_resolution: (u32, u32),
    /// Render resolution
    pub render_resolution: (u32, u32),
    /// Sharpening amount (0.0 - 1.0)
    pub sharpening: f32,
}

impl UpscalingContext {
    /// Create a new upscaling context
    pub fn new(
        backend: UpscalingBackend,
        quality: UpscalingQuality,
        display_width: u32,
        display_height: u32,
    ) -> Result<Self, UpscalingError> {
        let render_resolution = quality.render_resolution(display_width, display_height);

        log::info!(
            "Creating {} upscaling context: {}x{} -> {}x{} ({})",
            backend,
            render_resolution.0,
            render_resolution.1,
            display_width,
            display_height,
            match quality {
                UpscalingQuality::UltraPerformance => "Ultra Performance",
                UpscalingQuality::Performance => "Performance",
                UpscalingQuality::Balanced => "Balanced",
                UpscalingQuality::Quality => "Quality",
                UpscalingQuality::UltraQuality => "Ultra Quality",
            }
        );

        Ok(Self {
            backend,
            quality,
            display_resolution: (display_width, display_height),
            render_resolution,
            sharpening: 0.5,
        })
    }

    /// Set sharpening amount
    pub fn set_sharpening(&mut self, amount: f32) {
        self.sharpening = amount.clamp(0.0, 1.0);
    }
}
