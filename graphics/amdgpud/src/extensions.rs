//! GPU Extensions: RTX, DLSS, FSR, Anti-Lag
//!
//! Provides abstraction for GPU compute and upscaling features.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// GPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Amd,
    Nvidia,
    Intel,
    Unknown,
}

/// Ray tracing capability tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtTier {
    None,
    Tier1_0, // Basic RT (GTX 16xx, RX 6600)
    Tier1_1, // Enhanced RT (RTX 20xx, RX 6700+)
    Tier2_0, // Full RT (RTX 30xx, RX 7000)
}

/// Upscaling technology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpscaleTech {
    None,
    DlssBasic,  // NVIDIA DLSS 2.x
    DlssRayRec, // NVIDIA DLSS 3.x with Ray Reconstruction
    Fsr1,       // AMD FidelityFX Super Resolution 1.0
    Fsr2,       // AMD FSR 2.x (temporal)
    Fsr3,       // AMD FSR 3.x (frame generation)
    XeSS,       // Intel XeSS
}

/// Anti-lag technology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiLagTech {
    None,
    Reflex,   // NVIDIA Reflex
    AntiLag,  // AMD Anti-Lag
    AntiLag2, // AMD Anti-Lag 2
}

/// GPU capabilities structure
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    pub vendor: GpuVendor,
    pub device_name: String,
    pub vram_mb: u32,
    pub compute_units: u32,
    pub rt_tier: RtTier,
    pub upscaling: Vec<UpscaleTech>,
    pub anti_lag: AntiLagTech,
    pub tensor_cores: bool,
    pub matrix_cores: bool,
    pub hw_ray_tracing: bool,
}

impl GpuCapabilities {
    pub fn supports_dlss(&self) -> bool {
        self.upscaling
            .iter()
            .any(|u| matches!(u, UpscaleTech::DlssBasic | UpscaleTech::DlssRayRec))
    }

    pub fn supports_fsr(&self) -> bool {
        self.upscaling
            .iter()
            .any(|u| matches!(u, UpscaleTech::Fsr1 | UpscaleTech::Fsr2 | UpscaleTech::Fsr3))
    }

    pub fn supports_rt(&self) -> bool {
        self.rt_tier != RtTier::None
    }
}

/// DLSS configuration
#[derive(Debug, Clone, Copy)]
pub struct DlssConfig {
    pub quality: DlssQuality,
    pub sharpness: f32,
    pub ray_reconstruction: bool,
    pub frame_generation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlssQuality {
    UltraPerformance, // 66% scale
    Performance,      // 50% scale
    Balanced,         // 58% scale
    Quality,          // 67% scale
    UltraQuality,     // 77% scale
    Native,           // 100% (DLAA)
}

impl DlssQuality {
    pub fn scale_factor(&self) -> f32 {
        match self {
            Self::UltraPerformance => 0.33,
            Self::Performance => 0.50,
            Self::Balanced => 0.58,
            Self::Quality => 0.67,
            Self::UltraQuality => 0.77,
            Self::Native => 1.0,
        }
    }
}

/// FSR configuration
#[derive(Debug, Clone, Copy)]
pub struct FsrConfig {
    pub quality: FsrQuality,
    pub sharpness: f32,
    pub frame_generation: bool,
    pub fluid_motion_frames: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsrQuality {
    Performance,  // 50% scale
    Balanced,     // 59% scale
    Quality,      // 67% scale
    UltraQuality, // 77% scale
    Native,       // 100%
}

impl FsrQuality {
    pub fn scale_factor(&self) -> f32 {
        match self {
            Self::Performance => 0.50,
            Self::Balanced => 0.59,
            Self::Quality => 0.67,
            Self::UltraQuality => 0.77,
            Self::Native => 1.0,
        }
    }
}

/// Ray tracing settings
#[derive(Debug, Clone, Copy)]
pub struct RayTracingConfig {
    pub enabled: bool,
    pub quality: RtQuality,
    pub reflections: bool,
    pub shadows: bool,
    pub global_illumination: bool,
    pub ambient_occlusion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtQuality {
    Low,
    Medium,
    High,
    Ultra,
    Psycho,
}

/// Anti-lag marker
pub struct AntiLagMarker {
    frame_id: u64,
    timestamp: u64,
}

/// GPU extension interface
pub struct GpuExtensions {
    capabilities: GpuCapabilities,
    dlss_enabled: AtomicBool,
    fsr_enabled: AtomicBool,
    anti_lag_enabled: AtomicBool,
    frame_counter: AtomicU32,
}

impl GpuExtensions {
    pub fn new(capabilities: GpuCapabilities) -> Self {
        Self {
            capabilities,
            dlss_enabled: AtomicBool::new(false),
            fsr_enabled: AtomicBool::new(false),
            anti_lag_enabled: AtomicBool::new(false),
            frame_counter: AtomicU32::new(0),
        }
    }

    /// Initialize DLSS
    pub fn init_dlss(&self, config: DlssConfig) -> Result<(), GpuError> {
        if !self.capabilities.supports_dlss() {
            return Err(GpuError::NotSupported);
        }

        // TODO: Initialize DLSS library
        self.dlss_enabled.store(true, Ordering::Release);
        Ok(())
    }

    /// Initialize FSR
    pub fn init_fsr(&self, config: FsrConfig) -> Result<(), GpuError> {
        if !self.capabilities.supports_fsr() {
            return Err(GpuError::NotSupported);
        }

        // TODO: Initialize FSR compute shaders
        self.fsr_enabled.store(true, Ordering::Release);
        Ok(())
    }

    /// Enable Anti-Lag
    pub fn enable_anti_lag(&self) -> Result<(), GpuError> {
        if self.capabilities.anti_lag == AntiLagTech::None {
            return Err(GpuError::NotSupported);
        }

        self.anti_lag_enabled.store(true, Ordering::Release);
        Ok(())
    }

    /// Mark frame start for Anti-Lag
    pub fn anti_lag_frame_start(&self) -> AntiLagMarker {
        let frame_id = self.frame_counter.fetch_add(1, Ordering::Relaxed) as u64;
        AntiLagMarker {
            frame_id,
            timestamp: 0, // TODO: Get high-precision timestamp
        }
    }

    /// Get capabilities
    pub fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }
}

/// GPU extension errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuError {
    NotSupported,
    InitFailed,
    InvalidConfig,
    OutOfMemory,
    DeviceLost,
}
