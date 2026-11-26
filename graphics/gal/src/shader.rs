//! Shader module management
//!
//! This module provides abstractions for shader programs.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{Error, Result};

/// Shader stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessellationControl,
    TessellationEvaluation,
}

impl ShaderStage {
    /// Get the stage name for debugging
    pub fn name(&self) -> &'static str {
        match self {
            ShaderStage::Vertex => "Vertex",
            ShaderStage::Fragment => "Fragment",
            ShaderStage::Compute => "Compute",
            ShaderStage::Geometry => "Geometry",
            ShaderStage::TessellationControl => "TessControl",
            ShaderStage::TessellationEvaluation => "TessEval",
        }
    }

    /// Convert to shader stage flags
    pub fn to_flags(&self) -> crate::command::ShaderStageFlags {
        match self {
            ShaderStage::Vertex => crate::command::ShaderStageFlags::VERTEX,
            ShaderStage::Fragment => crate::command::ShaderStageFlags::FRAGMENT,
            ShaderStage::Compute => crate::command::ShaderStageFlags::COMPUTE,
            ShaderStage::Geometry => crate::command::ShaderStageFlags::GEOMETRY,
            ShaderStage::TessellationControl => {
                crate::command::ShaderStageFlags::TESSELLATION_CONTROL
            }
            ShaderStage::TessellationEvaluation => {
                crate::command::ShaderStageFlags::TESSELLATION_EVALUATION
            }
        }
    }
}

/// Shader module format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderFormat {
    /// SPIR-V binary
    SpirV,
    /// GLSL source
    Glsl,
    /// HLSL source
    Hlsl,
    /// Metal Shading Language
    Msl,
    /// WGSL (WebGPU Shading Language)
    Wgsl,
}

/// Shader module trait
pub trait Shader: Send + Sync {
    /// Get shader handle ID
    fn handle(&self) -> usize;

    /// Get shader stage
    fn stage(&self) -> ShaderStage;

    /// Get entry point name
    fn entry_point(&self) -> &str;
}

/// Concrete shader module implementation
#[derive(Debug, Clone)]
pub struct ShaderModule {
    /// Handle ID
    handle: usize,
    /// Shader stage
    stage: ShaderStage,
    /// Entry point name
    entry_point: String,
    /// SPIR-V bytecode
    spirv: Vec<u32>,
}

impl ShaderModule {
    /// Create a new shader module from SPIR-V bytecode
    pub fn new(stage: ShaderStage, spirv: Vec<u32>, entry_point: &str) -> Self {
        static NEXT_HANDLE: core::sync::atomic::AtomicUsize =
            core::sync::atomic::AtomicUsize::new(1);
        let handle = NEXT_HANDLE.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        Self {
            handle,
            stage,
            entry_point: String::from(entry_point),
            spirv,
        }
    }

    /// Create from descriptor
    pub fn from_descriptor(desc: &ShaderModuleDescriptor) -> Result<Self> {
        match &desc.code {
            ShaderCode::SpirV(code) => Ok(Self::new(desc.stage, code.to_vec(), desc.entry_point)),
            ShaderCode::Source { .. } => Err(Error::NotSupported),
        }
    }

    /// Get SPIR-V bytecode
    pub fn spirv(&self) -> &[u32] {
        &self.spirv
    }
}

impl Shader for ShaderModule {
    fn handle(&self) -> usize {
        self.handle
    }

    fn stage(&self) -> ShaderStage {
        self.stage
    }

    fn entry_point(&self) -> &str {
        &self.entry_point
    }
}

/// Shader module descriptor
#[derive(Debug, Clone)]
pub struct ShaderModuleDescriptor<'a> {
    /// Shader stage
    pub stage: ShaderStage,
    /// Shader code
    pub code: ShaderCode<'a>,
    /// Entry point name
    pub entry_point: &'a str,
    /// Debug label
    pub label: Option<&'a str>,
}

/// Shader code (source or binary)
#[derive(Debug, Clone)]
pub enum ShaderCode<'a> {
    /// SPIR-V binary
    SpirV(&'a [u32]),
    /// Source code with format
    Source { code: &'a str, format: ShaderFormat },
}

impl<'a> ShaderModuleDescriptor<'a> {
    /// Create a vertex shader descriptor from SPIR-V
    pub fn vertex_spirv(code: &'a [u32]) -> Self {
        Self {
            stage: ShaderStage::Vertex,
            code: ShaderCode::SpirV(code),
            entry_point: "main",
            label: None,
        }
    }

    /// Create a fragment shader descriptor from SPIR-V
    pub fn fragment_spirv(code: &'a [u32]) -> Self {
        Self {
            stage: ShaderStage::Fragment,
            code: ShaderCode::SpirV(code),
            entry_point: "main",
            label: None,
        }
    }

    /// Create a compute shader descriptor from SPIR-V
    pub fn compute_spirv(code: &'a [u32]) -> Self {
        Self {
            stage: ShaderStage::Compute,
            code: ShaderCode::SpirV(code),
            entry_point: "main",
            label: None,
        }
    }

    /// Set entry point
    pub fn entry_point(mut self, entry_point: &'a str) -> Self {
        self.entry_point = entry_point;
        self
    }

    /// Set debug label
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Shader reflection info (extracted from SPIR-V)
#[derive(Debug, Clone, Default)]
pub struct ShaderReflection {
    /// Input variables
    pub inputs: Vec<ShaderVariable>,
    /// Output variables
    pub outputs: Vec<ShaderVariable>,
    /// Uniform buffers
    pub uniform_buffers: Vec<UniformBufferInfo>,
    /// Storage buffers
    pub storage_buffers: Vec<StorageBufferInfo>,
    /// Textures
    pub textures: Vec<TextureInfo>,
    /// Samplers
    pub samplers: Vec<SamplerInfo>,
    /// Push constant range
    pub push_constants: Option<PushConstantInfo>,
    /// Compute work group size (for compute shaders)
    pub work_group_size: Option<[u32; 3]>,
}

/// Shader variable info
#[derive(Debug, Clone)]
pub struct ShaderVariable {
    pub name: String,
    pub location: u32,
    pub format: VariableFormat,
}

/// Variable format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableFormat {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    IVec2,
    IVec3,
    IVec4,
    UInt,
    UVec2,
    UVec3,
    UVec4,
    Mat2,
    Mat3,
    Mat4,
}

/// Uniform buffer info
#[derive(Debug, Clone)]
pub struct UniformBufferInfo {
    pub name: String,
    pub set: u32,
    pub binding: u32,
    pub size: u32,
}

/// Storage buffer info
#[derive(Debug, Clone)]
pub struct StorageBufferInfo {
    pub name: String,
    pub set: u32,
    pub binding: u32,
    pub read_only: bool,
}

/// Texture info
#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub name: String,
    pub set: u32,
    pub binding: u32,
    pub dimension: TextureDimension,
    pub multisampled: bool,
}

/// Texture dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
    Cube,
    D1Array,
    D2Array,
    CubeArray,
}

/// Sampler info
#[derive(Debug, Clone)]
pub struct SamplerInfo {
    pub name: String,
    pub set: u32,
    pub binding: u32,
}

/// Push constant info
#[derive(Debug, Clone)]
pub struct PushConstantInfo {
    pub size: u32,
    pub stages: crate::command::ShaderStageFlags,
}
