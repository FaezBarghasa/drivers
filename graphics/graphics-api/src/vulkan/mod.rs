//! Vulkan module

pub mod ray_tracing;

pub use ray_tracing::*;

pub fn init_vulkan() -> Result<(), &'static str> {
    ray_tracing::init_vulkan()
}
