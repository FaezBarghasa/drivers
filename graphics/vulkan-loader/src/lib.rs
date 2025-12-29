//! Vulkan ICD (Installable Client Driver) Loader for Redox OS
//!
//! This loader discovers and loads Vulkan drivers, providing a unified
//! Vulkan API surface for applications.

#![no_std]

extern crate alloc;

pub mod extensions;
pub mod icd;
pub mod loader;

pub use extensions::{Extension, RayTracingExtensions};
pub use icd::{IcdDriver, IcdManifest};
pub use loader::{LoaderError, VulkanLoader};

use alloc::string::String;
use alloc::vec::Vec;

/// Vulkan API version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VulkanVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl VulkanVersion {
    pub const VK_1_0: Self = Self {
        major: 1,
        minor: 0,
        patch: 0,
    };
    pub const VK_1_1: Self = Self {
        major: 1,
        minor: 1,
        patch: 0,
    };
    pub const VK_1_2: Self = Self {
        major: 1,
        minor: 2,
        patch: 0,
    };
    pub const VK_1_3: Self = Self {
        major: 1,
        minor: 3,
        patch: 0,
    };

    pub fn to_u32(self) -> u32 {
        (self.major << 22) | (self.minor << 12) | self.patch
    }

    pub fn from_u32(version: u32) -> Self {
        Self {
            major: version >> 22,
            minor: (version >> 12) & 0x3FF,
            patch: version & 0xFFF,
        }
    }
}

/// Initialize the Vulkan loader
pub fn init() -> Result<(), &'static str> {
    log::info!("Initializing Vulkan loader");

    // Check for GAL scheme (would use libredox in real implementation)
    #[cfg(target_os = "redox")]
    {
        use libredox::flag;
        if libredox::call::open("/scheme/gal", flag::O_RDONLY, 0).is_err() {
            log::warn!("GAL scheme not available");
            return Err("GAL scheme not found");
        }
    }

    log::info!("Vulkan loader initialized");
    Ok(())
}

/// Enumerate available Vulkan drivers
pub fn enumerate_drivers() -> Result<Vec<IcdManifest>, LoaderError> {
    loader::discover_icds()
}
