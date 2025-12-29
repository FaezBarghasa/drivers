//! Native Graphics API Layer for Redox OS
//!
//! High-performance graphics API with Ray Tracing, AI upscaling, and Anti-Lag support.

pub mod latency;
pub mod shader;
pub mod upscaling;
pub mod vulkan;

pub use latency::*;
pub use shader::*;
pub use upscaling::*;
pub use vulkan::*;

/// Graphics API initialization
pub fn init() -> Result<(), &'static str> {
    log::info!("Initializing native graphics API...");

    // Initialize Vulkan with Ray Tracing extensions
    vulkan::init_vulkan()?;

    // Initialize upscaling subsystem
    upscaling::init_upscaling()?;

    // Initialize Anti-Lag
    latency::init_anti_lag()?;

    // Initialize shader cache
    shader::init_shader_cache()?;

    log::info!("Graphics API initialized successfully");
    Ok(())
}
