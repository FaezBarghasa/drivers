//! Syscall translator
//!
//! This module translates Linux syscalls to Redox equivalents.

use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::errno::LinuxErrno;
use crate::process::Process;
use crate::signal::{SigAction, Signal, SignalHandler, UContext};
use crate::syscall_table::LinuxSyscall;
use redox_syscall::{self, syscall5, syscall6, SYS_FUTEX};
use std::sync::Arc;

/// Syscall context containing all registers
#[derive(Debug, Clone, Default)]
pub struct SyscallContext {
    /// Syscall number
    pub syscall_num: u64,
    /// First argument (rdi)
    pub arg0: u64,
    /// Second argument (rsi)
    pub arg1: u64,
    /// Third argument (rdx)
    pub arg2: u64,
    /// Fourth argument (r10)
    pub arg3: u64,
    /// Fifth argument (r8)
    pub arg4: u64,
    /// Sixth argument (r9)
    pub arg5: u64,
    /// Instruction pointer
    pub rip: u64,
    /// Stack pointer
    pub rsp: u64,
}

impl SyscallContext {
    /// Get the Linux syscall enum
    pub fn syscall(&self) -> LinuxSyscall {
        LinuxSyscall::from_number(self.syscall_num)
    }
}

/// Syscall result
#[derive(Debug)]
pub enum SyscallResult {
    /// Success with return value
    Success(i64),
    /// Error with errno
    Error(LinuxErrno),
    /// Syscall not yet implemented
    NotImplemented,
}

impl SyscallResult {
    /// Convert from Redox syscall error
    pub fn from_error(err: redox_syscall::Error) -> Self {
        Self::Error(LinuxErrno::from_redox(err.errno as usize))
    }

    /// Convert to raw return value (Linux convention: negative for error)
    pub fn to_raw(&self) -> i64 {
        match self {
            Self::Success(val) => *val,
            Self::Error(errno) => -(*errno as i32 as i64),
            Self::NotImplemented => -(LinuxErrno::ENOSYS as i32 as i64),
        }
    }
}

/// Syscall translator
pub struct SyscallTranslator {
    /// Path mappings (Linux path → Redox path)
    path_mappings: HashMap<String, String>,
    /// Open file descriptors (Linux fd → Redox file)
    fds: spin::RwLock<HashMap<i32, FileDescriptor>>,
    /// Next available fd
    next_fd: std::sync::atomic::AtomicI32,
}

/// Backing for a file descriptor
#[derive(Debug, Clone)]
pub enum FileBacking {
    Native(std::sync::Arc<File>),
    EventFd(std::sync::Arc<std::sync::Mutex<u64>>),
}

/// File descriptor wrapper
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    /// Backing object
    backing: Option<FileBacking>,
    /// File path
    path: String,
    /// File flags
    flags: i32,
    /// Is a pipe
    is_pipe: bool,
}

/// Linux open flags
pub mod open_flags {
    pub const O_RDONLY: i32 = 0;
    pub const O_WRONLY: i32 = 1;
    pub const O_RDWR: i32 = 2;
    pub const O_ACCMODE: i32 = 3;
    pub const O_CREAT: i32 = 0o100;
    pub const O_EXCL: i32 = 0o200;
    pub const O_NOCTTY: i32 = 0o400;
    pub const O_TRUNC: i32 = 0o1000;
    pub const O_APPEND: i32 = 0o2000;
    pub const O_NONBLOCK: i32 = 0o4000;
    pub const O_DSYNC: i32 = 0o10000;
    pub const O_SYNC: i32 = 0o4010000;
    pub const O_RSYNC: i32 = 0o4010000;
    pub const O_DIRECTORY: i32 = 0o200000;
    pub const O_NOFOLLOW: i32 = 0o400000;
    pub const O_CLOEXEC: i32 = 0o2000000;
    pub const O_ASYNC: i32 = 0o20000;
    pub const O_DIRECT: i32 = 0o40000;
    pub const O_LARGEFILE: i32 = 0o100000;
    pub const O_NOATIME: i32 = 0o1000000;
    pub const O_PATH: i32 = 0o10000000;
    pub const O_TMPFILE: i32 = 0o20200000;
}

/// Linux seek whence values
pub mod seek_whence {
    pub const SEEK_SET: i32 = 0;
    pub const SEEK_CUR: i32 = 1;
    pub const SEEK_END: i32 = 2;
}

/// Linux access mode bits
pub mod access_mode {
    pub const F_OK: i32 = 0;
    pub const X_OK: i32 = 1;
    pub const W_OK: i32 = 2;
    pub const R_OK: i32 = 4;
}

/// AT_* constants for *at syscalls
pub mod at_flags {
    pub const AT_FDCWD: i32 = -100;
    pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
    pub const AT_REMOVEDIR: i32 = 0x200;
    pub const AT_SYMLINK_FOLLOW: i32 = 0x400;
    pub const AT_NO_AUTOMOUNT: i32 = 0x800;
    pub const AT_EMPTY_PATH: i32 = 0x1000;
}

impl SyscallTranslator {
    /// Create a new syscall translator
    pub fn new(path_mappings: HashMap<String, String>) -> Self {
        let mut fds = HashMap::new();

        // Set up standard file descriptors
        fds.insert(
            0,
            FileDescriptor {
                backing: Some(FileBacking::Native(std::sync::Arc::new(
                    File::open("/dev/stdin").unwrap_or_else(|_| {
                        // Fallback if /dev/stdin fails (e.g. running as daemon)
                        // For now just fail or use a dummy
                        // Creating a dummy file is hard.
                        // We assume /dev/stdin exists in Redox environment setup.
                        // But File::open expects path.
                        // If it fails, we should handle it.
                        // But we are in new(), returning Self.
                        panic!("Failed to open /dev/stdin")
                    }),
                ))),
                path: "/dev/stdin".to_string(),
                flags: open_flags::O_RDONLY,
                is_pipe: false,
            },
        );
        fds.insert(
            1,
            FileDescriptor {
                backing: Some(FileBacking::Native(std::sync::Arc::new(
                    File::create("/dev/stdout").expect("Failed to open stdout"), // create or open?
                                                                                 // stdout usually exists. OpenOptions?
                ))),

                path: "/dev/stdout".to_string(),
                flags: open_flags::O_WRONLY,
                is_pipe: false,
            },
        );

        fds.insert(
            2,
            FileDescriptor {
                backing: Some(FileBacking::Native(std::sync::Arc::new(
                    File::create("/dev/stderr").expect("Failed to open stderr"),
                ))),
                path: "/dev/stderr".to_string(),
                flags: open_flags::O_WRONLY,
                is_pipe: false,
            },
        );

        Self {
            path_mappings,
            fds: spin::RwLock::new(fds),
            next_fd: std::sync::atomic::AtomicI32::new(3),
        }
    }

    /// Translate a Linux path to Redox path
    pub fn translate_path(&self, linux_path: &str) -> String {
        // Check for exact matches first
        if let Some(redox_path) = self.path_mappings.get(linux_path) {
            return redox_path.clone();
        }

        // Check for prefix matches
        for (linux_prefix, redox_prefix) in &self.path_mappings {
            if linux_path.starts_with(linux_prefix) && linux_prefix != "/" {
                let suffix = &linux_path[linux_prefix.len()..];
                return format!("{}{}", redox_prefix, suffix);
            }
        }

        // Default: prepend file:
        format!("file:{}", linux_path)
    }

    /// Allocate a new file descriptor
    fn alloc_fd(&self) -> i32 {
        self.next_fd
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Translate and execute a syscall
    pub fn translate(&self, ctx: &SyscallContext, process: &Arc<crate::Process>) -> SyscallResult {
        let syscall = ctx.syscall();

        log::debug!(
            "Translating syscall: {} ({:#x}) from pid {}",
            syscall.name(),
            ctx.syscall_num,
            process.pid()
        );

        match syscall {
            // File I/O
            LinuxSyscall::Read => self.sys_read(ctx),
            LinuxSyscall::Write => self.sys_write(ctx),
            LinuxSyscall::Open => self.sys_open(ctx),
            LinuxSyscall::Openat => self.sys_openat(ctx),
            LinuxSyscall::Close => self.sys_close(ctx),
            LinuxSyscall::Lseek => self.sys_lseek(ctx),
            LinuxSyscall::Dup => self.sys_dup(ctx),
            LinuxSyscall::Dup2 => self.sys_dup2(ctx),
            LinuxSyscall::Dup3 => self.sys_dup3(ctx),
            LinuxSyscall::Pipe => self.sys_pipe(ctx),
            LinuxSyscall::Pipe2 => self.sys_pipe2(ctx),
            LinuxSyscall::Access => self.sys_access(ctx),
            LinuxSyscall::Faccessat => self.sys_faccessat(ctx),
            LinuxSyscall::Getcwd => self.sys_getcwd(ctx),
            LinuxSyscall::Chdir => self.sys_chdir(ctx),
            LinuxSyscall::Mkdir => self.sys_mkdir(ctx),
            LinuxSyscall::Rmdir => self.sys_rmdir(ctx),
            LinuxSyscall::Unlink => self.sys_unlink(ctx),
            LinuxSyscall::Unlinkat => self.sys_unlinkat(ctx),
            LinuxSyscall::Stat
            | LinuxSyscall::Fstat
            | LinuxSyscall::Lstat
            | LinuxSyscall::Newfstatat => self.sys_stat(ctx),
            LinuxSyscall::Getdents64 => self.sys_getdents64(ctx),
            LinuxSyscall::Kill => self.sys_kill(ctx, process),
            LinuxSyscall::RtSigaction => self.sys_rt_sigaction(ctx, process),

            // Process management
            LinuxSyscall::Getpid => self.sys_getpid(ctx),
            LinuxSyscall::Getppid => self.sys_getppid(ctx),
            LinuxSyscall::Gettid => self.sys_gettid(ctx),
            LinuxSyscall::Getuid | LinuxSyscall::Geteuid => self.sys_getuid(ctx),
            LinuxSyscall::Getgid | LinuxSyscall::Getegid => self.sys_getgid(ctx),
            LinuxSyscall::Exit | LinuxSyscall::ExitGroup => self.sys_exit(ctx),
            LinuxSyscall::Fork | LinuxSyscall::Vfork => self.sys_fork(ctx),
            LinuxSyscall::Clone => self.sys_clone(ctx),
            LinuxSyscall::Execve => self.sys_execve(ctx),
            LinuxSyscall::Wait4 => self.sys_wait4(ctx),

            // Signals
            LinuxSyscall::Kill => self.sys_kill(ctx),
            LinuxSyscall::Tkill => self.sys_tkill(ctx),
            LinuxSyscall::Tgkill => self.sys_tgkill(ctx),
            LinuxSyscall::RtSigaction => self.sys_sigaction(ctx, process),
            LinuxSyscall::RtSigprocmask => self.sys_sigprocmask(ctx, process),

            // Memory management
            LinuxSyscall::Brk => self.sys_brk(ctx),
            LinuxSyscall::Mmap => self.sys_mmap(ctx),
            LinuxSyscall::Munmap => self.sys_munmap(ctx),
            LinuxSyscall::Mprotect => self.sys_mprotect(ctx),

            // Time
            LinuxSyscall::ClockGettime => self.sys_clock_gettime(ctx),
            LinuxSyscall::Gettimeofday => self.sys_gettimeofday(ctx),
            LinuxSyscall::Nanosleep => self.sys_nanosleep(ctx),

            // Misc
            LinuxSyscall::Uname => self.sys_uname(ctx),
            LinuxSyscall::Getrandom => self.sys_getrandom(ctx),
            LinuxSyscall::SetTidAddress => self.sys_set_tid_address(ctx),
            LinuxSyscall::Futex => self.sys_futex(ctx),
            LinuxSyscall::FutexWaitv => self.sys_futex_waitv(ctx),
            LinuxSyscall::Prlimit64 => self.sys_prlimit64(ctx),
            LinuxSyscall::ArchPrctl => self.sys_arch_prctl(ctx),

            // New Syscalls
            LinuxSyscall::MemfdCreate => self.sys_memfd_create(ctx),
            LinuxSyscall::Eventfd2 => self.sys_eventfd2(ctx),
            LinuxSyscall::Eventfd => self.sys_eventfd2(ctx),

            _ => {
                log::warn!(
                    "Unimplemented syscall: {} ({:#x})",
                    syscall.name(),
                    ctx.syscall_num
                );
                SyscallResult::NotImplemented
            }
        }
    }
    // ...
    fn sys_sigaction(&self, ctx: &SyscallContext, process: &Arc<crate::Process>) -> SyscallResult {
        let sig_num = ctx.arg0 as i32;
        let act_ptr = ctx.arg1;
        let oact_ptr = ctx.arg2;
        let sigsetsize = ctx.arg3;

        log::debug!(
            "sys_sigaction: sig={}, act={:#x}, oact={:#x} pid={}",
            sig_num,
            act_ptr,
            oact_ptr,
            process.pid()
        );

        if let Some(sig) = crate::signal::Signal::from_number(sig_num) {
            // Update signal handler in process
            // We can't safely read the action struct without unsafe copy_from_user implementation
            // matching the target architecture.
            // However, we can track THAT a handler was set.
            {
                let mut signals = process.signals.write();
                // We'll trust the guest set a valid handler if ptr != 0
                if act_ptr != 0 {
                    log::info!("Registered custom handler for signal {:?}", sig);
                    // signals.actions[sig_num as usize] = ...; // If actions existed
                }
            }

            SyscallResult::Success(0)
        } else {
            SyscallResult::Error(LinuxErrno::EINVAL)
        }
    }

    fn sys_sigprocmask(
        &self,
        ctx: &SyscallContext,
        process: &Arc<crate::Process>,
    ) -> SyscallResult {
        let how = ctx.arg0 as i32;
        let set_ptr = ctx.arg1;
        let oldset_ptr = ctx.arg2;

        log::debug!(
            "sys_sigprocmask: how={}, set={:#x} pid={}",
            how,
            set_ptr,
            process.pid()
        );

        {
            let mut signals = process.signals.write();
            // signals.blocked = ...; // Update blocked mask
        }

        SyscallResult::Success(0)
    }

    // === File I/O syscalls ===

    fn sys_read(&self, ctx: &SyscallContext) -> SyscallResult {
        let fd = ctx.arg0 as i32;
        let buf_ptr = ctx.arg1 as *mut u8;
        let count = ctx.arg2 as usize;

        let fds = self.fds.read();
        match fds.get(&fd) {
            Some(fd_info) => {
                match &fd_info.backing {
                    Some(FileBacking::Native(ref file)) => {
                        // Would need unsafe to read into the buffer
                        // For now, return success with 0 bytes
                        SyscallResult::Success(0)
                    }
                    Some(FileBacking::EventFd(ref counter)) => {
                        if count < 8 {
                            return SyscallResult::Error(LinuxErrno::EINVAL);
                        }
                        // Use a separate lock for the counter
                        let counter = counter.clone();
                        drop(fds); // Release fd hierarchy lock

                        match counter.lock() {
                            Ok(mut val) => {
                                // TODO: Handle blocking if val == 0
                                let current = *val;
                                if current == 0 {
                                    // EAGAIN for now
                                    SyscallResult::Error(LinuxErrno::EAGAIN)
                                } else {
                                    *val = 0; // Reset
                                              // Write u64 to buf_ptr (unsafe)
                                              // Logic simulated for now
                                    SyscallResult::Success(8)
                                }
                            }
                            Err(_) => SyscallResult::Error(LinuxErrno::EIO),
                        }
                    }
                    None => {
                        // Standard I/O
                        match fd {
                            0 => {
                                // stdin - would read from actual stdin
                                SyscallResult::Success(0)
                            }
                            _ => SyscallResult::Error(LinuxErrno::EBADF),
                        }
                    }
                }
            }
            None => SyscallResult::Error(LinuxErrno::EBADF),
        }
    }

    fn sys_write(&self, ctx: &SyscallContext) -> SyscallResult {
        let fd = ctx.arg0 as i32;
        let buf_ptr = ctx.arg1 as *const u8;
        let count = ctx.arg2 as usize;

        let fds = self.fds.read();
        match fds.get(&fd) {
            Some(fd_info) => {
                match &fd_info.backing {
                    Some(FileBacking::Native(ref file)) => {
                        // Would need unsafe to write from the buffer
                        SyscallResult::Success(count as i64)
                    }
                    Some(FileBacking::EventFd(ref counter)) => {
                        if count < 8 {
                            return SyscallResult::Error(LinuxErrno::EINVAL);
                        }
                        let counter = counter.clone();
                        drop(fds);

                        match counter.lock() {
                            Ok(mut val) => {
                                // Read u64 from buf_ptr (unsafe)
                                // Add to counter
                                *val = (*val).saturating_add(1); // Simulating write
                                SyscallResult::Success(8)
                            }
                            Err(_) => SyscallResult::Error(LinuxErrno::EIO),
                        }
                    }
                    None => {
                        // Standard I/O (stdout/stderr)
                        match fd {
                            1 | 2 => {
                                // Would write to actual stdout/stderr
                                SyscallResult::Success(count as i64)
                            }
                            _ => SyscallResult::Error(LinuxErrno::EBADF),
                        }
                    }
                }
            }
            None => SyscallResult::Error(LinuxErrno::EBADF),
        }
    }

    fn sys_open(&self, ctx: &SyscallContext) -> SyscallResult {
        let path_ptr = ctx.arg0 as *const i8;
        let flags = ctx.arg1 as i32;
        let mode = ctx.arg2 as u32;

        // Would need to read the path string from process memory
        // For now, simulate success
        let fd = self.alloc_fd();
        self.fds.write().insert(
            fd,
            FileDescriptor {
                backing: None,
                path: String::new(),
                flags,
                is_pipe: false,
            },
        );

        SyscallResult::Success(fd as i64)
    }

    fn sys_openat(&self, ctx: &SyscallContext) -> SyscallResult {
        let dirfd = ctx.arg0 as i32;
        let path_ptr = ctx.arg1 as *const i8;
        let flags = ctx.arg2 as i32;
        let mode = ctx.arg3 as u32;

        // Similar to open, but relative to dirfd
        let fd = self.alloc_fd();
        self.fds.write().insert(
            fd,
            FileDescriptor {
                backing: None,
                path: String::new(),
                flags,
                is_pipe: false,
            },
        );

        SyscallResult::Success(fd as i64)
    }

    fn sys_close(&self, ctx: &SyscallContext) -> SyscallResult {
        let fd = ctx.arg0 as i32;

        if self.fds.write().remove(&fd).is_some() {
            SyscallResult::Success(0)
        } else {
            SyscallResult::Error(LinuxErrno::EBADF)
        }
    }

    fn sys_lseek(&self, ctx: &SyscallContext) -> SyscallResult {
        let fd = ctx.arg0 as i32;
        let offset = ctx.arg1 as i64;
        let whence = ctx.arg2 as i32;

        // Would seek in the actual file
        SyscallResult::Success(offset)
    }

    fn sys_dup(&self, ctx: &SyscallContext) -> SyscallResult {
        let oldfd = ctx.arg0 as i32;

        let fds = self.fds.read();
        if let Some(fd_info) = fds.get(&oldfd) {
            let newfd = self.alloc_fd();
            //drop(fds);

            self.fds.write().insert(
                newfd,
                FileDescriptor {
                    backing: fd_info.backing.clone(), // Clone backing
                    path: fd_info.path.clone(),
                    flags: fd_info.flags,
                    is_pipe: fd_info.is_pipe,
                },
            );

            SyscallResult::Success(newfd as i64)
        } else {
            SyscallResult::Error(LinuxErrno::EBADF)
        }
    }

    fn sys_dup2(&self, ctx: &SyscallContext) -> SyscallResult {
        let oldfd = ctx.arg0 as i32;
        let newfd = ctx.arg1 as i32;

        if oldfd == newfd {
            return SyscallResult::Success(newfd as i64);
        }

        // Would duplicate the file descriptor
        SyscallResult::Success(newfd as i64)
    }

    fn sys_dup3(&self, ctx: &SyscallContext) -> SyscallResult {
        let oldfd = ctx.arg0 as i32;
        let newfd = ctx.arg1 as i32;
        let flags = ctx.arg2 as i32;

        if oldfd == newfd {
            return SyscallResult::Error(LinuxErrno::EINVAL);
        }

        // Would duplicate with flags
        SyscallResult::Success(newfd as i64)
    }

    fn sys_pipe(&self, ctx: &SyscallContext) -> SyscallResult {
        let pipefd_ptr = ctx.arg0 as *mut [i32; 2];

        // Would create a pipe and write fds to pipefd_ptr
        let read_fd = self.alloc_fd();
        let write_fd = self.alloc_fd();

        self.fds.write().insert(
            read_fd,
            FileDescriptor {
                backing: None,
                path: "pipe:read".to_string(),
                flags: open_flags::O_RDONLY,
                is_pipe: true,
            },
        );

        self.fds.write().insert(
            write_fd,
            FileDescriptor {
                backing: None,
                path: "pipe:write".to_string(),
                flags: open_flags::O_WRONLY,
                is_pipe: true,
            },
        );

        SyscallResult::Success(0)
    }

    fn sys_pipe2(&self, ctx: &SyscallContext) -> SyscallResult {
        // Same as pipe but with flags
        self.sys_pipe(ctx)
    }

    fn sys_access(&self, ctx: &SyscallContext) -> SyscallResult {
        // Check file access - would check actual permissions
        SyscallResult::Success(0)
    }

    fn sys_faccessat(&self, ctx: &SyscallContext) -> SyscallResult {
        // Check file access relative to directory fd
        SyscallResult::Success(0)
    }

    fn sys_getcwd(&self, ctx: &SyscallContext) -> SyscallResult {
        let buf_ptr = ctx.arg0 as *mut u8;
        let size = ctx.arg1 as usize;

        // Would write current directory to buffer
        SyscallResult::Success(buf_ptr as i64)
    }

    fn sys_chdir(&self, ctx: &SyscallContext) -> SyscallResult {
        // Change current directory
        SyscallResult::Success(0)
    }

    fn sys_mkdir(&self, ctx: &SyscallContext) -> SyscallResult {
        // Create directory
        SyscallResult::Success(0)
    }

    fn sys_rmdir(&self, ctx: &SyscallContext) -> SyscallResult {
        // Remove directory
        SyscallResult::Success(0)
    }

    fn sys_unlink(&self, ctx: &SyscallContext) -> SyscallResult {
        // Remove file
        SyscallResult::Success(0)
    }

    fn sys_unlinkat(&self, ctx: &SyscallContext) -> SyscallResult {
        // Remove file relative to directory fd
        SyscallResult::Success(0)
    }

    fn sys_stat(&self, ctx: &SyscallContext) -> SyscallResult {
        // Get file status - would fill in stat structure
        SyscallResult::Success(0)
    }

    fn sys_getdents64(&self, ctx: &SyscallContext) -> SyscallResult {
        // Read directory entries
        SyscallResult::Success(0)
    }

    // === Process management syscalls ===

    fn sys_getpid(&self, _ctx: &SyscallContext) -> SyscallResult {
        // Would return actual process ID
        SyscallResult::Success(1000)
    }

    fn sys_getppid(&self, _ctx: &SyscallContext) -> SyscallResult {
        SyscallResult::Success(1)
    }

    fn sys_gettid(&self, _ctx: &SyscallContext) -> SyscallResult {
        SyscallResult::Success(1000)
    }

    fn sys_getuid(&self, _ctx: &SyscallContext) -> SyscallResult {
        SyscallResult::Success(1000)
    }

    fn sys_getgid(&self, _ctx: &SyscallContext) -> SyscallResult {
        SyscallResult::Success(1000)
    }

    fn sys_exit(&self, ctx: &SyscallContext) -> SyscallResult {
        let status = ctx.arg0 as i32;
        log::info!("Process exiting with status: {}", status);
        SyscallResult::Success(0)
    }

    fn sys_fork(&self, _ctx: &SyscallContext) -> SyscallResult {
        // Would create a new process
        // Return 0 in child, child PID in parent
        SyscallResult::Success(0)
    }

    fn sys_clone(&self, ctx: &SyscallContext) -> SyscallResult {
        // Clone with flags
        SyscallResult::Success(0)
    }

    fn sys_execve(&self, ctx: &SyscallContext) -> SyscallResult {
        // Execute a new program
        SyscallResult::Success(0)
    }

    fn sys_wait4(&self, ctx: &SyscallContext) -> SyscallResult {
        let pid = ctx.arg0 as i32;
        // Would wait for child process
        SyscallResult::Success(0)
    }

    // === Signal syscalls ===

    // === Signal syscalls ===
    fn sys_kill(&self, ctx: &SyscallContext, process: &Arc<Process>) -> SyscallResult {
        let pid = ctx.arg0 as i32;
        let sig = ctx.arg1 as i32;

        log::debug!(
            "sys_kill: pid={}, sig={} from pid={}",
            pid,
            sig,
            process.pid()
        );

        let redox_sig = if sig == 0 {
            0
        } else if let Some(s) = crate::signal::Signal::from_number(sig) {
            match s.to_redox() {
                Some(rs) => rs,
                None => return SyscallResult::Error(LinuxErrno::EINVAL),
            }
        } else {
            return SyscallResult::Error(LinuxErrno::EINVAL);
        };

        let target_pid = if pid > 0 {
            pid as usize
        } else if pid == 0 {
            process.pid() as usize
        } else {
            return SyscallResult::NotImplemented;
        };

        match redox_syscall::kill(target_pid, redox_sig) {
            Ok(_) => SyscallResult::Success(0),
            Err(e) => SyscallResult::from_error(e),
        }
    }

    fn sys_rt_sigaction(&self, ctx: &SyscallContext, process: &Arc<Process>) -> SyscallResult {
        let sig = ctx.arg0 as i32;
        let act_ptr = ctx.arg1 as *const crate::signal::LinuxSigAction;
        let oldact_ptr = ctx.arg2 as *mut crate::signal::LinuxSigAction;
        let sigsetsize = ctx.arg3 as usize;

        if sigsetsize != 8 {
            return SyscallResult::Error(LinuxErrno::EINVAL);
        }

        let signal = match Signal::from_number(sig) {
            Some(s) => s,
            None => return SyscallResult::Error(LinuxErrno::EINVAL),
        };

        let mut signals = process.signals.write();

        if !oldact_ptr.is_null() {
            let old_act = signals.get_handler(signal);
            let sa_handler = match old_act.handler {
                SignalHandler::Default => 0,
                SignalHandler::Ignore => 1,
                SignalHandler::Handler(addr) => addr,
            };

            let linux_act = crate::signal::LinuxSigAction {
                sa_handler,
                sa_flags: old_act.flags,
                sa_restorer: old_act.restorer,
                sa_mask: old_act.mask,
            };

            unsafe { *oldact_ptr = linux_act };
        }

        if !act_ptr.is_null() {
            if !signal.can_be_caught() {
                return SyscallResult::Error(LinuxErrno::EINVAL);
            }

            let linux_act = unsafe { *act_ptr };
            let handler = match linux_act.sa_handler {
                0 => SignalHandler::Default,
                1 => SignalHandler::Ignore,
                addr => SignalHandler::Handler(addr),
            };

            let act = SigAction {
                handler,
                flags: linux_act.sa_flags,
                restorer: linux_act.sa_restorer,
                mask: linux_act.sa_mask,
            };

            signals.set_handler(signal, act);
        }

        SyscallResult::Success(0)
    }

    fn sys_rt_sigprocmask(&self, ctx: &SyscallContext, process: &Arc<Process>) -> SyscallResult {
        let how = ctx.arg0 as i32;
        let set_ptr = ctx.arg1 as *const u64;
        let oldset_ptr = ctx.arg2 as *mut u64;
        let sigsetsize = ctx.arg3 as usize;

        if sigsetsize != 8 {
            return SyscallResult::Error(LinuxErrno::EINVAL);
        }

        let mut signals = process.signals.write();

        if !oldset_ptr.is_null() {
            let old_blocked = signals.blocked();
            unsafe { *oldset_ptr = old_blocked };
        }

        if !set_ptr.is_null() {
            let set = unsafe { *set_ptr };
            let current = signals.blocked();

            let new_set = match how {
                0 => current | set,  // SIG_BLOCK
                1 => current & !set, // SIG_UNBLOCK
                2 => set,            // SIG_SETMASK
                _ => return SyscallResult::Error(LinuxErrno::EINVAL),
            };

            signals.set_blocked(new_set);

            // TODO: Update Redox kernel sigprocmask?
            // redox_syscall::sigprocmask(how_redox, ...)
            // Redox has sigprocmask syscall.
            match how {
                0 => {
                    let _ = redox_syscall::sigprocmask(redox_syscall::SIG_BLOCK, &set, None);
                }
                1 => {
                    let _ = redox_syscall::sigprocmask(redox_syscall::SIG_UNBLOCK, &set, None);
                }
                2 => {
                    let _ = redox_syscall::sigprocmask(redox_syscall::SIG_SETMASK, &set, None);
                }
                _ => {}
            }
        }

        SyscallResult::Success(0)
    }

    fn sys_rt_sigreturn(&self, ctx: &SyscallContext, process: &Arc<Process>) -> SyscallResult {
        // The stack pointer points to rt_sigframe.
        // On x86_64 Linux, rt_sigframe contains ucontext at offset (usually +8 or directly if adjusted).
        // However, the handler returns to the trampoline which calls this syscall.
        // The sp used for this syscall should point to the frame.

        let sp = ctx.rsp;

        // TODO: Verify memory range validity before access. This requires a way to check user memory map.
        // Note: This relies on lacd being able to read the user stack directly, which implies
        // shared memory or a mechanism not fully visible here. We assume `sp` is accessible.

        // This offset is arch-dependent. For x86_64, the stack frame set up by the kernel
        // usually puts ucontext after some other info.
        // But if we generated the frame in userspace (lacd), we know layout.
        // Assuming standard layout: sp points to ucontext (after pretcode).
        // Let's assume sp points directly to UContext for now as a starting point.
        let ucontext_ptr = sp as *const UContext;

        unsafe {
            let ucontext = *ucontext_ptr;

            // 1. Restore signal mask
            {
                let mut signals = process.signals.write();
                signals.set_blocked(ucontext.uc_sigmask);
                let _ = redox_syscall::sigprocmask(
                    redox_syscall::SIG_SETMASK,
                    &ucontext.uc_sigmask,
                    None,
                );
            }

            // 2. Restore registers
            // We need to find the current thread. Assuming main thread for now.
            if let Some(thread) = process.threads.read().first() {
                let mut regs = thread.registers.write();

                let mc = &ucontext.uc_mcontext;
                regs.r8 = mc.r8;
                regs.r9 = mc.r9;
                regs.r10 = mc.r10;
                regs.r11 = mc.r11;
                regs.r12 = mc.r12;
                regs.r13 = mc.r13;
                regs.r14 = mc.r14;
                regs.r15 = mc.r15;
                regs.rdi = mc.rdi;
                regs.rsi = mc.rsi;
                regs.rbp = mc.rbp;
                regs.rbx = mc.rbx;
                regs.rdx = mc.rdx;
                regs.rax = mc.rax;
                regs.rcx = mc.rcx;
                regs.rsp = mc.rsp;
                regs.rip = mc.rip;
                regs.rflags = mc.eflags;
                regs.cs = mc.cs as u64;
                regs.gs = mc.gs as u64;
                regs.fs = mc.fs as u64;

                // TODO: Restore FP state (regs.fpstate)
            }
        }

        // Return Success(0) ?
        // Actually, we want to return a value that tells the kernel to switch context.
        // If we just return 0, RAX becomes 0.
        // But we updated `regs.rax`.
        // If the machinery reads `thread.registers` after this call, it will work.
        SyscallResult::Success(0)
    }

    fn sys_tkill(&self, ctx: &SyscallContext) -> SyscallResult {
        let tid = ctx.arg0 as i32;
        let sig = ctx.arg1 as i32;
        SyscallResult::Success(0)
    }

    fn sys_tgkill(&self, ctx: &SyscallContext) -> SyscallResult {
        let tgid = ctx.arg0 as i32;
        let tid = ctx.arg1 as i32;
        let sig = ctx.arg2 as i32;
        SyscallResult::Success(0)
    }

    // === Memory syscalls ===

    fn sys_brk(&self, ctx: &SyscallContext) -> SyscallResult {
        let addr = ctx.arg0;
        // Would adjust program break
        SyscallResult::Success(addr as i64)
    }

    fn sys_mmap(&self, ctx: &SyscallContext) -> SyscallResult {
        let addr = ctx.arg0;
        let len = ctx.arg1;
        let prot = ctx.arg2;
        let flags = ctx.arg3;
        let fd = ctx.arg4 as i32;
        let offset = ctx.arg5;

        // Would map memory
        SyscallResult::Success(addr as i64)
    }

    fn sys_munmap(&self, ctx: &SyscallContext) -> SyscallResult {
        let addr = ctx.arg0;
        let len = ctx.arg1;
        SyscallResult::Success(0)
    }

    fn sys_mprotect(&self, ctx: &SyscallContext) -> SyscallResult {
        let addr = ctx.arg0;
        let len = ctx.arg1;
        let prot = ctx.arg2;
        SyscallResult::Success(0)
    }

    // === Time syscalls ===

    fn sys_clock_gettime(&self, ctx: &SyscallContext) -> SyscallResult {
        let clockid = ctx.arg0 as i32;
        let tp_ptr = ctx.arg1 as *mut u8;
        // Would fill in timespec
        SyscallResult::Success(0)
    }

    fn sys_gettimeofday(&self, ctx: &SyscallContext) -> SyscallResult {
        let tv_ptr = ctx.arg0 as *mut u8;
        let tz_ptr = ctx.arg1 as *mut u8;
        // Would fill in timeval
        SyscallResult::Success(0)
    }

    fn sys_nanosleep(&self, ctx: &SyscallContext) -> SyscallResult {
        let req_ptr = ctx.arg0 as *const u8;
        let rem_ptr = ctx.arg1 as *mut u8;
        // Would sleep
        SyscallResult::Success(0)
    }

    // === Extended Compatibility ===

    fn sys_memfd_create(&self, ctx: &SyscallContext) -> SyscallResult {
        let name_ptr = ctx.arg0 as *const u8;
        let flags = ctx.arg1 as u32;

        let name = format!("memfd_{}_{}", std::process::id(), self.alloc_fd());
        let path = format!("shm:{}", name);

        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
        {
            Ok(file) => {
                let fd = self.alloc_fd();
                self.fds.write().insert(
                    fd,
                    FileDescriptor {
                        backing: Some(FileBacking::Native(std::sync::Arc::new(file))),
                        path,
                        flags: 0,
                        is_pipe: false,
                    },
                );
                SyscallResult::Success(fd as i64)
            }
            Err(e) => {
                log::error!("sys_memfd_create failed: {}", e);
                SyscallResult::Error(LinuxErrno::EFAULT)
            }
        }
    }

    fn sys_eventfd2(&self, ctx: &SyscallContext) -> SyscallResult {
        let initval = ctx.arg0 as u64;
        let flags = ctx.arg1 as i32;

        let fd = self.alloc_fd();
        self.fds.write().insert(
            fd,
            FileDescriptor {
                backing: Some(FileBacking::EventFd(std::sync::Arc::new(
                    std::sync::Mutex::new(initval),
                ))),
                path: "eventfd".to_string(),
                flags,
                is_pipe: false,
            },
        );
        SyscallResult::Success(fd as i64)
    }

    // === Misc syscalls ===

    fn sys_uname(&self, ctx: &SyscallContext) -> SyscallResult {
        let buf_ptr = ctx.arg0 as *mut u8;
        // Would fill in utsname structure with Redox info
        SyscallResult::Success(0)
    }

    fn sys_getrandom(&self, ctx: &SyscallContext) -> SyscallResult {
        let buf_ptr = ctx.arg0 as *mut u8;
        let buflen = ctx.arg1 as usize;
        let flags = ctx.arg2 as u32;
        // Would fill buffer with random bytes
        SyscallResult::Success(buflen as i64)
    }

    fn sys_set_tid_address(&self, ctx: &SyscallContext) -> SyscallResult {
        let tidptr = ctx.arg0;
        // Would set thread ID address
        SyscallResult::Success(1000) // Return thread ID
    }

    fn sys_futex(&self, ctx: &SyscallContext) -> SyscallResult {
        let uaddr = ctx.arg0 as usize;
        let op = ctx.arg1 as usize;
        let val = ctx.arg2 as usize;
        let timeout = ctx.arg3 as usize;
        let uaddr2 = ctx.arg4 as usize;
        let _val3 = ctx.arg5 as usize;

        let res = unsafe { syscall6(SYS_FUTEX, uaddr, op, val, timeout, uaddr2, 0) };

        match res {
            Ok(val) => SyscallResult::Success(val as i64),
            Err(err) => SyscallResult::Error(LinuxErrno::from_redox(err)),
        }
    }

    fn sys_futex_waitv(&self, ctx: &SyscallContext) -> SyscallResult {
        let waiters_addr = ctx.arg0 as usize;
        let nr_futexes = ctx.arg1 as usize;
        let flags = ctx.arg2 as usize;
        let timeout_addr = ctx.arg3 as usize;
        let clockid = ctx.arg4 as usize;

        let res = unsafe {
            syscall5(
                449, // Using custom syscall number for futex_waitv
                waiters_addr,
                nr_futexes,
                flags,
                timeout_addr,
                clockid,
            )
        };

        match res {
            Ok(val) => SyscallResult::Success(val as i64),
            Err(err) => SyscallResult::Error(LinuxErrno::from_redox(err)),
        }
    }

    fn sys_prlimit64(&self, ctx: &SyscallContext) -> SyscallResult {
        let pid = ctx.arg0 as i32;
        let resource = ctx.arg1 as i32;
        // Would get/set resource limits
        SyscallResult::Success(0)
    }

    fn sys_arch_prctl(&self, ctx: &SyscallContext) -> SyscallResult {
        let code = ctx.arg0 as i32;
        let addr = ctx.arg1;
        // Would set architecture-specific thread state (e.g., FS/GS base)
        SyscallResult::Success(0)
    }
}
