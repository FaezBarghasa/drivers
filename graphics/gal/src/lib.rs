//! Graphics Abstraction Layer (GAL)
//!
//! This crate provides a hardware-agnostic interface for GPU operations,
//! enabling Vulkan/WGPU command submission and hardware-accelerated rendering
//! in the RedoxOS microkernel environment.
//!
//! # Architecture
//!
//! The GAL sits between high-level graphics APIs (Vulkan, WGPU) and low-level
//! GPU drivers (VirtIO-GPU, discrete GPUs). It provides:
//!
//! - Memory management (buffers, images, allocations)
//! - Command buffer recording and submission
//! - Synchronization primitives (fences, semaphores)
//! - Pipeline state management
//! - Resource binding and descriptors
//!
//! # Usage
//!
//! ```ignore
//! use gal::{Device, CommandBuffer, Pipeline};
//!
//! let device = Device::create(DeviceType::VirtioGpu)?;
//! let cmd = device.create_command_buffer()?;
//!
//! cmd.begin()?;
//! cmd.bind_pipeline(&pipeline);
//! cmd.draw(vertex_count, instance_count, first_vertex, first_instance);
//! cmd.end()?;
//!
//! device.submit(&[cmd], fence)?;
//! ```

#![no_std]

extern crate alloc;

pub mod buffer;
pub mod command;
pub mod device;
pub mod image;
pub mod memory;
pub mod pipeline;
pub mod queue;
pub mod shader;
pub mod sync;
pub mod types;

// Re-exports
pub use buffer::{Buffer, BufferDescriptor, BufferUsage};
pub use command::{CommandBuffer, CommandPool, DrawCommand, RenderPass};
pub use device::{Device, DeviceCapabilities, DeviceInfo, DeviceType};
pub use image::{Image, ImageDescriptor, ImageFormat, ImageUsage, Sampler};
pub use memory::{AllocationInfo, Memory, MemoryAllocator, MemoryType};
pub use pipeline::{ComputePipeline, GraphicsPipeline, Pipeline, PipelineType};
pub use queue::{Queue, QueueType, SubmitInfo};
pub use shader::{Shader, ShaderModule, ShaderStage};
pub use sync::{Event, Fence, Semaphore};
pub use types::*;

use alloc::string::String;

/// GAL version
pub const GAL_VERSION: (u32, u32, u32) = (0, 1, 0);

/// Maximum number of descriptor sets per pipeline
pub const MAX_DESCRIPTOR_SETS: usize = 8;

/// Maximum number of push constant ranges
pub const MAX_PUSH_CONSTANT_RANGES: usize = 8;

/// Maximum number of vertex input bindings
pub const MAX_VERTEX_INPUT_BINDINGS: usize = 16;

/// Maximum number of vertex input attributes
pub const MAX_VERTEX_INPUT_ATTRIBUTES: usize = 32;

/// Maximum number of color attachments
pub const MAX_COLOR_ATTACHMENTS: usize = 8;

/// Result type for GAL operations
pub type Result<T> = core::result::Result<T, Error>;

/// GAL error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Device not found or initialization failed
    DeviceNotFound,
    /// Out of memory
    OutOfMemory,
    /// Out of device memory
    OutOfDeviceMemory,
    /// Invalid parameter
    InvalidParameter,
    /// Feature not supported
    NotSupported,
    /// Operation failed
    OperationFailed,
    /// Resource in use
    ResourceInUse,
    /// Timeout
    Timeout,
    /// Device lost
    DeviceLost,
    /// Shader compilation error
    ShaderCompilationFailed(String),
    /// Pipeline creation error
    PipelineCreationFailed(String),
    /// Command buffer error
    CommandBufferError(String),
    /// Synchronization error
    SyncError(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::DeviceNotFound => write!(f, "Device not found"),
            Error::OutOfMemory => write!(f, "Out of memory"),
            Error::OutOfDeviceMemory => write!(f, "Out of device memory"),
            Error::InvalidParameter => write!(f, "Invalid parameter"),
            Error::NotSupported => write!(f, "Not supported"),
            Error::OperationFailed => write!(f, "Operation failed"),
            Error::ResourceInUse => write!(f, "Resource in use"),
            Error::Timeout => write!(f, "Timeout"),
            Error::DeviceLost => write!(f, "Device lost"),
            Error::ShaderCompilationFailed(msg) => write!(f, "Shader compilation failed: {}", msg),
            Error::PipelineCreationFailed(msg) => write!(f, "Pipeline creation failed: {}", msg),
            Error::CommandBufferError(msg) => write!(f, "Command buffer error: {}", msg),
            Error::SyncError(msg) => write!(f, "Sync error: {}", msg),
        }
    }
}

/// Physical extent (2D)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

impl Extent2D {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Physical extent (3D)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl Extent3D {
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }
}

/// 2D offset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Offset2D {
    pub x: i32,
    pub y: i32,
}

impl Offset2D {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// 3D offset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Offset3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Offset3D {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Rectangle (2D region)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Rect2D {
    pub offset: Offset2D,
    pub extent: Extent2D,
}

impl Rect2D {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            offset: Offset2D::new(x, y),
            extent: Extent2D::new(width, height),
        }
    }
}

/// Viewport
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Viewport {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

/// Clear color value
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct ClearColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ClearColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
}

/// Clear depth-stencil value
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct ClearDepthStencil {
    pub depth: f32,
    pub stencil: u32,
}

impl ClearDepthStencil {
    pub const fn new(depth: f32, stencil: u32) -> Self {
        Self { depth, stencil }
    }
}

/// Union of clear values
#[derive(Clone, Copy)]
#[repr(C)]
pub union ClearValue {
    pub color: ClearColor,
    pub depth_stencil: ClearDepthStencil,
}

impl core::fmt::Debug for ClearValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Safely display as color by default
        write!(f, "ClearValue {{ ... }}")
    }
}

impl Default for ClearValue {
    fn default() -> Self {
        Self {
            color: ClearColor::BLACK,
        }
    }
}
