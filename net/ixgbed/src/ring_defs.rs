use std::sync::atomic::{AtomicU32, AtomicU64};

#[repr(C)]
pub struct IpcRing {
    pub sq_head: AtomicU32,
    pub sq_tail: AtomicU32,
    pub sq_mask: u32,
    pub sq_entries: u32,
    pub cq_head: AtomicU32,
    pub cq_tail: AtomicU32,
    pub cq_mask: u32,
    pub cq_entries: u32,
    pub sq_flags: AtomicU32,
    pub cq_flags: AtomicU32,
    pub features: u32,
    pub cq_overflow: AtomicU64,
    pub _reserved: [u32; 4],
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Sqe {
    pub opcode: u8,
    pub flags: u8,
    pub ioprio: u16,
    pub fd: i32,
    pub off: u64,
    pub addr: u64,
    pub len: u32,
    pub rw_flags: u32,
    pub user_data: u64,
    pub buf_index: u16,
    pub personality: u16,
    pub splice_fd: i32,
    pub addr3: u64,
    pub _pad: [u64; 1],
}

pub const IORING_OP_RDMA_READ: u8 = 16;
pub const IORING_OP_RDMA_WRITE: u8 = 17;
