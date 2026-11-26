//! Android Syscall Table
//!
//! Linux syscall numbers used by Android (ARM64 / x86_64).

/// Android/Linux syscall numbers (ARM64)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AndroidSyscall {
    // Process control
    Exit = 93,
    ExitGroup = 94,
    Clone = 220,
    Clone3 = 435,
    Execve = 221,
    Execveat = 281,
    Wait4 = 260,
    Waitid = 95,

    // File operations
    Openat = 56,
    Close = 57,
    Read = 63,
    Write = 64,
    Readv = 65,
    Writev = 66,
    Pread64 = 67,
    Pwrite64 = 68,
    Lseek = 62,
    Fstat = 80,
    Fstatat = 79,
    Faccessat = 48,
    Mkdirat = 34,
    Mknodat = 33,
    Unlinkat = 35,
    Renameat = 38,
    Linkat = 37,
    Symlinkat = 36,
    Readlinkat = 78,
    Fchmod = 52,
    Fchmodat = 53,
    Fchown = 55,
    Fchownat = 54,
    Fcntl = 25,
    Ioctl = 29,
    Dup = 23,
    Dup3 = 24,
    Pipe2 = 59,

    // Memory
    Mmap = 222,
    Munmap = 215,
    Mprotect = 226,
    Mremap = 216,
    Madvise = 233,
    Mlock = 228,
    Munlock = 229,
    Brk = 214,

    // Signals
    RtSigaction = 134,
    RtSigprocmask = 135,
    RtSigreturn = 139,
    Kill = 129,
    Tkill = 130,
    Tgkill = 131,
    Sigaltstack = 132,

    // Time
    ClockGettime = 113,
    ClockSettime = 112,
    ClockGetres = 114,
    ClockNanosleep = 115,
    Nanosleep = 101,
    Gettimeofday = 169,

    // Socket
    Socket = 198,
    Socketpair = 199,
    Bind = 200,
    Listen = 201,
    Accept = 202,
    Accept4 = 242,
    Connect = 203,
    Getsockname = 204,
    Getpeername = 205,
    Sendto = 206,
    Recvfrom = 207,
    Setsockopt = 208,
    Getsockopt = 209,
    Shutdown = 210,
    Sendmsg = 211,
    Recvmsg = 212,

    // Process info
    Getpid = 172,
    Getppid = 173,
    Getuid = 174,
    Geteuid = 175,
    Getgid = 176,
    Getegid = 177,
    Gettid = 178,
    Getgroups = 158,
    Setgroups = 159,
    Setuid = 146,
    Setgid = 144,
    Setreuid = 145,
    Setregid = 143,
    Prctl = 167,

    // Futex
    Futex = 98,

    // Epoll
    EpollCreate1 = 20,
    EpollCtl = 21,
    EpollPwait = 22,

    // Eventfd
    Eventfd2 = 19,

    // Timerfd
    TimerfdCreate = 85,
    TimerfdSettime = 86,
    TimerfdGettime = 87,

    // INotify
    InotifyInit1 = 26,
    InotifyAddWatch = 27,
    InotifyRmWatch = 28,

    // Android specific (Binder)
    Binder = 9999, // Custom - handled via /dev/binder

    // Unknown
    Unknown = 0xFFFFFFFF,
}

impl AndroidSyscall {
    /// Convert from syscall number
    pub fn from_number(num: u32) -> Self {
        match num {
            93 => Self::Exit,
            94 => Self::ExitGroup,
            220 => Self::Clone,
            221 => Self::Execve,
            56 => Self::Openat,
            57 => Self::Close,
            63 => Self::Read,
            64 => Self::Write,
            222 => Self::Mmap,
            215 => Self::Munmap,
            226 => Self::Mprotect,
            98 => Self::Futex,
            172 => Self::Getpid,
            178 => Self::Gettid,
            113 => Self::ClockGettime,
            198 => Self::Socket,
            203 => Self::Connect,
            _ => Self::Unknown,
        }
    }

    /// Get syscall name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Exit => "exit",
            Self::ExitGroup => "exit_group",
            Self::Clone => "clone",
            Self::Clone3 => "clone3",
            Self::Execve => "execve",
            Self::Openat => "openat",
            Self::Close => "close",
            Self::Read => "read",
            Self::Write => "write",
            Self::Mmap => "mmap",
            Self::Munmap => "munmap",
            Self::Mprotect => "mprotect",
            Self::Futex => "futex",
            Self::Getpid => "getpid",
            Self::Gettid => "gettid",
            Self::ClockGettime => "clock_gettime",
            Self::Socket => "socket",
            Self::Connect => "connect",
            Self::Binder => "binder",
            Self::Unknown => "unknown",
            _ => "other",
        }
    }
}
