//! Queue management and command submission
//!
//! This module provides abstractions for GPU queues and command submission.

use alloc::boxed::Box;

use crate::{CommandBuffer, Error, Fence, Result, Semaphore};

/// Queue type/family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    /// Graphics queue (supports all operations)
    Graphics,
    /// Compute queue (compute and transfer)
    Compute,
    /// Transfer queue (DMA only)
    Transfer,
}

impl QueueType {
    /// Check if this queue supports graphics operations
    pub fn supports_graphics(&self) -> bool {
        matches!(self, QueueType::Graphics)
    }

    /// Check if this queue supports compute operations
    pub fn supports_compute(&self) -> bool {
        matches!(self, QueueType::Graphics | QueueType::Compute)
    }

    /// Check if this queue supports transfer operations
    pub fn supports_transfer(&self) -> bool {
        true // All queues support transfer
    }
}

/// Submit info for queue submission
pub struct SubmitInfo<'a> {
    /// Command buffers to submit
    pub command_buffers: &'a [&'a dyn CommandBuffer],
    /// Semaphores to wait on before execution
    pub wait_semaphores: &'a [(&'a dyn Semaphore, PipelineStage)],
    /// Semaphores to signal after execution
    pub signal_semaphores: &'a [&'a dyn Semaphore],
}

impl<'a> SubmitInfo<'a> {
    /// Create a simple submit info with just command buffers
    pub fn new(command_buffers: &'a [&'a dyn CommandBuffer]) -> Self {
        Self {
            command_buffers,
            wait_semaphores: &[],
            signal_semaphores: &[],
        }
    }
}

/// Pipeline stage for synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    TopOfPipe,
    DrawIndirect,
    VertexInput,
    VertexShader,
    FragmentShader,
    EarlyFragmentTests,
    LateFragmentTests,
    ColorAttachmentOutput,
    ComputeShader,
    Transfer,
    BottomOfPipe,
    Host,
    AllGraphics,
    AllCommands,
}

/// GPU queue
pub trait Queue: Send + Sync {
    /// Get queue type
    fn queue_type(&self) -> QueueType;

    /// Submit command buffers for execution
    fn submit(&self, submits: &[SubmitInfo], fence: Option<&dyn Fence>) -> Result<()>;

    /// Wait for queue to be idle
    fn wait_idle(&self) -> Result<()>;

    /// Present a swapchain image (for graphics queue)
    fn present(&self, present_info: &PresentInfo) -> Result<()>;
}

/// Present info for swapchain presentation
pub struct PresentInfo<'a> {
    /// Semaphores to wait on before presentation
    pub wait_semaphores: &'a [&'a dyn Semaphore],
    /// Swapchain handles
    pub swapchains: &'a [usize],
    /// Image indices to present
    pub image_indices: &'a [u32],
}
