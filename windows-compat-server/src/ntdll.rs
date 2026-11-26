//! NTDLL Emulation
//!
//! Provides emulation of key NTDLL functions and structures.

/// Windows UNICODE_STRING structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UnicodeString {
    /// Length of the string in bytes (not including null)
    pub length: u16,
    /// Maximum length of the buffer in bytes
    pub maximum_length: u16,
    /// Pointer to the buffer
    pub buffer: *mut u16,
}

impl UnicodeString {
    pub const fn new() -> Self {
        Self {
            length: 0,
            maximum_length: 0,
            buffer: core::ptr::null_mut(),
        }
    }
}

/// Windows OBJECT_ATTRIBUTES structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObjectAttributes {
    pub length: u32,
    pub root_directory: usize, // HANDLE
    pub object_name: *const UnicodeString,
    pub attributes: u32,
    pub security_descriptor: *const (),
    pub security_qos: *const (),
}

/// Object attribute flags
pub mod obj_flags {
    pub const OBJ_INHERIT: u32 = 0x00000002;
    pub const OBJ_PERMANENT: u32 = 0x00000010;
    pub const OBJ_EXCLUSIVE: u32 = 0x00000020;
    pub const OBJ_CASE_INSENSITIVE: u32 = 0x00000040;
    pub const OBJ_OPENIF: u32 = 0x00000080;
    pub const OBJ_OPENLINK: u32 = 0x00000100;
    pub const OBJ_KERNEL_HANDLE: u32 = 0x00000200;
    pub const OBJ_FORCE_ACCESS_CHECK: u32 = 0x00000400;
}

/// Windows IO_STATUS_BLOCK structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IoStatusBlock {
    pub status: u32,        // NTSTATUS or PVOID (union)
    pub information: usize, // Bytes transferred or other info
}

impl IoStatusBlock {
    pub const fn new() -> Self {
        Self {
            status: 0,
            information: 0,
        }
    }
}

/// Windows LARGE_INTEGER (64-bit signed)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub union LargeInteger {
    pub quad_part: i64,
    pub parts: LargeIntegerParts,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LargeIntegerParts {
    pub low_part: u32,
    pub high_part: i32,
}

/// File access rights
pub mod file_access {
    pub const FILE_READ_DATA: u32 = 0x0001;
    pub const FILE_LIST_DIRECTORY: u32 = 0x0001;
    pub const FILE_WRITE_DATA: u32 = 0x0002;
    pub const FILE_ADD_FILE: u32 = 0x0002;
    pub const FILE_APPEND_DATA: u32 = 0x0004;
    pub const FILE_ADD_SUBDIRECTORY: u32 = 0x0004;
    pub const FILE_READ_EA: u32 = 0x0008;
    pub const FILE_WRITE_EA: u32 = 0x0010;
    pub const FILE_EXECUTE: u32 = 0x0020;
    pub const FILE_TRAVERSE: u32 = 0x0020;
    pub const FILE_DELETE_CHILD: u32 = 0x0040;
    pub const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    pub const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
    pub const FILE_ALL_ACCESS: u32 = 0x001F01FF;
    pub const FILE_GENERIC_READ: u32 = 0x00120089;
    pub const FILE_GENERIC_WRITE: u32 = 0x00120116;
    pub const FILE_GENERIC_EXECUTE: u32 = 0x001200A0;
}

/// File share mode
pub mod file_share {
    pub const FILE_SHARE_READ: u32 = 0x00000001;
    pub const FILE_SHARE_WRITE: u32 = 0x00000002;
    pub const FILE_SHARE_DELETE: u32 = 0x00000004;
}

/// File creation disposition
pub mod file_disposition {
    pub const FILE_SUPERSEDE: u32 = 0x00000000;
    pub const FILE_OPEN: u32 = 0x00000001;
    pub const FILE_CREATE: u32 = 0x00000002;
    pub const FILE_OPEN_IF: u32 = 0x00000003;
    pub const FILE_OVERWRITE: u32 = 0x00000004;
    pub const FILE_OVERWRITE_IF: u32 = 0x00000005;
}

/// File options
pub mod file_options {
    pub const FILE_DIRECTORY_FILE: u32 = 0x00000001;
    pub const FILE_WRITE_THROUGH: u32 = 0x00000002;
    pub const FILE_SEQUENTIAL_ONLY: u32 = 0x00000004;
    pub const FILE_NO_INTERMEDIATE_BUFFERING: u32 = 0x00000008;
    pub const FILE_SYNCHRONOUS_IO_ALERT: u32 = 0x00000010;
    pub const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x00000020;
    pub const FILE_NON_DIRECTORY_FILE: u32 = 0x00000040;
    pub const FILE_CREATE_TREE_CONNECTION: u32 = 0x00000080;
    pub const FILE_COMPLETE_IF_OPLOCKED: u32 = 0x00000100;
    pub const FILE_NO_EA_KNOWLEDGE: u32 = 0x00000200;
    pub const FILE_RANDOM_ACCESS: u32 = 0x00000800;
    pub const FILE_DELETE_ON_CLOSE: u32 = 0x00001000;
    pub const FILE_OPEN_BY_FILE_ID: u32 = 0x00002000;
}

/// Memory protection constants
pub mod mem_protect {
    pub const PAGE_NOACCESS: u32 = 0x01;
    pub const PAGE_READONLY: u32 = 0x02;
    pub const PAGE_READWRITE: u32 = 0x04;
    pub const PAGE_WRITECOPY: u32 = 0x08;
    pub const PAGE_EXECUTE: u32 = 0x10;
    pub const PAGE_EXECUTE_READ: u32 = 0x20;
    pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;
    pub const PAGE_EXECUTE_WRITECOPY: u32 = 0x80;
    pub const PAGE_GUARD: u32 = 0x100;
    pub const PAGE_NOCACHE: u32 = 0x200;
    pub const PAGE_WRITECOMBINE: u32 = 0x400;
}

/// Memory allocation types
pub mod mem_alloc {
    pub const MEM_COMMIT: u32 = 0x1000;
    pub const MEM_RESERVE: u32 = 0x2000;
    pub const MEM_DECOMMIT: u32 = 0x4000;
    pub const MEM_RELEASE: u32 = 0x8000;
    pub const MEM_FREE: u32 = 0x10000;
    pub const MEM_PRIVATE: u32 = 0x20000;
    pub const MEM_MAPPED: u32 = 0x40000;
    pub const MEM_RESET: u32 = 0x80000;
    pub const MEM_TOP_DOWN: u32 = 0x100000;
    pub const MEM_LARGE_PAGES: u32 = 0x20000000;
    pub const MEM_4MB_PAGES: u32 = 0x80000000;
}
