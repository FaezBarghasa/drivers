//! VirtIO-GPU Device implementation
//!
//! This module implements the GAL Device trait for VirtIO-GPU.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use spin::Mutex;

use gal::{
    Buffer, BufferDescriptor, CommandPool, Device, DeviceCapabilities, DeviceInfo, DeviceType,
    DisplayInfo, Error, Extent2D, Fence, GraphicsPipelineDescriptor, Image, ImageDescriptor,
    Memory, MemoryType, Pipeline, Queue, QueueType, Result, Semaphore, Shader, ShaderStage,
    SwapchainConfig,
};

use crate::command::VirtioCommandPool;
use crate::protocol::{self, CapsetType, CommandType, ControlHeader, MAX_SCANOUTS};
use crate::resource::{VirtioBuffer, VirtioImage, VirtioMemory};

/// Next resource ID counter
static NEXT_RESOURCE_ID: AtomicU32 = AtomicU32::new(1);

/// Next fence ID counter
static NEXT_FENCE_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a new resource ID
pub fn alloc_resource_id() -> u32 {
    NEXT_RESOURCE_ID.fetch_add(1, Ordering::SeqCst)
}

/// Allocate a new fence ID
pub fn alloc_fence_id() -> u64 {
    NEXT_FENCE_ID.fetch_add(1, Ordering::SeqCst)
}

/// VirtIO-GPU device implementation
pub struct VirtioGpuDevice {
    /// Device info
    info: DeviceInfo,
    /// Displays
    displays: Vec<DisplayInfo>,
    /// Control queue state
    control_queue: Mutex<ControlQueueState>,
    /// Graphics queue
    graphics_queue: VirtioQueue,
    /// Available capsets
    capsets: Vec<CapsetInfo>,
    /// Next context ID
    next_ctx_id: AtomicU32,
}

/// Control queue state
struct ControlQueueState {
    // In a real implementation, this would hold virtio queue handles
    pending_fences: Vec<u64>,
}

/// Graphics queue implementation
pub struct VirtioQueue {
    queue_type: QueueType,
}

impl Queue for VirtioQueue {
    fn queue_type(&self) -> QueueType {
        self.queue_type
    }

    fn submit(
        &self,
        _submits: &[gal::queue::SubmitInfo],
        _fence: Option<&dyn Fence>,
    ) -> Result<()> {
        // In a real implementation, this would submit commands to the virtio queue
        Ok(())
    }

    fn wait_idle(&self) -> Result<()> {
        // Wait for all pending commands to complete
        Ok(())
    }

    fn present(&self, _present_info: &gal::queue::PresentInfo) -> Result<()> {
        // Present swapchain image
        Ok(())
    }
}

/// Capset information
#[derive(Debug, Clone)]
struct CapsetInfo {
    id: CapsetType,
    max_version: u32,
    max_size: u32,
}

impl VirtioGpuDevice {
    /// Create a new VirtIO-GPU device
    pub fn create() -> Result<Self> {
        // In a real implementation, this would probe the PCI device
        // and initialize the virtio queues

        let mut capabilities = DeviceCapabilities::BLIT_2D | DeviceCapabilities::HW_CURSOR;

        // Check for 3D support
        let capsets = Self::probe_capsets()?;

        let has_virgl = capsets
            .iter()
            .any(|c| c.id == CapsetType::Virgl || c.id == CapsetType::Virgl2);
        let has_venus = capsets.iter().any(|c| c.id == CapsetType::Venus);

        if has_virgl || has_venus {
            capabilities |= DeviceCapabilities::RENDER_3D;
            capabilities |= DeviceCapabilities::CONTEXTS;
            capabilities |= DeviceCapabilities::SYNC_OBJECTS;
        }

        if has_venus {
            capabilities |= DeviceCapabilities::VULKAN;
        }

        let info = DeviceInfo {
            name: String::from("VirtIO GPU"),
            vendor_id: 0x1AF4, // Red Hat
            device_id: 0x1050, // VirtIO GPU
            device_type: if has_venus {
                DeviceType::VirtioGpu3D
            } else {
                DeviceType::VirtioGpu
            },
            capabilities,
            display_count: 1, // Will be updated after querying displays
            max_texture_2d: 16384,
            max_texture_3d: 2048,
            max_texture_layers: 2048,
            max_uniform_buffer_size: 65536,
            max_storage_buffer_size: 128 * 1024 * 1024,
            max_push_constant_size: 256,
            max_compute_work_group_count: [65535, 65535, 65535],
            max_compute_work_group_size: [1024, 1024, 64],
            max_compute_work_group_invocations: 1024,
            total_memory: 256 * 1024 * 1024, // 256 MB default
        };

        let displays = Self::query_displays()?;

        Ok(Self {
            info,
            displays,
            control_queue: Mutex::new(ControlQueueState {
                pending_fences: Vec::new(),
            }),
            graphics_queue: VirtioQueue {
                queue_type: QueueType::Graphics,
            },
            capsets,
            next_ctx_id: AtomicU32::new(1),
        })
    }

    /// Probe available capsets
    fn probe_capsets() -> Result<Vec<CapsetInfo>> {
        // In a real implementation, this would query the device for capsets
        // For now, return empty (2D only mode)
        Ok(Vec::new())
    }

    /// Query display information
    fn query_displays() -> Result<Vec<DisplayInfo>> {
        // In a real implementation, this would send VIRTIO_GPU_CMD_GET_DISPLAY_INFO
        Ok(vec![DisplayInfo {
            id: 0,
            name: String::from("VirtIO Display 0"),
            extent: Extent2D::new(1024, 768),
            refresh_rate: 60,
            is_primary: true,
            enabled: true,
        }])
    }

    /// Create a 3D context for Vulkan/OpenGL
    pub fn create_3d_context(&self, name: &str, capset: CapsetType) -> Result<u32> {
        if !self
            .info
            .capabilities
            .contains(DeviceCapabilities::CONTEXTS)
        {
            return Err(Error::NotSupported);
        }

        let ctx_id = self.next_ctx_id.fetch_add(1, Ordering::SeqCst);

        // In a real implementation, this would send VIRTIO_GPU_CMD_CTX_CREATE
        let _request = protocol::CtxCreate::new(ctx_id, capset as u32, name.as_bytes());

        Ok(ctx_id)
    }

    /// Destroy a 3D context
    pub fn destroy_3d_context(&self, ctx_id: u32) -> Result<()> {
        // In a real implementation, this would send VIRTIO_GPU_CMD_CTX_DESTROY
        let _request = protocol::CtxDestroy::new(ctx_id);
        Ok(())
    }

    /// Submit 3D commands (virgl/venus command stream)
    pub fn submit_3d(&self, ctx_id: u32, commands: &[u8]) -> Result<()> {
        // In a real implementation, this would send VIRTIO_GPU_CMD_SUBMIT_3D
        let _request = protocol::Submit3d::new(ctx_id, commands.len() as u32);
        Ok(())
    }

    /// Check if Venus (Vulkan) is supported
    pub fn supports_venus(&self) -> bool {
        self.capsets.iter().any(|c| c.id == CapsetType::Venus)
    }

    /// Check if virgl (OpenGL) is supported
    pub fn supports_virgl(&self) -> bool {
        self.capsets
            .iter()
            .any(|c| c.id == CapsetType::Virgl || c.id == CapsetType::Virgl2)
    }
}

impl Device for VirtioGpuDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn displays(&self) -> Vec<DisplayInfo> {
        self.displays.clone()
    }

    fn display(&self, id: usize) -> Option<DisplayInfo> {
        self.displays.get(id).cloned()
    }

    fn create_command_pool(&self, queue_type: QueueType) -> Result<Box<dyn CommandPool>> {
        Ok(Box::new(VirtioCommandPool::new(queue_type)))
    }

    fn create_buffer(&self, descriptor: &BufferDescriptor) -> Result<Box<dyn Buffer>> {
        let resource_id = alloc_resource_id();
        Ok(Box::new(VirtioBuffer::new(resource_id, descriptor)))
    }

    fn create_image(&self, descriptor: &ImageDescriptor) -> Result<Box<dyn Image>> {
        let resource_id = alloc_resource_id();
        Ok(Box::new(VirtioImage::new(resource_id, descriptor)))
    }

    fn allocate_memory(&self, size: u64, memory_type: MemoryType) -> Result<Box<dyn Memory>> {
        Ok(Box::new(VirtioMemory::new(size, memory_type)))
    }

    fn create_fence(&self, signaled: bool) -> Result<Box<dyn Fence>> {
        let fence_id = alloc_fence_id();
        Ok(Box::new(VirtioFence::new(fence_id, signaled)))
    }

    fn create_semaphore(&self) -> Result<Box<dyn Semaphore>> {
        Ok(Box::new(VirtioSemaphore::new()))
    }

    fn create_shader(&self, stage: ShaderStage, code: &[u8]) -> Result<Box<dyn Shader>> {
        Ok(Box::new(VirtioShader::new(stage, code)))
    }

    fn create_graphics_pipeline(
        &self,
        _desc: &GraphicsPipelineDescriptor,
    ) -> Result<Box<dyn Pipeline>> {
        Ok(Box::new(VirtioPipeline::new_graphics()))
    }

    fn create_compute_pipeline(&self, _shader: &dyn Shader) -> Result<Box<dyn Pipeline>> {
        Ok(Box::new(VirtioPipeline::new_compute()))
    }

    fn graphics_queue(&self) -> &dyn Queue {
        &self.graphics_queue
    }

    fn compute_queue(&self) -> Option<&dyn Queue> {
        // VirtIO-GPU doesn't have separate compute queue
        None
    }

    fn transfer_queue(&self) -> Option<&dyn Queue> {
        // VirtIO-GPU doesn't have separate transfer queue
        None
    }

    fn wait_idle(&self) -> Result<()> {
        self.graphics_queue.wait_idle()
    }

    fn create_swapchain(
        &self,
        config: &SwapchainConfig,
    ) -> Result<Box<dyn gal::device::Swapchain>> {
        Ok(Box::new(VirtioSwapchain::new(config)?))
    }
}

/// VirtIO fence implementation
pub struct VirtioFence {
    handle: usize,
    fence_id: u64,
    signaled: spin::RwLock<bool>,
}

impl VirtioFence {
    fn new(fence_id: u64, signaled: bool) -> Self {
        Self {
            handle: fence_id as usize,
            fence_id,
            signaled: spin::RwLock::new(signaled),
        }
    }
}

impl Fence for VirtioFence {
    fn handle(&self) -> usize {
        self.handle
    }

    fn is_signaled(&self) -> Result<bool> {
        Ok(*self.signaled.read())
    }

    fn wait(&self, _timeout_ns: u64) -> Result<bool> {
        // In a real implementation, this would wait for the fence
        Ok(*self.signaled.read())
    }

    fn reset(&self) -> Result<()> {
        *self.signaled.write() = false;
        Ok(())
    }
}

/// VirtIO semaphore implementation
pub struct VirtioSemaphore {
    handle: usize,
}

impl VirtioSemaphore {
    fn new() -> Self {
        static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);
        Self {
            handle: NEXT_HANDLE.fetch_add(1, Ordering::SeqCst) as usize,
        }
    }
}

impl Semaphore for VirtioSemaphore {
    fn handle(&self) -> usize {
        self.handle
    }
}

/// VirtIO shader implementation
pub struct VirtioShader {
    handle: usize,
    stage: ShaderStage,
    code: Vec<u8>,
}

impl VirtioShader {
    fn new(stage: ShaderStage, code: &[u8]) -> Self {
        static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);
        Self {
            handle: NEXT_HANDLE.fetch_add(1, Ordering::SeqCst) as usize,
            stage,
            code: code.to_vec(),
        }
    }
}

impl Shader for VirtioShader {
    fn handle(&self) -> usize {
        self.handle
    }

    fn stage(&self) -> ShaderStage {
        self.stage
    }

    fn entry_point(&self) -> &str {
        "main"
    }
}

/// VirtIO pipeline implementation
pub struct VirtioPipeline {
    handle: usize,
    pipeline_type: gal::PipelineType,
}

impl VirtioPipeline {
    fn new_graphics() -> Self {
        static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);
        Self {
            handle: NEXT_HANDLE.fetch_add(1, Ordering::SeqCst) as usize,
            pipeline_type: gal::PipelineType::Graphics,
        }
    }

    fn new_compute() -> Self {
        static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);
        Self {
            handle: NEXT_HANDLE.fetch_add(1, Ordering::SeqCst) as usize,
            pipeline_type: gal::PipelineType::Compute,
        }
    }
}

impl Pipeline for VirtioPipeline {
    fn handle(&self) -> usize {
        self.handle
    }

    fn pipeline_type(&self) -> gal::PipelineType {
        self.pipeline_type
    }
}

/// VirtIO swapchain implementation
pub struct VirtioSwapchain {
    extent: Extent2D,
    buffer_count: u32,
    images: Vec<VirtioImage>,
    current_image: AtomicU32,
}

impl VirtioSwapchain {
    fn new(config: &SwapchainConfig) -> Result<Self> {
        let buffer_count = config.buffer_count.max(2).min(3);
        let mut images = Vec::with_capacity(buffer_count as usize);

        for _ in 0..buffer_count {
            let resource_id = alloc_resource_id();
            let desc = ImageDescriptor::render_target(
                config.extent.width,
                config.extent.height,
                gal::ImageFormat::Bgra8Unorm,
            );
            images.push(VirtioImage::new(resource_id, &desc));
        }

        Ok(Self {
            extent: config.extent,
            buffer_count,
            images,
            current_image: AtomicU32::new(0),
        })
    }
}

impl gal::device::Swapchain for VirtioSwapchain {
    fn extent(&self) -> Extent2D {
        self.extent
    }

    fn buffer_count(&self) -> u32 {
        self.buffer_count
    }

    fn acquire_next_image(
        &self,
        _timeout_ns: u64,
        _semaphore: Option<&dyn Semaphore>,
        _fence: Option<&dyn Fence>,
    ) -> Result<u32> {
        let current = self.current_image.fetch_add(1, Ordering::SeqCst);
        Ok(current % self.buffer_count)
    }

    fn image(&self, index: u32) -> &dyn Image {
        &self.images[index as usize]
    }

    fn present(&self, _image_index: u32, _wait_semaphores: &[&dyn Semaphore]) -> Result<()> {
        // In a real implementation, this would flush the framebuffer
        Ok(())
    }
}
