//! VirtIO-GPU command buffer implementation
//!
//! This module provides command buffer and command pool for VirtIO-GPU.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use gal::command::{
    AccessFlags, BufferCopy, BufferImageCopy, BufferMemoryBarrier, ColorAttachment,
    DepthStencilAttachment, DrawCommand, DrawIndexedCommand, Filter, ImageAspect, ImageBlit,
    ImageCopy, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers, ImageSubresourceRange,
    IndexType, LoadOp, MemoryBarrier, PipelineBarrier, PipelineStageFlags, RenderPassDescriptor,
    ShaderStageFlags, StoreOp,
};
use gal::{
    Buffer, ClearValue, CommandBuffer, CommandBufferState, CommandPool, Error, Image, Pipeline,
    QueueType, Rect2D, Result, Viewport,
};

/// VirtIO command pool
pub struct VirtioCommandPool {
    handle: usize,
    queue_type: QueueType,
    command_buffers: spin::Mutex<Vec<usize>>,
    next_id: AtomicU32,
}

impl VirtioCommandPool {
    pub fn new(queue_type: QueueType) -> Self {
        static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

        Self {
            handle: NEXT_HANDLE.fetch_add(1, Ordering::SeqCst) as usize,
            queue_type,
            command_buffers: spin::Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl CommandPool for VirtioCommandPool {
    fn allocate(&self) -> Result<Box<dyn CommandBuffer>> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as usize;
        self.command_buffers.lock().push(id);
        Ok(Box::new(VirtioCommandBuffer::new(id, self.queue_type)))
    }

    fn reset(&self) -> Result<()> {
        self.command_buffers.lock().clear();
        Ok(())
    }
}

/// Recorded command
#[derive(Debug, Clone)]
pub enum RecordedCommand {
    BeginRenderPass {
        color_attachments: Vec<ColorAttachmentInfo>,
        depth_attachment: Option<DepthAttachmentInfo>,
        render_area: Rect2D,
    },
    EndRenderPass,
    BindPipeline {
        handle: usize,
    },
    SetViewport {
        viewport: Viewport,
    },
    SetScissor {
        scissor: Rect2D,
    },
    BindVertexBuffers {
        first_binding: u32,
        buffer_handles: Vec<usize>,
        offsets: Vec<u64>,
    },
    BindIndexBuffer {
        buffer_handle: usize,
        offset: u64,
        index_type: IndexType,
    },
    Draw(DrawCommand),
    DrawIndexed(DrawIndexedCommand),
    DrawIndirect {
        buffer_handle: usize,
        offset: u64,
        draw_count: u32,
        stride: u32,
    },
    DrawIndexedIndirect {
        buffer_handle: usize,
        offset: u64,
        draw_count: u32,
        stride: u32,
    },
    Dispatch {
        x: u32,
        y: u32,
        z: u32,
    },
    DispatchIndirect {
        buffer_handle: usize,
        offset: u64,
    },
    CopyBuffer {
        src_handle: usize,
        dst_handle: usize,
        regions: Vec<BufferCopy>,
    },
    CopyBufferToImage {
        src_handle: usize,
        dst_handle: usize,
        regions: Vec<BufferImageCopy>,
    },
    CopyImageToBuffer {
        src_handle: usize,
        dst_handle: usize,
        regions: Vec<BufferImageCopy>,
    },
    CopyImage {
        src_handle: usize,
        dst_handle: usize,
        regions: Vec<ImageCopy>,
    },
    BlitImage {
        src_handle: usize,
        dst_handle: usize,
        regions: Vec<ImageBlit>,
        filter: Filter,
    },
    ClearColorImage {
        image_handle: usize,
        color: ClearValue,
        ranges: Vec<ImageSubresourceRange>,
    },
    ClearDepthStencilImage {
        image_handle: usize,
        depth_stencil: ClearValue,
        ranges: Vec<ImageSubresourceRange>,
    },
    PipelineBarrier(PipelineBarrier),
    PushConstants {
        stages: ShaderStageFlags,
        offset: u32,
        data: Vec<u8>,
    },
}

/// Simplified color attachment info for recording
#[derive(Debug, Clone)]
pub struct ColorAttachmentInfo {
    pub image_handle: usize,
    pub load_op: LoadOp,
    pub store_op: StoreOp,
    pub clear_value: ClearValue,
}

/// Simplified depth attachment info for recording
#[derive(Debug, Clone)]
pub struct DepthAttachmentInfo {
    pub image_handle: usize,
    pub depth_load_op: LoadOp,
    pub depth_store_op: StoreOp,
    pub stencil_load_op: LoadOp,
    pub stencil_store_op: StoreOp,
    pub clear_value: ClearValue,
}

/// VirtIO command buffer
pub struct VirtioCommandBuffer {
    handle: usize,
    queue_type: QueueType,
    state: spin::RwLock<CommandBufferState>,
    commands: spin::Mutex<Vec<RecordedCommand>>,
    in_render_pass: spin::RwLock<bool>,
}

impl VirtioCommandBuffer {
    pub fn new(handle: usize, queue_type: QueueType) -> Self {
        Self {
            handle,
            queue_type,
            state: spin::RwLock::new(CommandBufferState::Initial),
            commands: spin::Mutex::new(Vec::new()),
            in_render_pass: spin::RwLock::new(false),
        }
    }

    /// Get the recorded commands
    pub fn recorded_commands(&self) -> Vec<RecordedCommand> {
        self.commands.lock().clone()
    }
}

impl CommandBuffer for VirtioCommandBuffer {
    fn state(&self) -> CommandBufferState {
        *self.state.read()
    }

    fn begin(&mut self) -> Result<()> {
        let state = *self.state.read();
        if state != CommandBufferState::Initial && state != CommandBufferState::Executable {
            return Err(Error::CommandBufferError("Invalid state for begin".into()));
        }

        self.commands.lock().clear();
        *self.state.write() = CommandBufferState::Recording;
        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        if *self.state.read() != CommandBufferState::Recording {
            return Err(Error::CommandBufferError("Not recording".into()));
        }

        if *self.in_render_pass.read() {
            return Err(Error::CommandBufferError("Render pass not ended".into()));
        }

        *self.state.write() = CommandBufferState::Executable;
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.commands.lock().clear();
        *self.state.write() = CommandBufferState::Initial;
        *self.in_render_pass.write() = false;
        Ok(())
    }

    fn begin_render_pass(&mut self, desc: &RenderPassDescriptor) -> Result<()> {
        if *self.in_render_pass.read() {
            return Err(Error::CommandBufferError("Already in render pass".into()));
        }

        let color_attachments: Vec<ColorAttachmentInfo> = desc
            .color_attachments
            .iter()
            .map(|a| ColorAttachmentInfo {
                image_handle: a.image.handle(),
                load_op: a.load_op,
                store_op: a.store_op,
                clear_value: a.clear_value,
            })
            .collect();

        let depth_attachment =
            desc.depth_stencil_attachment
                .as_ref()
                .map(|a| DepthAttachmentInfo {
                    image_handle: a.image.handle(),
                    depth_load_op: a.depth_load_op,
                    depth_store_op: a.depth_store_op,
                    stencil_load_op: a.stencil_load_op,
                    stencil_store_op: a.stencil_store_op,
                    clear_value: a.clear_value,
                });

        self.commands.lock().push(RecordedCommand::BeginRenderPass {
            color_attachments,
            depth_attachment,
            render_area: desc.render_area,
        });

        *self.in_render_pass.write() = true;
        Ok(())
    }

    fn end_render_pass(&mut self) {
        if *self.in_render_pass.read() {
            self.commands.lock().push(RecordedCommand::EndRenderPass);
            *self.in_render_pass.write() = false;
        }
    }

    fn bind_pipeline(&mut self, pipeline: &dyn Pipeline) {
        self.commands.lock().push(RecordedCommand::BindPipeline {
            handle: pipeline.handle(),
        });
    }

    fn set_viewport(&mut self, viewport: Viewport) {
        self.commands
            .lock()
            .push(RecordedCommand::SetViewport { viewport });
    }

    fn set_scissor(&mut self, scissor: Rect2D) {
        self.commands
            .lock()
            .push(RecordedCommand::SetScissor { scissor });
    }

    fn bind_vertex_buffers(
        &mut self,
        first_binding: u32,
        buffers: &[&dyn Buffer],
        offsets: &[u64],
    ) {
        let buffer_handles: Vec<usize> = buffers.iter().map(|b| b.handle()).collect();
        self.commands
            .lock()
            .push(RecordedCommand::BindVertexBuffers {
                first_binding,
                buffer_handles,
                offsets: offsets.to_vec(),
            });
    }

    fn bind_index_buffer(&mut self, buffer: &dyn Buffer, offset: u64, index_type: IndexType) {
        self.commands.lock().push(RecordedCommand::BindIndexBuffer {
            buffer_handle: buffer.handle(),
            offset,
            index_type,
        });
    }

    fn draw(&mut self, cmd: DrawCommand) {
        self.commands.lock().push(RecordedCommand::Draw(cmd));
    }

    fn draw_indexed(&mut self, cmd: DrawIndexedCommand) {
        self.commands.lock().push(RecordedCommand::DrawIndexed(cmd));
    }

    fn draw_indirect(&mut self, buffer: &dyn Buffer, offset: u64, draw_count: u32, stride: u32) {
        self.commands.lock().push(RecordedCommand::DrawIndirect {
            buffer_handle: buffer.handle(),
            offset,
            draw_count,
            stride,
        });
    }

    fn draw_indexed_indirect(
        &mut self,
        buffer: &dyn Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        self.commands
            .lock()
            .push(RecordedCommand::DrawIndexedIndirect {
                buffer_handle: buffer.handle(),
                offset,
                draw_count,
                stride,
            });
    }

    fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.commands
            .lock()
            .push(RecordedCommand::Dispatch { x, y, z });
    }

    fn dispatch_indirect(&mut self, buffer: &dyn Buffer, offset: u64) {
        self.commands
            .lock()
            .push(RecordedCommand::DispatchIndirect {
                buffer_handle: buffer.handle(),
                offset,
            });
    }

    fn copy_buffer(&mut self, src: &dyn Buffer, dst: &dyn Buffer, regions: &[BufferCopy]) {
        self.commands.lock().push(RecordedCommand::CopyBuffer {
            src_handle: src.handle(),
            dst_handle: dst.handle(),
            regions: regions.to_vec(),
        });
    }

    fn copy_buffer_to_image(
        &mut self,
        src: &dyn Buffer,
        dst: &dyn Image,
        regions: &[BufferImageCopy],
    ) {
        self.commands
            .lock()
            .push(RecordedCommand::CopyBufferToImage {
                src_handle: src.handle(),
                dst_handle: dst.handle(),
                regions: regions.to_vec(),
            });
    }

    fn copy_image_to_buffer(
        &mut self,
        src: &dyn Image,
        dst: &dyn Buffer,
        regions: &[BufferImageCopy],
    ) {
        self.commands
            .lock()
            .push(RecordedCommand::CopyImageToBuffer {
                src_handle: src.handle(),
                dst_handle: dst.handle(),
                regions: regions.to_vec(),
            });
    }

    fn copy_image(&mut self, src: &dyn Image, dst: &dyn Image, regions: &[ImageCopy]) {
        self.commands.lock().push(RecordedCommand::CopyImage {
            src_handle: src.handle(),
            dst_handle: dst.handle(),
            regions: regions.to_vec(),
        });
    }

    fn blit_image(
        &mut self,
        src: &dyn Image,
        dst: &dyn Image,
        regions: &[ImageBlit],
        filter: Filter,
    ) {
        self.commands.lock().push(RecordedCommand::BlitImage {
            src_handle: src.handle(),
            dst_handle: dst.handle(),
            regions: regions.to_vec(),
            filter,
        });
    }

    fn clear_color_image(
        &mut self,
        image: &dyn Image,
        color: ClearValue,
        ranges: &[ImageSubresourceRange],
    ) {
        self.commands.lock().push(RecordedCommand::ClearColorImage {
            image_handle: image.handle(),
            color,
            ranges: ranges.to_vec(),
        });
    }

    fn clear_depth_stencil_image(
        &mut self,
        image: &dyn Image,
        depth_stencil: ClearValue,
        ranges: &[ImageSubresourceRange],
    ) {
        self.commands
            .lock()
            .push(RecordedCommand::ClearDepthStencilImage {
                image_handle: image.handle(),
                depth_stencil,
                ranges: ranges.to_vec(),
            });
    }

    fn pipeline_barrier(&mut self, barrier: &PipelineBarrier) {
        self.commands
            .lock()
            .push(RecordedCommand::PipelineBarrier(barrier.clone()));
    }

    fn push_constants(&mut self, stages: ShaderStageFlags, offset: u32, data: &[u8]) {
        self.commands.lock().push(RecordedCommand::PushConstants {
            stages,
            offset,
            data: data.to_vec(),
        });
    }
}
