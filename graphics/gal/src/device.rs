//! Device abstraction for GPU hardware
//!
//! This module provides the core `Device` trait and implementations for
//! different GPU backends (VirtIO-GPU, etc.)

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use bitflags::bitflags;

use crate::{
    Buffer, BufferDescriptor, CommandPool, Error, Extent2D, Fence, Image, ImageDescriptor, Memory,
    MemoryType, Pipeline, Queue, QueueType, Result, Semaphore, Shader, ShaderStage,
};

/// Type of GPU device
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// VirtIO GPU device (for VMs)
    VirtioGpu,
    /// VirtIO GPU with 3D acceleration (virgl/venus)
    VirtioGpu3D,
    /// Discrete GPU (AMD, NVIDIA, Intel)
    Discrete,
    /// Integrated GPU
    Integrated,
    /// Software renderer (fallback)
    Software,
    /// Unknown device type
    Unknown,
}

impl DeviceType {
    /// Check if this device type supports hardware acceleration
    pub fn supports_acceleration(&self) -> bool {
        matches!(
            self,
            DeviceType::VirtioGpu3D | DeviceType::Discrete | DeviceType::Integrated
        )
    }
}

bitflags! {
    /// Device capability flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DeviceCapabilities: u64 {
        /// Supports 2D blitting operations
        const BLIT_2D = 1 << 0;
        /// Supports 3D rendering
        const RENDER_3D = 1 << 1;
        /// Supports compute shaders
        const COMPUTE = 1 << 2;
        /// Supports Vulkan API
        const VULKAN = 1 << 3;
        /// Supports hardware cursor
        const HW_CURSOR = 1 << 4;
        /// Supports multiple displays
        const MULTI_DISPLAY = 1 << 5;
        /// Supports EDID reading
        const EDID = 1 << 6;
        /// Supports blob resources (host-visible memory)
        const BLOB_RESOURCES = 1 << 7;
        /// Supports context isolation
        const CONTEXTS = 1 << 8;
        /// Supports synchronization objects
        const SYNC_OBJECTS = 1 << 9;
        /// Supports timeline semaphores
        const TIMELINE_SEMAPHORES = 1 << 10;
        /// Supports sparse resources
        const SPARSE = 1 << 11;
        /// Supports async compute
        const ASYNC_COMPUTE = 1 << 12;
        /// Supports async transfer
        const ASYNC_TRANSFER = 1 << 13;
        /// Supports ray tracing
        const RAY_TRACING = 1 << 14;
        /// Supports mesh shaders
        const MESH_SHADERS = 1 << 15;
    }
}

/// Information about a GPU device
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device name
    pub name: String,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Device type
    pub device_type: DeviceType,
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
    /// Number of displays
    pub display_count: usize,
    /// Maximum texture dimension (2D)
    pub max_texture_2d: u32,
    /// Maximum texture dimension (3D)
    pub max_texture_3d: u32,
    /// Maximum texture array layers
    pub max_texture_layers: u32,
    /// Maximum uniform buffer size
    pub max_uniform_buffer_size: u32,
    /// Maximum storage buffer size
    pub max_storage_buffer_size: u64,
    /// Maximum push constant size
    pub max_push_constant_size: u32,
    /// Maximum compute work group count
    pub max_compute_work_group_count: [u32; 3],
    /// Maximum compute work group size
    pub max_compute_work_group_size: [u32; 3],
    /// Maximum compute work group invocations
    pub max_compute_work_group_invocations: u32,
    /// Total device memory (bytes)
    pub total_memory: u64,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            vendor_id: 0,
            device_id: 0,
            device_type: DeviceType::Unknown,
            capabilities: DeviceCapabilities::empty(),
            display_count: 0,
            max_texture_2d: 4096,
            max_texture_3d: 256,
            max_texture_layers: 256,
            max_uniform_buffer_size: 16384,
            max_storage_buffer_size: 128 * 1024 * 1024,
            max_push_constant_size: 128,
            max_compute_work_group_count: [65535, 65535, 65535],
            max_compute_work_group_size: [256, 256, 64],
            max_compute_work_group_invocations: 256,
            total_memory: 0,
        }
    }
}

/// Display information
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// Display index
    pub id: usize,
    /// Display name
    pub name: String,
    /// Current resolution
    pub extent: Extent2D,
    /// Refresh rate in Hz
    pub refresh_rate: u32,
    /// Is this the primary display
    pub is_primary: bool,
    /// Is the display enabled
    pub enabled: bool,
}

/// Swapchain configuration
#[derive(Debug, Clone)]
pub struct SwapchainConfig {
    /// Display to present to
    pub display_id: usize,
    /// Preferred extent
    pub extent: Extent2D,
    /// Number of buffers (double/triple buffering)
    pub buffer_count: u32,
    /// V-sync enabled
    pub vsync: bool,
}

/// Core device trait for GPU operations
pub trait Device: Send + Sync {
    /// Get device information
    fn info(&self) -> &DeviceInfo;

    /// Get list of displays
    fn displays(&self) -> Vec<DisplayInfo>;

    /// Get display by ID
    fn display(&self, id: usize) -> Option<DisplayInfo>;

    /// Create a command pool
    fn create_command_pool(&self, queue_type: QueueType) -> Result<Box<dyn CommandPool>>;

    /// Create a buffer
    fn create_buffer(&self, descriptor: &BufferDescriptor) -> Result<Box<dyn Buffer>>;

    /// Create an image
    fn create_image(&self, descriptor: &ImageDescriptor) -> Result<Box<dyn Image>>;

    /// Allocate device memory
    fn allocate_memory(&self, size: u64, memory_type: MemoryType) -> Result<Box<dyn Memory>>;

    /// Create a fence
    fn create_fence(&self, signaled: bool) -> Result<Box<dyn Fence>>;

    /// Create a semaphore
    fn create_semaphore(&self) -> Result<Box<dyn Semaphore>>;

    /// Create a shader module
    fn create_shader(&self, stage: ShaderStage, code: &[u8]) -> Result<Box<dyn Shader>>;

    /// Create a graphics pipeline
    fn create_graphics_pipeline(
        &self,
        desc: &GraphicsPipelineDescriptor,
    ) -> Result<Box<dyn Pipeline>>;

    /// Create a compute pipeline
    fn create_compute_pipeline(&self, shader: &dyn Shader) -> Result<Box<dyn Pipeline>>;

    /// Get the graphics queue
    fn graphics_queue(&self) -> &dyn Queue;

    /// Get the compute queue (if available)
    fn compute_queue(&self) -> Option<&dyn Queue>;

    /// Get the transfer queue (if available)
    fn transfer_queue(&self) -> Option<&dyn Queue>;

    /// Wait for device to be idle
    fn wait_idle(&self) -> Result<()>;

    /// Create a swapchain for presentation
    fn create_swapchain(&self, config: &SwapchainConfig) -> Result<Box<dyn Swapchain>>;
}

/// Swapchain for presenting to displays
pub trait Swapchain: Send + Sync {
    /// Get the current extent
    fn extent(&self) -> Extent2D;

    /// Get buffer count
    fn buffer_count(&self) -> u32;

    /// Acquire next image for rendering
    fn acquire_next_image(
        &self,
        timeout_ns: u64,
        semaphore: Option<&dyn Semaphore>,
        fence: Option<&dyn Fence>,
    ) -> Result<u32>;

    /// Get image at index
    fn image(&self, index: u32) -> &dyn Image;

    /// Present the rendered image
    fn present(&self, image_index: u32, wait_semaphores: &[&dyn Semaphore]) -> Result<()>;
}

/// Descriptor for graphics pipeline creation
#[derive(Debug, Clone)]
pub struct GraphicsPipelineDescriptor {
    /// Vertex shader
    pub vertex_shader: usize, // Handle to shader
    /// Fragment shader
    pub fragment_shader: usize, // Handle to shader
    /// Vertex input bindings
    pub vertex_bindings: Vec<VertexBinding>,
    /// Vertex input attributes
    pub vertex_attributes: Vec<VertexAttribute>,
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face
    pub front_face: FrontFace,
    /// Depth test enabled
    pub depth_test: bool,
    /// Depth write enabled
    pub depth_write: bool,
    /// Depth compare operation
    pub depth_compare: CompareOp,
    /// Color blend attachments
    pub blend_attachments: Vec<ColorBlendAttachment>,
    /// Color attachment formats
    pub color_formats: Vec<crate::ImageFormat>,
    /// Depth attachment format
    pub depth_format: Option<crate::ImageFormat>,
}

impl Default for GraphicsPipelineDescriptor {
    fn default() -> Self {
        Self {
            vertex_shader: 0,
            fragment_shader: 0,
            vertex_bindings: Vec::new(),
            vertex_attributes: Vec::new(),
            topology: PrimitiveTopology::TriangleList,
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            depth_test: false,
            depth_write: false,
            depth_compare: CompareOp::Less,
            blend_attachments: Vec::new(),
            color_formats: Vec::new(),
            depth_format: None,
        }
    }
}

/// Vertex input binding
#[derive(Debug, Clone, Copy)]
pub struct VertexBinding {
    pub binding: u32,
    pub stride: u32,
    pub input_rate: VertexInputRate,
}

/// Vertex input attribute
#[derive(Debug, Clone, Copy)]
pub struct VertexAttribute {
    pub location: u32,
    pub binding: u32,
    pub format: VertexFormat,
    pub offset: u32,
}

/// Vertex input rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

/// Vertex attribute format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float,
    Float2,
    Float3,
    Float4,
    Int,
    Int2,
    Int3,
    Int4,
    UInt,
    UInt2,
    UInt3,
    UInt4,
    Byte4,
    Byte4Norm,
    UByte4,
    UByte4Norm,
}

/// Primitive topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    TriangleFan,
}

/// Polygon fill mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    Fill,
    Line,
    Point,
}

/// Face culling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None,
    Front,
    Back,
    FrontAndBack,
}

/// Winding order for front-facing polygons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

/// Comparison operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

/// Color blend attachment
#[derive(Debug, Clone, Copy)]
pub struct ColorBlendAttachment {
    pub blend_enable: bool,
    pub src_color_factor: BlendFactor,
    pub dst_color_factor: BlendFactor,
    pub color_op: BlendOp,
    pub src_alpha_factor: BlendFactor,
    pub dst_alpha_factor: BlendFactor,
    pub alpha_op: BlendOp,
    pub write_mask: ColorWriteMask,
}

impl Default for ColorBlendAttachment {
    fn default() -> Self {
        Self {
            blend_enable: false,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::Zero,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::Zero,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::all(),
        }
    }
}

/// Blend factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
}

/// Blend operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

bitflags! {
    /// Color write mask
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ColorWriteMask: u8 {
        const R = 1 << 0;
        const G = 1 << 1;
        const B = 1 << 2;
        const A = 1 << 3;
    }
}

/// Enumerate available GPU devices
pub fn enumerate_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    // Try VirtIO-GPU
    #[cfg(feature = "virtio")]
    {
        if let Ok(virtio_devices) = enumerate_virtio_devices() {
            devices.extend(virtio_devices);
        }
    }

    // Add software renderer as fallback
    devices.push(DeviceInfo {
        name: String::from("Software Renderer"),
        vendor_id: 0,
        device_id: 0,
        device_type: DeviceType::Software,
        capabilities: DeviceCapabilities::BLIT_2D | DeviceCapabilities::RENDER_3D,
        ..Default::default()
    });

    Ok(devices)
}

#[cfg(feature = "virtio")]
fn enumerate_virtio_devices() -> Result<Vec<DeviceInfo>> {
    // This would probe for VirtIO-GPU devices
    // For now, return empty - actual implementation would query PCId
    Ok(Vec::new())
}
