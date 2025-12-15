//! Process management for Linux compatibility
//!
//! This module manages processes running under the LAC layer.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use crate::elf_loader::LoadedElf;
use crate::errno::LinuxErrno;
use crate::signal::SignalState;

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is being created
    Creating,
    /// Process is ready to run
    Ready,
    /// Process is running
    Running,
    /// Process is blocked waiting for something
    Blocked,
    /// Process is stopped (e.g., by SIGSTOP)
    Stopped,
    /// Process has exited
    Zombie,
    /// Process has been fully cleaned up
    Dead,
}

/// Process descriptor
pub struct Process {
    /// Process ID
    pid: u32,
    /// Parent process ID
    ppid: AtomicU32,
    /// Thread group ID (same as pid for main thread)
    tgid: u32,
    /// Process state
    state: spin::RwLock<ProcessState>,
    /// Executable path
    exe_path: String,
    /// Command line arguments
    args: spin::RwLock<Vec<String>>,
    /// Environment variables
    env: spin::RwLock<Vec<String>>,
    /// Current working directory
    cwd: spin::RwLock<String>,
    /// User ID
    uid: AtomicU32,
    /// Effective user ID
    euid: AtomicU32,
    /// Group ID
    gid: AtomicU32,
    /// Effective group ID
    egid: AtomicU32,
    /// Exit status
    exit_status: spin::RwLock<Option<i32>>,
    /// Memory regions
    memory: spin::RwLock<MemoryMap>,
    /// Signal state
    signals: spin::RwLock<SignalState>,
    /// Threads in this process
    threads: spin::RwLock<Vec<Arc<Thread>>>,
    /// File descriptor table
    fd_table: spin::RwLock<FdTable>,
    /// Creation time (nanoseconds since boot)
    start_time: u64,
    /// CPU time used (nanoseconds)
    cpu_time: AtomicU64,
}

/// Thread within a process
pub struct Thread {
    /// Thread ID
    tid: u32,
    /// Thread state
    state: spin::RwLock<ThreadState>,
    /// Thread-local storage pointer
    tls_ptr: AtomicU64,
    /// Set on thread exit
    clear_child_tid: AtomicU64,
    /// Register state
    registers: spin::RwLock<RegisterState>,
}

/// Thread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Running,
    Interruptible,
    Uninterruptible,
    Stopped,
    Zombie,
}

/// CPU register state (x86_64)
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct RegisterState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u64,
    pub ss: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
    pub fs_base: u64,
    pub gs_base: u64,
}

/// Memory mapping for a process
pub struct MemoryMap {
    /// Memory regions
    regions: Vec<MemoryRegion>,
    /// Program break (brk)
    brk: u64,
    /// Initial brk
    start_brk: u64,
    /// Stack top
    stack_top: u64,
    /// Stack bottom
    stack_bottom: u64,
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self {
            regions: Vec::new(),
            brk: 0,
            start_brk: 0,
            stack_top: 0,
            stack_bottom: 0,
        }
    }
}

/// Memory region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Start address
    pub start: u64,
    /// End address
    pub end: u64,
    /// Protection flags
    pub prot: u32,
    /// Flags
    pub flags: u32,
    /// File offset (if backed by file)
    pub offset: u64,
    /// File path (if backed by file)
    pub path: Option<String>,
}

/// Memory protection flags
pub mod prot_flags {
    pub const PROT_NONE: u32 = 0;
    pub const PROT_READ: u32 = 1;
    pub const PROT_WRITE: u32 = 2;
    pub const PROT_EXEC: u32 = 4;
}

/// Memory mapping flags
pub mod map_flags {
    pub const MAP_SHARED: u32 = 0x01;
    pub const MAP_PRIVATE: u32 = 0x02;
    pub const MAP_FIXED: u32 = 0x10;
    pub const MAP_ANONYMOUS: u32 = 0x20;
    pub const MAP_GROWSDOWN: u32 = 0x100;
    pub const MAP_DENYWRITE: u32 = 0x800;
    pub const MAP_EXECUTABLE: u32 = 0x1000;
    pub const MAP_LOCKED: u32 = 0x2000;
    pub const MAP_NORESERVE: u32 = 0x4000;
    pub const MAP_POPULATE: u32 = 0x8000;
    pub const MAP_NONBLOCK: u32 = 0x10000;
    pub const MAP_STACK: u32 = 0x20000;
    pub const MAP_HUGETLB: u32 = 0x40000;
}

/// File descriptor table
pub struct FdTable {
    /// Open file descriptors
    files: HashMap<i32, FileDescriptor>,
    /// Next available fd
    next_fd: i32,
    /// Close-on-exec flags
    cloexec: HashMap<i32, bool>,
}

impl Default for FdTable {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
            next_fd: 3, // 0, 1, 2 are stdin, stdout, stderr
            cloexec: HashMap::new(),
        }
    }
}

/// File descriptor
pub struct FileDescriptor {
    /// Redox file handle (or scheme path)
    pub path: String,
    /// Open flags
    pub flags: i32,
    /// Current offset
    pub offset: u64,
}

impl Process {
    /// Create a new process
    pub fn new(pid: u32, exe_path: String) -> Self {
        Self {
            pid,
            ppid: AtomicU32::new(1), // Init is parent
            tgid: pid,
            state: spin::RwLock::new(ProcessState::Creating),
            exe_path,
            args: spin::RwLock::new(Vec::new()),
            env: spin::RwLock::new(Vec::new()),
            cwd: spin::RwLock::new("/".to_string()),
            uid: AtomicU32::new(1000),
            euid: AtomicU32::new(1000),
            gid: AtomicU32::new(1000),
            egid: AtomicU32::new(1000),
            exit_status: spin::RwLock::new(None),
            memory: spin::RwLock::new(MemoryMap::default()),
            signals: spin::RwLock::new(SignalState::default()),
            threads: spin::RwLock::new(Vec::new()),
            fd_table: spin::RwLock::new(FdTable::default()),
            start_time: 0, // Would be set to current time
            cpu_time: AtomicU64::new(0),
        }
    }

    /// Get process ID
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Get parent process ID
    pub fn ppid(&self) -> u32 {
        self.ppid.load(Ordering::SeqCst)
    }

    /// Get thread group ID
    pub fn tgid(&self) -> u32 {
        self.tgid
    }

    /// Get process state
    pub fn state(&self) -> ProcessState {
        *self.state.read()
    }

    /// Set process state
    pub fn set_state(&self, state: ProcessState) {
        *self.state.write() = state;
    }

    /// Get executable path
    pub fn exe_path(&self) -> &str {
        &self.exe_path
    }

    /// Get current working directory
    pub fn cwd(&self) -> String {
        self.cwd.read().clone()
    }

    /// Set current working directory
    pub fn set_cwd(&self, cwd: String) {
        *self.cwd.write() = cwd;
    }

    /// Get user ID
    pub fn uid(&self) -> u32 {
        self.uid.load(Ordering::SeqCst)
    }

    /// Get effective user ID
    pub fn euid(&self) -> u32 {
        self.euid.load(Ordering::SeqCst)
    }

    /// Get group ID
    pub fn gid(&self) -> u32 {
        self.gid.load(Ordering::SeqCst)
    }

    /// Get effective group ID
    pub fn egid(&self) -> u32 {
        self.egid.load(Ordering::SeqCst)
    }

    /// Set up memory from ELF
    pub fn setup_memory(&self, elf: &LoadedElf) -> Result<(), LinuxErrno> {
        let mut memory = self.memory.write();

        // Map program segments
        for phdr in &elf.program_headers {
            if phdr.p_type == crate::elf_loader::pt_type::PT_LOAD {
                let prot = elf_flags_to_prot(phdr.p_flags);

                memory.regions.push(MemoryRegion {
                    start: phdr.p_vaddr,
                    end: phdr.p_vaddr + phdr.p_memsz,
                    prot,
                    flags: map_flags::MAP_PRIVATE,
                    offset: phdr.p_offset,
                    path: Some(self.exe_path.clone()),
                });
            }
        }

        // Set up brk after the last segment
        let end_addr = memory.regions.iter().map(|r| r.end).max().unwrap_or(0);

        let brk_start = (end_addr + 0xFFF) & !0xFFF; // Page align
        memory.start_brk = brk_start;
        memory.brk = brk_start;

        Ok(())
    }

    /// Set up stack with arguments and environment
    pub fn setup_stack(&self, args: &[String], env: &[String]) -> Result<(), LinuxErrno> {
        *self.args.write() = args.to_vec();
        *self.env.write() = env.to_vec();

        let mut memory = self.memory.write();

        // Set up stack (grows down)
        let stack_size = 8 * 1024 * 1024; // 8 MB
        let stack_top = 0x7fff_ffff_f000u64;
        let stack_bottom = stack_top - stack_size as u64;

        memory.stack_top = stack_top;
        memory.stack_bottom = stack_bottom;

        memory.regions.push(MemoryRegion {
            start: stack_bottom,
            end: stack_top,
            prot: prot_flags::PROT_READ | prot_flags::PROT_WRITE,
            flags: map_flags::MAP_PRIVATE | map_flags::MAP_GROWSDOWN | map_flags::MAP_STACK,
            offset: 0,
            path: Some("[stack]".to_string()),
        });

        Ok(())
    }

    /// Start process execution
    pub fn start(&self, entry_point: u64) -> Result<(), LinuxErrno> {
        self.set_state(ProcessState::Ready);

        // Create main thread
        let main_thread = Arc::new(Thread::new(self.pid));

        // Set up initial registers
        {
            let mut regs = main_thread.registers.write();
            regs.rip = entry_point;

            let memory = self.memory.read();
            regs.rsp = memory.stack_top - 8; // Leave space for return address
        }

        self.threads.write().push(main_thread);

        self.set_state(ProcessState::Running);

        Ok(())
    }

    /// Exit the process
    pub fn exit(&self, status: i32) {
        *self.exit_status.write() = Some(status);
        self.set_state(ProcessState::Zombie);
    }

    /// Get exit status
    pub fn exit_status(&self) -> Option<i32> {
        *self.exit_status.read()
    }

    /// Add a memory region
    pub fn add_memory_region(&self, region: MemoryRegion) {
        self.memory.write().regions.push(region);
    }

    /// Set program break
    pub fn set_brk(&self, new_brk: u64) -> u64 {
        let mut memory = self.memory.write();

        if new_brk >= memory.start_brk {
            memory.brk = new_brk;
        }

        memory.brk
    }

    /// Get program break
    pub fn brk(&self) -> u64 {
        self.memory.read().brk
    }

    /// Allocate a file descriptor
    pub fn alloc_fd(&self, path: String, flags: i32) -> i32 {
        let mut fd_table = self.fd_table.write();
        let fd = fd_table.next_fd;
        fd_table.next_fd += 1;

        fd_table.files.insert(
            fd,
            FileDescriptor {
                path,
                flags,
                offset: 0,
            },
        );

        fd
    }

    /// Close a file descriptor
    pub fn close_fd(&self, fd: i32) -> Result<(), LinuxErrno> {
        let mut fd_table = self.fd_table.write();

        if fd_table.files.remove(&fd).is_some() {
            fd_table.cloexec.remove(&fd);
            Ok(())
        } else {
            Err(LinuxErrno::EBADF)
        }
    }

    /// Duplicate a file descriptor
    pub fn dup_fd(&self, oldfd: i32) -> Result<i32, LinuxErrno> {
        let mut fd_table = self.fd_table.write();

        if let Some(old_file) = fd_table.files.get(&oldfd) {
            let newfd = fd_table.next_fd;
            fd_table.next_fd += 1;

            fd_table.files.insert(
                newfd,
                FileDescriptor {
                    path: old_file.path.clone(),
                    flags: old_file.flags,
                    offset: old_file.offset,
                },
            );

            Ok(newfd)
        } else {
            Err(LinuxErrno::EBADF)
        }
    }
}

impl Thread {
    /// Create a new thread
    pub fn new(tid: u32) -> Self {
        Self {
            tid,
            state: spin::RwLock::new(ThreadState::Running),
            tls_ptr: AtomicU64::new(0),
            clear_child_tid: AtomicU64::new(0),
            registers: spin::RwLock::new(RegisterState::default()),
        }
    }

    /// Get thread ID
    pub fn tid(&self) -> u32 {
        self.tid
    }

    /// Get thread state
    pub fn state(&self) -> ThreadState {
        *self.state.read()
    }

    /// Set thread state
    pub fn set_state(&self, state: ThreadState) {
        *self.state.write() = state;
    }

    /// Get TLS pointer
    pub fn tls_ptr(&self) -> u64 {
        self.tls_ptr.load(Ordering::SeqCst)
    }

    /// Set TLS pointer
    pub fn set_tls_ptr(&self, ptr: u64) {
        self.tls_ptr.store(ptr, Ordering::SeqCst);
    }

    /// Get register state
    pub fn registers(&self) -> RegisterState {
        self.registers.read().clone()
    }

    /// Set register state
    pub fn set_registers(&self, regs: RegisterState) {
        *self.registers.write() = regs;
    }
}

/// Convert ELF flags to protection flags
fn elf_flags_to_prot(elf_flags: u32) -> u32 {
    let mut prot = 0;

    if elf_flags & crate::elf_loader::pf_flags::PF_R != 0 {
        prot |= prot_flags::PROT_READ;
    }
    if elf_flags & crate::elf_loader::pf_flags::PF_W != 0 {
        prot |= prot_flags::PROT_WRITE;
    }
    if elf_flags & crate::elf_loader::pf_flags::PF_X != 0 {
        prot |= prot_flags::PROT_EXEC;
    }

    prot
}
