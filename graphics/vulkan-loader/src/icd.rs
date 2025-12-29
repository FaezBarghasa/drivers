//! ICD (Installable Client Driver) discovery and management

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// ICD manifest describing a Vulkan driver
#[derive(Debug, Clone)]
pub struct IcdManifest {
    /// Driver name
    pub name: String,
    /// API version supported
    pub api_version: crate::VulkanVersion,
    /// Path to driver library
    pub library_path: String,
    /// Supported extensions
    pub extensions: Vec<String>,
}

impl IcdManifest {
    /// Create a new ICD manifest
    pub fn new(name: impl ToString, api_version: crate::VulkanVersion) -> Self {
        Self {
            name: name.to_string(),
            api_version,
            library_path: String::new(),
            extensions: Vec::new(),
        }
    }

    /// Check if extension is supported
    pub fn supports_extension(&self, ext_name: &str) -> bool {
        self.extensions.iter().any(|e| e == ext_name)
    }
}

/// Loaded ICD driver instance
pub struct IcdDriver {
    /// Manifest
    pub manifest: IcdManifest,
    /// Function pointers (opaque for now)
    _handle: usize,
}

impl IcdDriver {
    /// Load an ICD from manifest
    pub fn load(manifest: IcdManifest) -> Result<Self, &'static str> {
        log::info!("Loading ICD: {}", manifest.name);

        // In a real implementation, this would:
        // 1. Open the driver library
        // 2. Resolve vkGetInstanceProcAddr
        // 3. Initialize the driver

        Ok(Self {
            manifest,
            _handle: 0,
        })
    }

    /// Get instance proc address
    pub fn get_instance_proc_addr(&self, _name: &str) -> Option<usize> {
        // Would return function pointer
        None
    }
}

impl fmt::Debug for IcdDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IcdDriver")
            .field("manifest", &self.manifest)
            .finish()
    }
}
