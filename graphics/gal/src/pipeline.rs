//! Pipeline state management
//!
//! This module provides abstractions for graphics and compute pipelines.

use alloc::vec::Vec;

use crate::Shader;

/// Pipeline type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineType {
    Graphics,
    Compute,
}

/// Pipeline trait
pub trait Pipeline: Send + Sync {
    /// Get pipeline handle ID
    fn handle(&self) -> usize;

    /// Get pipeline type
    fn pipeline_type(&self) -> PipelineType;
}

/// Graphics pipeline
pub trait GraphicsPipeline: Pipeline {
    // Graphics pipeline specific methods would go here
}

/// Compute pipeline
pub trait ComputePipeline: Pipeline {
    /// Get work group size
    fn work_group_size(&self) -> [u32; 3];
}

/// Pipeline layout for resource binding
pub trait PipelineLayout: Send + Sync {
    /// Get layout handle ID
    fn handle(&self) -> usize;
}

/// Descriptor set layout
pub struct DescriptorSetLayout {
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

/// Descriptor set layout binding
#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: crate::command::ShaderStageFlags,
}

/// Descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    InputAttachment,
}

/// Push constant range
#[derive(Debug, Clone, Copy)]
pub struct PushConstantRange {
    pub stage_flags: crate::command::ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
}
