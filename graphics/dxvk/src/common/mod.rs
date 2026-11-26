//! Common DXVK types and utilities

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// DXVK error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxvkError {
    /// Vulkan initialization failed
    VulkanInitFailed,
    /// No suitable adapter found
    NoAdapter,
    /// Device creation failed
    DeviceCreationFailed,
    /// Shader compilation failed
    ShaderCompilationFailed(String),
    /// Resource creation failed
    ResourceCreationFailed(String),
    /// Invalid parameter
    InvalidParameter,
    /// Not supported
    NotSupported,
}

impl fmt::Display for DxvkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DxvkError::VulkanInitFailed => write!(f, "Vulkan initialization failed"),
            DxvkError::NoAdapter => write!(f, "No suitable adapter found"),
            DxvkError::DeviceCreationFailed => write!(f, "Device creation failed"),
            DxvkError::ShaderCompilationFailed(msg) => {
                write!(f, "Shader compilation failed: {}", msg)
            }
            DxvkError::ResourceCreationFailed(msg) => {
                write!(f, "Resource creation failed: {}", msg)
            }
            DxvkError::InvalidParameter => write!(f, "Invalid parameter"),
            DxvkError::NotSupported => write!(f, "Not supported"),
        }
    }
}

/// DXVK adapter (GPU)
#[derive(Debug, Clone)]
pub struct DxvkAdapter {
    /// Adapter name
    pub name: String,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// VRAM size in bytes
    pub vram_size: u64,
    /// Supports ray tracing
    pub supports_ray_tracing: bool,
}

impl DxvkAdapter {
    /// Create a new adapter
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vendor_id: 0,
            device_id: 0,
            vram_size: 0,
            supports_ray_tracing: false,
        }
    }

    /// Check if this is an AMD GPU
    pub fn is_amd(&self) -> bool {
        self.vendor_id == 0x1002
    }

    /// Check if this is an NVIDIA GPU
    pub fn is_nvidia(&self) -> bool {
        self.vendor_id == 0x10DE
    }

    /// Check if this is an Intel GPU
    pub fn is_intel(&self) -> bool {
        self.vendor_id == 0x8086
    }
}

/// DXVK device
pub struct DxvkDevice {
    /// Adapter this device was created from
    pub adapter: DxvkAdapter,
    /// Vulkan device handle (opaque)
    _vk_device: usize,
}

impl DxvkDevice {
    /// Create a device from an adapter
    pub fn create(adapter: DxvkAdapter) -> Result<Self, DxvkError> {
        log::info!("Creating DXVK device for adapter: {}", adapter.name);

        // In real implementation, would create Vulkan device
        Ok(Self {
            adapter,
            _vk_device: 0,
        })
    }

    /// Get device capabilities
    pub fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            max_texture_size: 16384,
            max_render_targets: 8,
            supports_compute: true,
            supports_geometry_shader: true,
            supports_tessellation: true,
            supports_ray_tracing: self.adapter.supports_ray_tracing,
        }
    }
}

impl fmt::Debug for DxvkDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DxvkDevice")
            .field("adapter", &self.adapter)
            .finish()
    }
}

/// Device capabilities
#[derive(Debug, Clone, Copy)]
pub struct DeviceCapabilities {
    pub max_texture_size: u32,
    pub max_render_targets: u32,
    pub supports_compute: bool,
    pub supports_geometry_shader: bool,
    pub supports_tessellation: bool,
    pub supports_ray_tracing: bool,
}

/// Enumerate available adapters
pub fn enumerate_adapters() -> Result<Vec<DxvkAdapter>, DxvkError> {
    log::info!("Enumerating DXVK adapters");

    // Get Vulkan drivers
    let drivers = vulkan_loader::enumerate_drivers().map_err(|_| DxvkError::VulkanInitFailed)?;

    let mut adapters = Vec::new();

    for driver in drivers {
        let mut adapter = DxvkAdapter::new(driver.name.clone());

        // Check for ray tracing support
        adapter.supports_ray_tracing = driver.supports_extension("VK_KHR_ray_tracing_pipeline");

        adapters.push(adapter);
    }

    if adapters.is_empty() {
        return Err(DxvkError::NoAdapter);
    }

    log::info!("Found {} adapter(s)", adapters.len());
    Ok(adapters)
}
