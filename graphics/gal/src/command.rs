//! Command buffer recording and submission
//!
//! This module provides abstractions for recording GPU commands and
//! submitting them for execution.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{
    Buffer, ClearValue, Error, Extent2D, Image, Offset2D, Pipeline, Rect2D, Result, Viewport,
};

/// Command pool for allocating command buffers
pub trait CommandPool: Send + Sync {
    /// Allocate a command buffer
    fn allocate(&self) -> Result<Box<dyn CommandBuffer>>;

    /// Free all command buffers
    fn reset(&self) -> Result<()>;
}

/// Command buffer recording level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferLevel {
    /// Primary command buffer (submitted directly)
    Primary,
    /// Secondary command buffer (executed from primary)
    Secondary,
}

/// Command buffer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferState {
    /// Initial state, ready for recording
    Initial,
    /// Recording commands
    Recording,
    /// Recording complete, ready for submission
    Executable,
    /// Submitted for execution
    Pending,
    /// Invalid state
    Invalid,
}

/// Command buffer for recording GPU commands
pub trait CommandBuffer: Send + Sync {
    /// Get current state
    fn state(&self) -> CommandBufferState;

    /// Begin recording
    fn begin(&mut self) -> Result<()>;

    /// End recording
    fn end(&mut self) -> Result<()>;

    /// Reset the command buffer
    fn reset(&mut self) -> Result<()>;

    // === Render Pass ===

    /// Begin a render pass
    fn begin_render_pass(&mut self, desc: &RenderPassDescriptor) -> Result<()>;

    /// End the current render pass
    fn end_render_pass(&mut self);

    // === Pipeline Binding ===

    /// Bind a graphics or compute pipeline
    fn bind_pipeline(&mut self, pipeline: &dyn Pipeline);

    // === Viewport and Scissor ===

    /// Set viewport
    fn set_viewport(&mut self, viewport: Viewport);

    /// Set scissor rectangle
    fn set_scissor(&mut self, scissor: Rect2D);

    // === Vertex and Index Buffers ===

    /// Bind vertex buffer(s)
    fn bind_vertex_buffers(&mut self, first_binding: u32, buffers: &[&dyn Buffer], offsets: &[u64]);

    /// Bind index buffer
    fn bind_index_buffer(&mut self, buffer: &dyn Buffer, offset: u64, index_type: IndexType);

    // === Drawing ===

    /// Draw primitives
    fn draw(&mut self, cmd: DrawCommand);

    /// Draw indexed primitives
    fn draw_indexed(&mut self, cmd: DrawIndexedCommand);

    /// Indirect draw
    fn draw_indirect(&mut self, buffer: &dyn Buffer, offset: u64, draw_count: u32, stride: u32);

    /// Indirect draw indexed
    fn draw_indexed_indirect(
        &mut self,
        buffer: &dyn Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    );

    // === Compute ===

    /// Dispatch compute work
    fn dispatch(&mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32);

    /// Indirect compute dispatch
    fn dispatch_indirect(&mut self, buffer: &dyn Buffer, offset: u64);

    // === Copy Operations ===

    /// Copy buffer to buffer
    fn copy_buffer(&mut self, src: &dyn Buffer, dst: &dyn Buffer, regions: &[BufferCopy]);

    /// Copy buffer to image
    fn copy_buffer_to_image(
        &mut self,
        src: &dyn Buffer,
        dst: &dyn Image,
        regions: &[BufferImageCopy],
    );

    /// Copy image to buffer
    fn copy_image_to_buffer(
        &mut self,
        src: &dyn Image,
        dst: &dyn Buffer,
        regions: &[BufferImageCopy],
    );

    /// Copy image to image
    fn copy_image(&mut self, src: &dyn Image, dst: &dyn Image, regions: &[ImageCopy]);

    /// Blit image (with scaling)
    fn blit_image(
        &mut self,
        src: &dyn Image,
        dst: &dyn Image,
        regions: &[ImageBlit],
        filter: Filter,
    );

    // === Clear Operations ===

    /// Clear color image
    fn clear_color_image(
        &mut self,
        image: &dyn Image,
        color: ClearValue,
        ranges: &[ImageSubresourceRange],
    );

    /// Clear depth/stencil image
    fn clear_depth_stencil_image(
        &mut self,
        image: &dyn Image,
        depth_stencil: ClearValue,
        ranges: &[ImageSubresourceRange],
    );

    // === Barriers ===

    /// Pipeline barrier
    fn pipeline_barrier(&mut self, barrier: &PipelineBarrier);

    // === Push Constants ===

    /// Set push constants
    fn push_constants(&mut self, stages: ShaderStageFlags, offset: u32, data: &[u8]);
}

/// Render pass descriptor
#[derive(Debug, Clone)]
pub struct RenderPassDescriptor<'a> {
    /// Color attachments
    pub color_attachments: Vec<ColorAttachment<'a>>,
    /// Depth/stencil attachment
    pub depth_stencil_attachment: Option<DepthStencilAttachment<'a>>,
    /// Render area
    pub render_area: Rect2D,
}

/// Color attachment for render pass
#[derive(Debug, Clone)]
pub struct ColorAttachment<'a> {
    /// Image view to render to
    pub image: &'a dyn Image,
    /// Load operation
    pub load_op: LoadOp,
    /// Store operation
    pub store_op: StoreOp,
    /// Clear value (used if load_op is Clear)
    pub clear_value: ClearValue,
}

/// Depth/stencil attachment for render pass
#[derive(Debug, Clone)]
pub struct DepthStencilAttachment<'a> {
    /// Image view to render to
    pub image: &'a dyn Image,
    /// Depth load operation
    pub depth_load_op: LoadOp,
    /// Depth store operation
    pub depth_store_op: StoreOp,
    /// Stencil load operation
    pub stencil_load_op: LoadOp,
    /// Stencil store operation
    pub stencil_store_op: StoreOp,
    /// Clear value
    pub clear_value: ClearValue,
}

/// Load operation for attachments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOp {
    /// Load existing contents
    Load,
    /// Clear to a value
    Clear,
    /// Don't care about previous contents
    DontCare,
}

/// Store operation for attachments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp {
    /// Store the results
    Store,
    /// Don't care about storing
    DontCare,
}

/// Draw command parameters
#[derive(Debug, Clone, Copy)]
pub struct DrawCommand {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl DrawCommand {
    pub fn new(vertex_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    pub fn instanced(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}

/// Draw indexed command parameters
#[derive(Debug, Clone, Copy)]
pub struct DrawIndexedCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

impl DrawIndexedCommand {
    pub fn new(index_count: u32) -> Self {
        Self {
            index_count,
            instance_count: 1,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }
}

/// Index type for indexed drawing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    U16,
    U32,
}

/// Buffer copy region
#[derive(Debug, Clone, Copy)]
pub struct BufferCopy {
    pub src_offset: u64,
    pub dst_offset: u64,
    pub size: u64,
}

/// Buffer to image copy region
#[derive(Debug, Clone, Copy)]
pub struct BufferImageCopy {
    pub buffer_offset: u64,
    pub buffer_row_length: u32,
    pub buffer_image_height: u32,
    pub image_subresource: ImageSubresourceLayers,
    pub image_offset: crate::Offset3D,
    pub image_extent: crate::Extent3D,
}

/// Image copy region
#[derive(Debug, Clone, Copy)]
pub struct ImageCopy {
    pub src_subresource: ImageSubresourceLayers,
    pub src_offset: crate::Offset3D,
    pub dst_subresource: ImageSubresourceLayers,
    pub dst_offset: crate::Offset3D,
    pub extent: crate::Extent3D,
}

/// Image blit region
#[derive(Debug, Clone, Copy)]
pub struct ImageBlit {
    pub src_subresource: ImageSubresourceLayers,
    pub src_offsets: [crate::Offset3D; 2],
    pub dst_subresource: ImageSubresourceLayers,
    pub dst_offsets: [crate::Offset3D; 2],
}

/// Image subresource layers
#[derive(Debug, Clone, Copy)]
pub struct ImageSubresourceLayers {
    pub aspect_mask: ImageAspect,
    pub mip_level: u32,
    pub base_array_layer: u32,
    pub layer_count: u32,
}

/// Image subresource range
#[derive(Debug, Clone, Copy)]
pub struct ImageSubresourceRange {
    pub aspect_mask: ImageAspect,
    pub base_mip_level: u32,
    pub level_count: u32,
    pub base_array_layer: u32,
    pub layer_count: u32,
}

use bitflags::bitflags;

bitflags! {
    /// Image aspect flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ImageAspect: u32 {
        const COLOR = 1 << 0;
        const DEPTH = 1 << 1;
        const STENCIL = 1 << 2;
    }
}

/// Filter mode for image operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Linear,
}

/// Pipeline barrier
#[derive(Debug, Clone)]
pub struct PipelineBarrier {
    pub src_stage: PipelineStageFlags,
    pub dst_stage: PipelineStageFlags,
    pub memory_barriers: Vec<MemoryBarrier>,
    pub buffer_barriers: Vec<BufferMemoryBarrier>,
    pub image_barriers: Vec<ImageMemoryBarrier>,
}

impl Default for PipelineBarrier {
    fn default() -> Self {
        Self {
            src_stage: PipelineStageFlags::TOP_OF_PIPE,
            dst_stage: PipelineStageFlags::BOTTOM_OF_PIPE,
            memory_barriers: Vec::new(),
            buffer_barriers: Vec::new(),
            image_barriers: Vec::new(),
        }
    }
}

bitflags! {
    /// Pipeline stage flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PipelineStageFlags: u32 {
        const TOP_OF_PIPE = 1 << 0;
        const DRAW_INDIRECT = 1 << 1;
        const VERTEX_INPUT = 1 << 2;
        const VERTEX_SHADER = 1 << 3;
        const FRAGMENT_SHADER = 1 << 4;
        const EARLY_FRAGMENT_TESTS = 1 << 5;
        const LATE_FRAGMENT_TESTS = 1 << 6;
        const COLOR_ATTACHMENT_OUTPUT = 1 << 7;
        const COMPUTE_SHADER = 1 << 8;
        const TRANSFER = 1 << 9;
        const BOTTOM_OF_PIPE = 1 << 10;
        const HOST = 1 << 11;
        const ALL_GRAPHICS = 1 << 12;
        const ALL_COMMANDS = 1 << 13;
    }
}

bitflags! {
    /// Shader stage flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ShaderStageFlags: u32 {
        const VERTEX = 1 << 0;
        const FRAGMENT = 1 << 1;
        const COMPUTE = 1 << 2;
        const GEOMETRY = 1 << 3;
        const TESSELLATION_CONTROL = 1 << 4;
        const TESSELLATION_EVALUATION = 1 << 5;
        const ALL_GRAPHICS = Self::VERTEX.bits() | Self::FRAGMENT.bits() | Self::GEOMETRY.bits();
        const ALL = 0xFFFFFFFF;
    }
}

/// Memory barrier
#[derive(Debug, Clone, Copy)]
pub struct MemoryBarrier {
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
}

/// Buffer memory barrier
#[derive(Debug, Clone)]
pub struct BufferMemoryBarrier {
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
    pub buffer_handle: usize, // Buffer handle ID
    pub offset: u64,
    pub size: u64,
}

/// Image memory barrier
#[derive(Debug, Clone)]
pub struct ImageMemoryBarrier {
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
    pub old_layout: ImageLayout,
    pub new_layout: ImageLayout,
    pub image_handle: usize, // Image handle ID
    pub subresource_range: ImageSubresourceRange,
}

bitflags! {
    /// Access flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u32 {
        const NONE = 0;
        const INDIRECT_COMMAND_READ = 1 << 0;
        const INDEX_READ = 1 << 1;
        const VERTEX_ATTRIBUTE_READ = 1 << 2;
        const UNIFORM_READ = 1 << 3;
        const INPUT_ATTACHMENT_READ = 1 << 4;
        const SHADER_READ = 1 << 5;
        const SHADER_WRITE = 1 << 6;
        const COLOR_ATTACHMENT_READ = 1 << 7;
        const COLOR_ATTACHMENT_WRITE = 1 << 8;
        const DEPTH_STENCIL_READ = 1 << 9;
        const DEPTH_STENCIL_WRITE = 1 << 10;
        const TRANSFER_READ = 1 << 11;
        const TRANSFER_WRITE = 1 << 12;
        const HOST_READ = 1 << 13;
        const HOST_WRITE = 1 << 14;
        const MEMORY_READ = 1 << 15;
        const MEMORY_WRITE = 1 << 16;
    }
}

/// Image layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageLayout {
    Undefined,
    General,
    ColorAttachmentOptimal,
    DepthStencilAttachmentOptimal,
    DepthStencilReadOnlyOptimal,
    ShaderReadOnlyOptimal,
    TransferSrcOptimal,
    TransferDstOptimal,
    Preinitialized,
    PresentSrc,
}

/// Render pass for legacy APIs (pre-recorded render pass)
pub trait RenderPass: Send + Sync {
    /// Get the render area extent
    fn extent(&self) -> Extent2D;
}
