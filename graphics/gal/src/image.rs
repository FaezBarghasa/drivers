//! Image and texture resource management
//!
//! This module provides GPU image abstractions for textures,
//! render targets, and depth/stencil buffers.

use bitflags::bitflags;

use crate::{Error, Extent2D, Extent3D, Memory, MemoryType, Result};

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageFormat {
    // 8-bit
    R8Unorm,
    R8Snorm,
    R8Uint,
    R8Sint,

    // 16-bit
    R16Unorm,
    R16Snorm,
    R16Uint,
    R16Sint,
    R16Float,
    Rg8Unorm,
    Rg8Snorm,
    Rg8Uint,
    Rg8Sint,

    // 32-bit
    R32Uint,
    R32Sint,
    R32Float,
    Rg16Unorm,
    Rg16Snorm,
    Rg16Uint,
    Rg16Sint,
    Rg16Float,
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Rgba8Snorm,
    Rgba8Uint,
    Rgba8Sint,
    Bgra8Unorm,
    Bgra8UnormSrgb,

    // 64-bit
    Rg32Uint,
    Rg32Sint,
    Rg32Float,
    Rgba16Unorm,
    Rgba16Snorm,
    Rgba16Uint,
    Rgba16Sint,
    Rgba16Float,

    // 128-bit
    Rgba32Uint,
    Rgba32Sint,
    Rgba32Float,

    // Depth/Stencil
    Depth16Unorm,
    Depth24Plus,
    Depth24PlusStencil8,
    Depth32Float,
    Depth32FloatStencil8,
    Stencil8,

    // Compressed formats
    Bc1RgbaUnorm,
    Bc1RgbaUnormSrgb,
    Bc2RgbaUnorm,
    Bc2RgbaUnormSrgb,
    Bc3RgbaUnorm,
    Bc3RgbaUnormSrgb,
    Bc4RUnorm,
    Bc4RSnorm,
    Bc5RgUnorm,
    Bc5RgSnorm,
    Bc6hRgufloat,
    Bc6hRgfloat,
    Bc7RgbaUnorm,
    Bc7RgbaUnormSrgb,
}

impl ImageFormat {
    /// Get bytes per pixel for uncompressed formats
    pub fn bytes_per_pixel(&self) -> Option<u32> {
        match self {
            ImageFormat::R8Unorm
            | ImageFormat::R8Snorm
            | ImageFormat::R8Uint
            | ImageFormat::R8Sint => Some(1),
            ImageFormat::R16Unorm
            | ImageFormat::R16Snorm
            | ImageFormat::R16Uint
            | ImageFormat::R16Sint
            | ImageFormat::R16Float
            | ImageFormat::Rg8Unorm
            | ImageFormat::Rg8Snorm
            | ImageFormat::Rg8Uint
            | ImageFormat::Rg8Sint
            | ImageFormat::Depth16Unorm => Some(2),
            ImageFormat::R32Uint
            | ImageFormat::R32Sint
            | ImageFormat::R32Float
            | ImageFormat::Rg16Unorm
            | ImageFormat::Rg16Snorm
            | ImageFormat::Rg16Uint
            | ImageFormat::Rg16Sint
            | ImageFormat::Rg16Float
            | ImageFormat::Rgba8Unorm
            | ImageFormat::Rgba8UnormSrgb
            | ImageFormat::Rgba8Snorm
            | ImageFormat::Rgba8Uint
            | ImageFormat::Rgba8Sint
            | ImageFormat::Bgra8Unorm
            | ImageFormat::Bgra8UnormSrgb
            | ImageFormat::Depth24Plus
            | ImageFormat::Depth24PlusStencil8
            | ImageFormat::Depth32Float => Some(4),
            ImageFormat::Rg32Uint
            | ImageFormat::Rg32Sint
            | ImageFormat::Rg32Float
            | ImageFormat::Rgba16Unorm
            | ImageFormat::Rgba16Snorm
            | ImageFormat::Rgba16Uint
            | ImageFormat::Rgba16Sint
            | ImageFormat::Rgba16Float
            | ImageFormat::Depth32FloatStencil8 => Some(8),
            ImageFormat::Rgba32Uint | ImageFormat::Rgba32Sint | ImageFormat::Rgba32Float => {
                Some(16)
            }
            ImageFormat::Stencil8 => Some(1),
            _ => None, // Compressed formats
        }
    }

    /// Check if this is a depth format
    pub fn is_depth(&self) -> bool {
        matches!(
            self,
            ImageFormat::Depth16Unorm
                | ImageFormat::Depth24Plus
                | ImageFormat::Depth24PlusStencil8
                | ImageFormat::Depth32Float
                | ImageFormat::Depth32FloatStencil8
        )
    }

    /// Check if this is a stencil format
    pub fn is_stencil(&self) -> bool {
        matches!(
            self,
            ImageFormat::Stencil8
                | ImageFormat::Depth24PlusStencil8
                | ImageFormat::Depth32FloatStencil8
        )
    }

    /// Check if this is a compressed format
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            ImageFormat::Bc1RgbaUnorm
                | ImageFormat::Bc1RgbaUnormSrgb
                | ImageFormat::Bc2RgbaUnorm
                | ImageFormat::Bc2RgbaUnormSrgb
                | ImageFormat::Bc3RgbaUnorm
                | ImageFormat::Bc3RgbaUnormSrgb
                | ImageFormat::Bc4RUnorm
                | ImageFormat::Bc4RSnorm
                | ImageFormat::Bc5RgUnorm
                | ImageFormat::Bc5RgSnorm
                | ImageFormat::Bc6hRgufloat
                | ImageFormat::Bc6hRgfloat
                | ImageFormat::Bc7RgbaUnorm
                | ImageFormat::Bc7RgbaUnormSrgb
        )
    }

    /// Check if this is an sRGB format
    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            ImageFormat::Rgba8UnormSrgb
                | ImageFormat::Bgra8UnormSrgb
                | ImageFormat::Bc1RgbaUnormSrgb
                | ImageFormat::Bc2RgbaUnormSrgb
                | ImageFormat::Bc3RgbaUnormSrgb
                | ImageFormat::Bc7RgbaUnormSrgb
        )
    }
}

bitflags! {
    /// Image usage flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ImageUsage: u32 {
        /// Image can be used as source for transfer operations
        const TRANSFER_SRC = 1 << 0;
        /// Image can be used as destination for transfer operations
        const TRANSFER_DST = 1 << 1;
        /// Image can be sampled in shaders
        const SAMPLED = 1 << 2;
        /// Image can be used as a storage image
        const STORAGE = 1 << 3;
        /// Image can be used as a color attachment
        const COLOR_ATTACHMENT = 1 << 4;
        /// Image can be used as a depth/stencil attachment
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        /// Image contents may be transient
        const TRANSIENT_ATTACHMENT = 1 << 6;
        /// Image can be used as an input attachment
        const INPUT_ATTACHMENT = 1 << 7;
    }
}

/// Image dimension type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDimension {
    D1,
    D2,
    D3,
}

/// Image descriptor for creation
#[derive(Debug, Clone)]
pub struct ImageDescriptor {
    /// Image dimension
    pub dimension: ImageDimension,
    /// Image extent
    pub extent: Extent3D,
    /// Pixel format
    pub format: ImageFormat,
    /// Number of mip levels
    pub mip_levels: u32,
    /// Number of array layers
    pub array_layers: u32,
    /// Sample count (for multisampling)
    pub sample_count: u32,
    /// Usage flags
    pub usage: ImageUsage,
    /// Memory type requirements
    pub memory_type: MemoryType,
    /// Debug label
    pub label: Option<&'static str>,
}

impl ImageDescriptor {
    /// Create a new 2D image descriptor
    pub fn new_2d(width: u32, height: u32, format: ImageFormat, usage: ImageUsage) -> Self {
        Self {
            dimension: ImageDimension::D2,
            extent: Extent3D::new(width, height, 1),
            format,
            mip_levels: 1,
            array_layers: 1,
            sample_count: 1,
            usage,
            memory_type: MemoryType::DeviceLocal,
            label: None,
        }
    }

    /// Create a render target descriptor
    pub fn render_target(width: u32, height: u32, format: ImageFormat) -> Self {
        Self::new_2d(
            width,
            height,
            format,
            ImageUsage::COLOR_ATTACHMENT | ImageUsage::SAMPLED | ImageUsage::TRANSFER_SRC,
        )
    }

    /// Create a depth buffer descriptor
    pub fn depth_buffer(width: u32, height: u32) -> Self {
        Self::new_2d(
            width,
            height,
            ImageFormat::Depth24PlusStencil8,
            ImageUsage::DEPTH_STENCIL_ATTACHMENT,
        )
    }

    /// Create a texture descriptor
    pub fn texture(width: u32, height: u32, format: ImageFormat) -> Self {
        Self::new_2d(
            width,
            height,
            format,
            ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
        )
    }

    /// Set mip levels
    pub fn mip_levels(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// Set array layers
    pub fn array_layers(mut self, layers: u32) -> Self {
        self.array_layers = layers;
        self
    }

    /// Set sample count
    pub fn sample_count(mut self, samples: u32) -> Self {
        self.sample_count = samples;
        self
    }

    /// Set debug label
    pub fn label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// Calculate maximum mip levels for this image
    pub fn max_mip_levels(&self) -> u32 {
        let max_dim = self
            .extent
            .width
            .max(self.extent.height)
            .max(self.extent.depth);
        (32 - max_dim.leading_zeros()).max(1)
    }

    /// Calculate full mip chain
    pub fn with_full_mip_chain(mut self) -> Self {
        self.mip_levels = self.max_mip_levels();
        self
    }
}

/// GPU image resource
pub trait Image: Send + Sync {
    /// Get image handle ID
    fn handle(&self) -> usize;

    /// Get image dimension
    fn dimension(&self) -> ImageDimension;

    /// Get image extent
    fn extent(&self) -> Extent3D;

    /// Get 2D extent (convenience)
    fn extent_2d(&self) -> Extent2D {
        let ext = self.extent();
        Extent2D::new(ext.width, ext.height)
    }

    /// Get image format
    fn format(&self) -> ImageFormat;

    /// Get mip level count
    fn mip_levels(&self) -> u32;

    /// Get array layer count
    fn array_layers(&self) -> u32;

    /// Get sample count
    fn sample_count(&self) -> u32;

    /// Get usage flags
    fn usage(&self) -> ImageUsage;

    /// Get associated memory
    fn memory(&self) -> Option<&dyn Memory>;
}

/// Sampler filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerFilter {
    Nearest,
    Linear,
}

/// Sampler address mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerAddressMode {
    Repeat,
    MirrorRepeat,
    ClampToEdge,
    ClampToBorder,
}

/// Sampler border color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerBorderColor {
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}

/// Sampler descriptor
#[derive(Debug, Clone)]
pub struct SamplerDescriptor {
    pub mag_filter: SamplerFilter,
    pub min_filter: SamplerFilter,
    pub mipmap_filter: SamplerFilter,
    pub address_mode_u: SamplerAddressMode,
    pub address_mode_v: SamplerAddressMode,
    pub address_mode_w: SamplerAddressMode,
    pub mip_lod_bias: f32,
    pub anisotropy_enable: bool,
    pub max_anisotropy: f32,
    pub compare_enable: bool,
    pub compare_op: crate::device::CompareOp,
    pub min_lod: f32,
    pub max_lod: f32,
    pub border_color: SamplerBorderColor,
    pub unnormalized_coordinates: bool,
}

impl Default for SamplerDescriptor {
    fn default() -> Self {
        Self {
            mag_filter: SamplerFilter::Linear,
            min_filter: SamplerFilter::Linear,
            mipmap_filter: SamplerFilter::Linear,
            address_mode_u: SamplerAddressMode::Repeat,
            address_mode_v: SamplerAddressMode::Repeat,
            address_mode_w: SamplerAddressMode::Repeat,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: crate::device::CompareOp::Always,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: SamplerBorderColor::TransparentBlack,
            unnormalized_coordinates: false,
        }
    }
}

impl SamplerDescriptor {
    /// Create a linear sampler
    pub fn linear() -> Self {
        Self::default()
    }

    /// Create a nearest-neighbor sampler
    pub fn nearest() -> Self {
        Self {
            mag_filter: SamplerFilter::Nearest,
            min_filter: SamplerFilter::Nearest,
            mipmap_filter: SamplerFilter::Nearest,
            ..Self::default()
        }
    }

    /// Enable anisotropic filtering
    pub fn anisotropy(mut self, max: f32) -> Self {
        self.anisotropy_enable = true;
        self.max_anisotropy = max;
        self
    }

    /// Set address mode for all dimensions
    pub fn address_mode(mut self, mode: SamplerAddressMode) -> Self {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self.address_mode_w = mode;
        self
    }
}

/// Texture sampler
pub trait Sampler: Send + Sync {
    /// Get sampler handle ID
    fn handle(&self) -> usize;
}
