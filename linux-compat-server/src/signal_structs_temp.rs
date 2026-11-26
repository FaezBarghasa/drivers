/// Floating point state
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FpState {
    pub cwd: u16,
    pub swd: u16,
    pub ftw: u16,
    pub fop: u16,
    pub rip: u64,
    pub rdp: u64,
    pub mxcsr: u32,
    pub mxcsr_mask: u32,
    pub st_space: [u32; 32],
    pub xmm_space: [u32; 64],
    pub reserved2: [u32; 24],
}

/// Machine context (registers)
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct MContext {
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rsp: u64,
    pub rip: u64,
    pub eflags: u64,
    pub cs: u16,
    pub gs: u16,
    pub fs: u16,
    pub __pad0: u16,
    pub err: u64,
    pub trapno: u64,
    pub oldmask: u64,
    pub cr2: u64,
    pub fpstate: *mut FpState,
    pub __reserved1: [u64; 8],
}

/// Stack descriptor
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct StackT {
    pub ss_sp: *mut std::ffi::c_void,
    pub ss_flags: i32,
    pub ss_size: usize,
}

/// User context
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct UContext {
    pub uc_flags: [u64; 5], // unsigned long uc_flags; + padding ? Linux definition varies.
    // Actually it is: unsigned long uc_flags; struct ucontext *uc_link; stack_t uc_stack; mcontext_t uc_mcontext; sigset_t uc_sigmask;
    // On x86_64:
    // unsigned long uc_flags;
    // struct ucontext *uc_link;
    // stack_t uc_stack;
    // struct sigcontext uc_mcontext;
    // sigset_t uc_sigmask;
    // struct _libc_fpstate __fpregs_mem;
    pub uc_link: *mut UContext,
    pub uc_stack: StackT,
    pub uc_mcontext: MContext,
    pub uc_sigmask: u64,
    pub __fpregs_mem: FpState,
}
