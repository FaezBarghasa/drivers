//! D3D9 to Vulkan translation

use crate::common::{DxvkDevice, DxvkError};
use alloc::vec::Vec;

/// D3D9 device wrapper
pub struct D3D9Device {
    /// Underlying DXVK device
    _device: DxvkDevice,
}

impl D3D9Device {
    /// Create D3D9 device
    pub fn create(device: DxvkDevice) -> Result<Self, DxvkError> {
        log::info!("Creating D3D9 device");
        Ok(Self { _device: device })
    }

    /// Begin scene
    pub fn begin_scene(&mut self) -> Result<(), DxvkError> {
        log::trace!("D3D9: BeginScene");
        Ok(())
    }

    /// End scene
    pub fn end_scene(&mut self) -> Result<(), DxvkError> {
        log::trace!("D3D9: EndScene");
        Ok(())
    }

    /// Clear
    pub fn clear(
        &mut self,
        _flags: u32,
        _color: u32,
        _z: f32,
        _stencil: u32,
    ) -> Result<(), DxvkError> {
        log::trace!("D3D9: Clear");
        Ok(())
    }

    /// Draw primitive
    pub fn draw_primitive(
        &mut self,
        _primitive_type: D3D9PrimitiveType,
        _start_vertex: u32,
        _primitive_count: u32,
    ) -> Result<(), DxvkError> {
        log::trace!("D3D9: DrawPrimitive");
        Ok(())
    }

    /// Draw indexed primitive
    pub fn draw_indexed_primitive(
        &mut self,
        _primitive_type: D3D9PrimitiveType,
        _base_vertex: i32,
        _min_index: u32,
        _num_vertices: u32,
        _start_index: u32,
        _primitive_count: u32,
    ) -> Result<(), DxvkError> {
        log::trace!("D3D9: DrawIndexedPrimitive");
        Ok(())
    }
}

/// D3D9 primitive types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3D9PrimitiveType {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    TriangleFan,
}

/// D3D9 vertex shader
pub struct D3D9VertexShader {
    /// Shader bytecode
    _bytecode: Vec<u8>,
}

impl D3D9VertexShader {
    /// Create from bytecode
    pub fn from_bytecode(bytecode: Vec<u8>) -> Result<Self, DxvkError> {
        log::debug!("Creating D3D9 vertex shader");
        Ok(Self {
            _bytecode: bytecode,
        })
    }
}

/// D3D9 pixel shader
pub struct D3D9PixelShader {
    /// Shader bytecode
    _bytecode: Vec<u8>,
}

impl D3D9PixelShader {
    /// Create from bytecode
    pub fn from_bytecode(bytecode: Vec<u8>) -> Result<Self, DxvkError> {
        log::debug!("Creating D3D9 pixel shader");
        Ok(Self {
            _bytecode: bytecode,
        })
    }
}
