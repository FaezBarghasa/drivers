//! Vulkan extensions support

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// Vulkan extension
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    /// Extension name
    pub name: String,
    /// Spec version
    pub spec_version: u32,
}

impl Extension {
    pub fn new(name: impl Into<String>, spec_version: u32) -> Self {
        Self {
            name: name.into(),
            spec_version,
        }
    }
}

/// Ray tracing extension support
pub struct RayTracingExtensions;

impl RayTracingExtensions {
    /// VK_KHR_ray_tracing_pipeline
    pub const RAY_TRACING_PIPELINE: &'static str = "VK_KHR_ray_tracing_pipeline";

    /// VK_KHR_acceleration_structure
    pub const ACCELERATION_STRUCTURE: &'static str = "VK_KHR_acceleration_structure";

    /// VK_KHR_ray_query
    pub const RAY_QUERY: &'static str = "VK_KHR_ray_query";

    /// VK_KHR_deferred_host_operations
    pub const DEFERRED_HOST_OPERATIONS: &'static str = "VK_KHR_deferred_host_operations";

    /// Get all required ray tracing extensions
    pub fn required_extensions() -> Vec<Extension> {
        vec![
            Extension::new(Self::RAY_TRACING_PIPELINE, 1),
            Extension::new(Self::ACCELERATION_STRUCTURE, 13),
            Extension::new(Self::RAY_QUERY, 1),
            Extension::new(Self::DEFERRED_HOST_OPERATIONS, 4),
        ]
    }

    /// Check if all ray tracing extensions are supported
    pub fn are_supported(available: &[String]) -> bool {
        let required = [
            Self::RAY_TRACING_PIPELINE,
            Self::ACCELERATION_STRUCTURE,
            Self::RAY_QUERY,
        ];

        required
            .iter()
            .all(|&ext| available.iter().any(|a| a == ext))
    }
}

/// Mesh shader extensions
pub struct MeshShaderExtensions;

impl MeshShaderExtensions {
    /// VK_EXT_mesh_shader
    pub const MESH_SHADER: &'static str = "VK_EXT_mesh_shader";

    /// Get mesh shader extensions
    pub fn required_extensions() -> Vec<Extension> {
        vec![Extension::new(Self::MESH_SHADER, 1)]
    }
}

/// Common instance extensions
pub struct InstanceExtensions;

impl InstanceExtensions {
    pub const SURFACE: &'static str = "VK_KHR_surface";
    pub const SWAPCHAIN: &'static str = "VK_KHR_swapchain";
    pub const GET_PHYSICAL_DEVICE_PROPERTIES_2: &'static str =
        "VK_KHR_get_physical_device_properties2";
}

/// Extension registry
pub struct ExtensionRegistry {
    extensions: Vec<Extension>,
}

impl ExtensionRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    /// Register an extension
    pub fn register(&mut self, ext: Extension) {
        if !self.extensions.iter().any(|e| e.name == ext.name) {
            self.extensions.push(ext);
        }
    }

    /// Check if extension is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.extensions.iter().any(|e| e.name == name)
    }

    /// Get all extensions
    pub fn all(&self) -> &[Extension] {
        &self.extensions
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
