//! VirtIO-GPU protocol definitions
//!
//! This module defines the VirtIO-GPU protocol structures and constants.

use core::mem;

/// VirtIO GPU command types
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandType {
    // 2D commands
    GetDisplayInfo = 0x0100,
    ResourceCreate2d,
    ResourceUnref,
    SetScanout,
    ResourceFlush,
    TransferToHost2d,
    ResourceAttachBacking,
    ResourceDetachBacking,
    GetCapsetInfo,
    GetCapset,
    GetEdid,
    ResourceAssignUuid,
    ResourceCreateBlob,
    SetScanoutBlob,

    // 3D commands
    CtxCreate = 0x0200,
    CtxDestroy,
    CtxAttachResource,
    CtxDetachResource,
    ResourceCreate3d,
    TransferToHost3d,
    TransferFromHost3d,
    Submit3d,
    ResourceMapBlob,
    ResourceUnmapBlob,

    // Cursor commands
    UpdateCursor = 0x0300,
    MoveCursor,

    // Success responses
    RespOkNodata = 0x1100,
    RespOkDisplayInfo,
    RespOkCapsetInfo,
    RespOkCapset,
    RespOkEdid,
    RespOkResourceUuid,
    RespOkMapInfo,

    // Error responses
    RespErrUnspec = 0x1200,
    RespErrOutOfMemory,
    RespErrInvalidScanoutId,
    RespErrInvalidResourceId,
    RespErrInvalidContextId,
    RespErrInvalidParameter,
}

/// Control header for all VirtIO GPU commands
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ControlHeader {
    pub cmd_type: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub ring_idx: u8,
    pub padding: [u8; 3],
}

impl ControlHeader {
    pub const FLAG_FENCE: u32 = 1 << 0;
    pub const FLAG_INFO_RING_IDX: u32 = 1 << 1;

    pub fn new(cmd_type: CommandType) -> Self {
        Self {
            cmd_type: cmd_type as u32,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0; 3],
        }
    }

    pub fn with_fence(mut self, fence_id: u64) -> Self {
        self.flags |= Self::FLAG_FENCE;
        self.fence_id = fence_id;
        self
    }

    pub fn with_context(mut self, ctx_id: u32) -> Self {
        self.ctx_id = ctx_id;
        self
    }
}

/// Display info response
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DisplayInfo {
    pub rect: Rect,
    pub enabled: u32,
    pub flags: u32,
}

/// Rectangle
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Resource create 2D request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResourceCreate2d {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
}

impl ResourceCreate2d {
    pub fn new(resource_id: u32, format: ResourceFormat, width: u32, height: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::ResourceCreate2d),
            resource_id,
            format: format as u32,
            width,
            height,
        }
    }
}

/// Resource format (2D)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResourceFormat {
    B8G8R8A8Unorm = 1,
    B8G8R8X8Unorm = 2,
    A8R8G8B8Unorm = 3,
    X8R8G8B8Unorm = 4,
    R8G8B8A8Unorm = 67,
    X8B8G8R8Unorm = 68,
    A8B8G8R8Unorm = 121,
    R8G8B8X8Unorm = 134,
}

/// Resource unref request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResourceUnref {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub padding: u32,
}

impl ResourceUnref {
    pub fn new(resource_id: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::ResourceUnref),
            resource_id,
            padding: 0,
        }
    }
}

/// Set scanout request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetScanout {
    pub header: ControlHeader,
    pub rect: Rect,
    pub scanout_id: u32,
    pub resource_id: u32,
}

impl SetScanout {
    pub fn new(scanout_id: u32, resource_id: u32, rect: Rect) -> Self {
        Self {
            header: ControlHeader::new(CommandType::SetScanout),
            rect,
            scanout_id,
            resource_id,
        }
    }
}

/// Resource flush request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResourceFlush {
    pub header: ControlHeader,
    pub rect: Rect,
    pub resource_id: u32,
    pub padding: u32,
}

impl ResourceFlush {
    pub fn new(resource_id: u32, rect: Rect) -> Self {
        Self {
            header: ControlHeader::new(CommandType::ResourceFlush),
            rect,
            resource_id,
            padding: 0,
        }
    }
}

/// Transfer to host 2D request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TransferToHost2d {
    pub header: ControlHeader,
    pub rect: Rect,
    pub offset: u64,
    pub resource_id: u32,
    pub padding: u32,
}

impl TransferToHost2d {
    pub fn new(resource_id: u32, rect: Rect, offset: u64) -> Self {
        Self {
            header: ControlHeader::new(CommandType::TransferToHost2d),
            rect,
            offset,
            resource_id,
            padding: 0,
        }
    }
}

/// Attach backing request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AttachBacking {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub nr_entries: u32,
}

impl AttachBacking {
    pub fn new(resource_id: u32, nr_entries: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::ResourceAttachBacking),
            resource_id,
            nr_entries,
        }
    }
}

/// Memory entry for attach backing
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemEntry {
    pub addr: u64,
    pub length: u32,
    pub padding: u32,
}

impl MemEntry {
    pub fn new(addr: u64, length: u32) -> Self {
        Self {
            addr,
            length,
            padding: 0,
        }
    }
}

/// Detach backing request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DetachBacking {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub padding: u32,
}

impl DetachBacking {
    pub fn new(resource_id: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::ResourceDetachBacking),
            resource_id,
            padding: 0,
        }
    }
}

/// Get capset info request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetCapsetInfo {
    pub header: ControlHeader,
    pub capset_index: u32,
    pub padding: u32,
}

impl GetCapsetInfo {
    pub fn new(capset_index: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::GetCapsetInfo),
            capset_index,
            padding: 0,
        }
    }
}

/// Capset info response
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RespCapsetInfo {
    pub header: ControlHeader,
    pub capset_id: u32,
    pub capset_max_version: u32,
    pub capset_max_size: u32,
    pub padding: u32,
}

/// Capset types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CapsetType {
    /// virgl (OpenGL)
    Virgl = 1,
    /// virgl2 (improved virgl)
    Virgl2 = 2,
    /// Venus (Vulkan)
    Venus = 3,
    /// Cross-domain (ChromeOS)
    CrossDomain = 4,
}

/// 3D context create request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CtxCreate {
    pub header: ControlHeader,
    pub nlen: u32,
    pub context_init: u32,
    pub debug_name: [u8; 64],
}

impl CtxCreate {
    pub fn new(ctx_id: u32, context_init: u32, name: &[u8]) -> Self {
        let mut debug_name = [0u8; 64];
        let len = name.len().min(64);
        debug_name[..len].copy_from_slice(&name[..len]);

        Self {
            header: ControlHeader::new(CommandType::CtxCreate).with_context(ctx_id),
            nlen: len as u32,
            context_init,
            debug_name,
        }
    }
}

/// Context init flags for Venus
pub mod ctx_init_flags {
    pub const CAPSET_ID_MASK: u32 = 0x000000FF;
    pub const POLL_RINGS_MASK: u32 = 0x00000100;
}

/// 3D context destroy request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CtxDestroy {
    pub header: ControlHeader,
}

impl CtxDestroy {
    pub fn new(ctx_id: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::CtxDestroy).with_context(ctx_id),
        }
    }
}

/// 3D command submit request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Submit3d {
    pub header: ControlHeader,
    pub size: u32,
    pub padding: u32,
    // Followed by command stream data
}

impl Submit3d {
    pub fn new(ctx_id: u32, size: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::Submit3d).with_context(ctx_id),
            size,
            padding: 0,
        }
    }
}

/// 3D resource create request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResourceCreate3d {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub target: u32,
    pub format: u32,
    pub bind: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub array_size: u32,
    pub last_level: u32,
    pub nr_samples: u32,
    pub flags: u32,
    pub padding: u32,
}

impl ResourceCreate3d {
    pub fn new(resource_id: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::ResourceCreate3d),
            resource_id,
            target: 0,
            format: 0,
            bind: 0,
            width: 0,
            height: 0,
            depth: 0,
            array_size: 0,
            last_level: 0,
            nr_samples: 0,
            flags: 0,
            padding: 0,
        }
    }
}

/// Transfer to host 3D request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TransferToHost3d {
    pub header: ControlHeader,
    pub box_: Box3d,
    pub offset: u64,
    pub resource_id: u32,
    pub level: u32,
    pub stride: u32,
    pub layer_stride: u32,
}

/// 3D box
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Box3d {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
    pub h: u32,
    pub d: u32,
}

/// Transfer from host 3D request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TransferFromHost3d {
    pub header: ControlHeader,
    pub box_: Box3d,
    pub offset: u64,
    pub resource_id: u32,
    pub level: u32,
    pub stride: u32,
    pub layer_stride: u32,
}

/// Attach resource to context
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CtxAttachResource {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub padding: u32,
}

impl CtxAttachResource {
    pub fn new(ctx_id: u32, resource_id: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::CtxAttachResource).with_context(ctx_id),
            resource_id,
            padding: 0,
        }
    }
}

/// Detach resource from context
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CtxDetachResource {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub padding: u32,
}

impl CtxDetachResource {
    pub fn new(ctx_id: u32, resource_id: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::CtxDetachResource).with_context(ctx_id),
            resource_id,
            padding: 0,
        }
    }
}

/// Blob resource create request (for host-visible memory)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResourceCreateBlob {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub blob_mem: u32,
    pub blob_flags: u32,
    pub nr_entries: u32,
    pub blob_id: u64,
    pub size: u64,
}

/// Blob memory types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BlobMem {
    Guest = 1,
    Host3d = 2,
    Host3dGuest = 3,
}

/// Blob flags
pub mod blob_flags {
    /// Blob can be mapped for read
    pub const MAPPABLE: u32 = 1 << 0;
    /// Blob can be shared between contexts
    pub const SHAREABLE: u32 = 1 << 1;
    /// Blob can be used as cross-device import
    pub const CROSS_DEVICE: u32 = 1 << 2;
}

/// Map blob resource request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResourceMapBlob {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub padding: u32,
    pub offset: u64,
}

/// Map info response
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RespMapInfo {
    pub header: ControlHeader,
    pub map_info: u32,
    pub padding: u32,
}

/// Unmap blob resource request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ResourceUnmapBlob {
    pub header: ControlHeader,
    pub resource_id: u32,
    pub padding: u32,
}

/// Cursor position
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CursorPos {
    pub scanout_id: u32,
    pub x: u32,
    pub y: u32,
    pub padding: u32,
}

/// Update cursor request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UpdateCursor {
    pub header: ControlHeader,
    pub pos: CursorPos,
    pub resource_id: u32,
    pub hot_x: u32,
    pub hot_y: u32,
    pub padding: u32,
}

impl UpdateCursor {
    pub fn new(scanout_id: u32, x: u32, y: u32, resource_id: u32, hot_x: u32, hot_y: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::UpdateCursor),
            pos: CursorPos {
                scanout_id,
                x,
                y,
                padding: 0,
            },
            resource_id,
            hot_x,
            hot_y,
            padding: 0,
        }
    }
}

/// Move cursor request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MoveCursor {
    pub header: ControlHeader,
    pub pos: CursorPos,
    pub padding: [u32; 3],
}

impl MoveCursor {
    pub fn new(scanout_id: u32, x: u32, y: u32) -> Self {
        Self {
            header: ControlHeader::new(CommandType::MoveCursor),
            pos: CursorPos {
                scanout_id,
                x,
                y,
                padding: 0,
            },
            padding: [0; 3],
        }
    }
}

/// Maximum scanouts supported
pub const MAX_SCANOUTS: usize = 16;

/// Display info response (full)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetDisplayInfoResponse {
    pub header: ControlHeader,
    pub pmodes: [DisplayInfo; MAX_SCANOUTS],
}
