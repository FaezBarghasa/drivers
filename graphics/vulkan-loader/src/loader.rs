//! Vulkan loader implementation

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use crate::icd::{IcdDriver, IcdManifest};
use crate::VulkanVersion;

/// Loader error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderError {
    /// No drivers found
    NoDriversFound,
    /// GAL scheme not available
    GalNotAvailable,
    /// Driver load failed
    DriverLoadFailed(String),
    /// Unsupported API version
    UnsupportedVersion,
}

impl fmt::Display for LoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoaderError::NoDriversFound => write!(f, "No Vulkan drivers found"),
            LoaderError::GalNotAvailable => write!(f, "GAL scheme not available"),
            LoaderError::DriverLoadFailed(msg) => write!(f, "Driver load failed: {}", msg),
            LoaderError::UnsupportedVersion => write!(f, "Unsupported API version"),
        }
    }
}

/// Main Vulkan loader
pub struct VulkanLoader {
    /// Loaded ICDs
    drivers: Vec<IcdDriver>,
    /// Enabled layers
    layers: Vec<String>,
}

impl VulkanLoader {
    /// Create a new loader
    pub fn new() -> Result<Self, LoaderError> {
        log::info!("Creating Vulkan loader");

        let drivers = discover_icds()?;
        let loaded_drivers: Vec<IcdDriver> = drivers
            .into_iter()
            .filter_map(|manifest| match IcdDriver::load(manifest) {
                Ok(driver) => Some(driver),
                Err(e) => {
                    log::warn!("Failed to load driver: {}", e);
                    None
                }
            })
            .collect();

        if loaded_drivers.is_empty() {
            return Err(LoaderError::NoDriversFound);
        }

        log::info!("Loaded {} Vulkan driver(s)", loaded_drivers.len());

        Ok(Self {
            drivers: loaded_drivers,
            layers: Vec::new(),
        })
    }

    /// Get available drivers
    pub fn drivers(&self) -> &[IcdDriver] {
        &self.drivers
    }

    /// Enable a validation layer
    pub fn enable_layer(&mut self, layer_name: impl Into<String>) {
        self.layers.push(layer_name.into());
    }

    /// Get instance proc address
    pub fn get_instance_proc_addr(&self, name: &str) -> Option<usize> {
        // Try each driver
        for driver in &self.drivers {
            if let Some(addr) = driver.get_instance_proc_addr(name) {
                return Some(addr);
            }
        }
        None
    }
}

impl Default for VulkanLoader {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            drivers: Vec::new(),
            layers: Vec::new(),
        })
    }
}

/// Discover available Vulkan ICDs via GAL
pub fn discover_icds() -> Result<Vec<IcdManifest>, LoaderError> {
    log::info!("Discovering Vulkan ICDs via /scheme/gal");

    // Check if GAL is available
    #[cfg(target_os = "redox")]
    {
        use libredox::flag;

        if let Ok(_gal_fd) = libredox::call::open("/scheme/gal", flag::O_RDONLY, 0) {
            log::info!("GAL scheme available");
        } else {
            log::warn!("GAL scheme not available");
            return Err(LoaderError::GalNotAvailable);
        }
    }

    let mut manifests = Vec::new();

    // VirtIO-GPU driver (always available in QEMU)
    let mut virtio_manifest = IcdManifest::new("virtio-gpu", VulkanVersion::VK_1_2);
    virtio_manifest.library_path = "/scheme/gal/virtio".into();
    virtio_manifest.extensions = vec!["VK_KHR_surface".into(), "VK_KHR_swapchain".into()];
    manifests.push(virtio_manifest);

    // AMD GPU driver
    #[cfg(feature = "amdgpu")]
    {
        let mut amd_manifest = IcdManifest::new("amdgpu", VulkanVersion::VK_1_3);
        amd_manifest.library_path = "/scheme/gal/amdgpu".into();
        amd_manifest.extensions = vec![
            "VK_KHR_surface".into(),
            "VK_KHR_swapchain".into(),
            "VK_KHR_ray_tracing_pipeline".into(),
            "VK_KHR_acceleration_structure".into(),
            "VK_KHR_ray_query".into(),
        ];
        manifests.push(amd_manifest);
    }

    // NVIDIA GPU driver
    #[cfg(feature = "nvidia")]
    {
        let mut nvidia_manifest = IcdManifest::new("nvidia", VulkanVersion::VK_1_3);
        nvidia_manifest.library_path = "/scheme/gal/nvidia".into();
        nvidia_manifest.extensions = vec![
            "VK_KHR_surface".into(),
            "VK_KHR_swapchain".into(),
            "VK_KHR_ray_tracing_pipeline".into(),
            "VK_KHR_acceleration_structure".into(),
            "VK_KHR_ray_query".into(),
            "VK_NV_ray_tracing".into(),
        ];
        manifests.push(nvidia_manifest);
    }

    if manifests.is_empty() {
        return Err(LoaderError::NoDriversFound);
    }

    log::info!("Discovered {} ICD(s)", manifests.len());
    Ok(manifests)
}
