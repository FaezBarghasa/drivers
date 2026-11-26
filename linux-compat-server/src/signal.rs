//! Signal handling for Linux compatibility
//!
//! This module implements Linux signal semantics.

use redox_syscall;
use std::collections::VecDeque;

/// Linux signal numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Signal {
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6,
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,
    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGSTKFLT = 16,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,
    SIGTTIN = 21,
    SIGTTOU = 22,
    SIGURG = 23,
    SIGXCPU = 24,
    SIGXFSZ = 25,
    SIGVTALRM = 26,
    SIGPROF = 27,
    SIGWINCH = 28,
    SIGIO = 29,
    SIGPWR = 30,
    SIGSYS = 31,
    // Real-time signals
    SIGRTMIN = 34,
    SIGRT_1 = 35,
    SIGRT_2 = 36,
    SIGRT_3 = 37,
    SIGRT_4 = 38,
    SIGRT_5 = 39,
    SIGRT_6 = 40,
    SIGRT_7 = 41,
    SIGRT_8 = 42,
    SIGRT_9 = 43,
    SIGRT_10 = 44,
    SIGRT_11 = 45,
    SIGRT_12 = 46,
    SIGRT_13 = 47,
    SIGRT_14 = 48,
    SIGRT_15 = 49,
    SIGRT_16 = 50,
    SIGRT_17 = 51,
    SIGRT_18 = 52,
    SIGRT_19 = 53,
    SIGRT_20 = 54,
    SIGRT_21 = 55,
    SIGRT_22 = 56,
    SIGRT_23 = 57,
    SIGRT_24 = 58,
    SIGRT_25 = 59,
    SIGRT_26 = 60,
    SIGRT_27 = 61,
    SIGRT_28 = 62,
    SIGRT_29 = 63,
    SIGRTMAX = 64,
}

impl Signal {
    /// Convert from signal number
    pub fn from_number(num: i32) -> Option<Self> {
        match num {
            1 => Some(Self::SIGHUP),
            2 => Some(Self::SIGINT),
            3 => Some(Self::SIGQUIT),
            4 => Some(Self::SIGILL),
            5 => Some(Self::SIGTRAP),
            6 => Some(Self::SIGABRT),
            7 => Some(Self::SIGBUS),
            8 => Some(Self::SIGFPE),
            9 => Some(Self::SIGKILL),
            10 => Some(Self::SIGUSR1),
            11 => Some(Self::SIGSEGV),
            12 => Some(Self::SIGUSR2),
            13 => Some(Self::SIGPIPE),
            14 => Some(Self::SIGALRM),
            15 => Some(Self::SIGTERM),
            16 => Some(Self::SIGSTKFLT),
            17 => Some(Self::SIGCHLD),
            18 => Some(Self::SIGCONT),
            19 => Some(Self::SIGSTOP),
            20 => Some(Self::SIGTSTP),
            21 => Some(Self::SIGTTIN),
            22 => Some(Self::SIGTTOU),
            23 => Some(Self::SIGURG),
            24 => Some(Self::SIGXCPU),
            25 => Some(Self::SIGXFSZ),
            26 => Some(Self::SIGVTALRM),
            27 => Some(Self::SIGPROF),
            28 => Some(Self::SIGWINCH),
            29 => Some(Self::SIGIO),
            30 => Some(Self::SIGPWR),
            31 => Some(Self::SIGSYS),
            34 => Some(Self::SIGRTMIN),
            35 => Some(Self::SIGRT_1),
            36 => Some(Self::SIGRT_2),
            37 => Some(Self::SIGRT_3),
            38 => Some(Self::SIGRT_4),
            39 => Some(Self::SIGRT_5),
            40 => Some(Self::SIGRT_6),
            41 => Some(Self::SIGRT_7),
            42 => Some(Self::SIGRT_8),
            43 => Some(Self::SIGRT_9),
            44 => Some(Self::SIGRT_10),
            45 => Some(Self::SIGRT_11),
            46 => Some(Self::SIGRT_12),
            47 => Some(Self::SIGRT_13),
            48 => Some(Self::SIGRT_14),
            49 => Some(Self::SIGRT_15),
            50 => Some(Self::SIGRT_16),
            51 => Some(Self::SIGRT_17),
            52 => Some(Self::SIGRT_18),
            53 => Some(Self::SIGRT_19),
            54 => Some(Self::SIGRT_20),
            55 => Some(Self::SIGRT_21),
            56 => Some(Self::SIGRT_22),
            57 => Some(Self::SIGRT_23),
            58 => Some(Self::SIGRT_24),
            59 => Some(Self::SIGRT_25),
            60 => Some(Self::SIGRT_26),
            61 => Some(Self::SIGRT_27),
            62 => Some(Self::SIGRT_28),
            63 => Some(Self::SIGRT_29),
            64 => Some(Self::SIGRTMAX), // 64
            _ => None,
        }
    }

    /// Get signal name
    pub fn name(&self) -> &'static str {
        match self {
            Self::SIGHUP => "SIGHUP",
            Self::SIGINT => "SIGINT",
            Self::SIGQUIT => "SIGQUIT",
            Self::SIGILL => "SIGILL",
            Self::SIGTRAP => "SIGTRAP",
            Self::SIGABRT => "SIGABRT",
            Self::SIGBUS => "SIGBUS",
            Self::SIGFPE => "SIGFPE",
            Self::SIGKILL => "SIGKILL",
            Self::SIGUSR1 => "SIGUSR1",
            Self::SIGSEGV => "SIGSEGV",
            Self::SIGUSR2 => "SIGUSR2",
            Self::SIGPIPE => "SIGPIPE",
            Self::SIGALRM => "SIGALRM",
            Self::SIGTERM => "SIGTERM",
            Self::SIGSTKFLT => "SIGSTKFLT",
            Self::SIGCHLD => "SIGCHLD",
            Self::SIGCONT => "SIGCONT",
            Self::SIGSTOP => "SIGSTOP",
            Self::SIGTSTP => "SIGTSTP",
            Self::SIGTTIN => "SIGTTIN",
            Self::SIGTTOU => "SIGTTOU",
            Self::SIGURG => "SIGURG",
            Self::SIGXCPU => "SIGXCPU",
            Self::SIGXFSZ => "SIGXFSZ",
            Self::SIGVTALRM => "SIGVTALRM",
            Self::SIGPROF => "SIGPROF",
            Self::SIGWINCH => "SIGWINCH",
            Self::SIGIO => "SIGIO",
            Self::SIGPWR => "SIGPWR",
            Self::SIGSYS => "SIGSYS",
            Self::SIGRTMIN
            | Self::SIGRT_1
            | Self::SIGRT_2
            | Self::SIGRT_3
            | Self::SIGRT_4
            | Self::SIGRT_5
            | Self::SIGRT_6
            | Self::SIGRT_7
            | Self::SIGRT_8
            | Self::SIGRT_9
            | Self::SIGRT_10
            | Self::SIGRT_11
            | Self::SIGRT_12
            | Self::SIGRT_13
            | Self::SIGRT_14
            | Self::SIGRT_15
            | Self::SIGRT_16
            | Self::SIGRT_17
            | Self::SIGRT_18
            | Self::SIGRT_19
            | Self::SIGRT_20
            | Self::SIGRT_21
            | Self::SIGRT_22
            | Self::SIGRT_23
            | Self::SIGRT_24
            | Self::SIGRT_25
            | Self::SIGRT_26
            | Self::SIGRT_27
            | Self::SIGRT_28
            | Self::SIGRT_29
            | Self::SIGRTMAX => "SIGRT",
        }
    }

    /// Check if signal can be caught or ignored
    pub fn can_be_caught(&self) -> bool {
        !matches!(self, Self::SIGKILL | Self::SIGSTOP)
    }

    /// Get default action for signal
    pub fn default_action(&self) -> SignalAction {
        match self {
            // Terminate
            Self::SIGHUP
            | Self::SIGINT
            | Self::SIGKILL
            | Self::SIGPIPE
            | Self::SIGALRM
            | Self::SIGTERM
            | Self::SIGUSR1
            | Self::SIGUSR2
            | Self::SIGPWR
            | Self::SIGSTKFLT
            | Self::SIGIO
            | Self::SIGPROF
            | Self::SIGVTALRM => SignalAction::Terminate,

            // Terminate with core dump
            Self::SIGQUIT
            | Self::SIGILL
            | Self::SIGTRAP
            | Self::SIGABRT
            | Self::SIGBUS
            | Self::SIGFPE
            | Self::SIGSEGV
            | Self::SIGXCPU
            | Self::SIGXFSZ
            | Self::SIGSYS => SignalAction::CoreDump,

            // Stop
            Self::SIGSTOP | Self::SIGTSTP | Self::SIGTTIN | Self::SIGTTOU => SignalAction::Stop,

            // Continue
            Self::SIGCONT => SignalAction::Continue,

            // Ignore
            Self::SIGCHLD | Self::SIGURG | Self::SIGWINCH => SignalAction::Ignore,

            // Real-time signals default to terminate
            Self::SIGRTMIN
            | Self::SIGRT_1
            | Self::SIGRT_2
            | Self::SIGRT_3
            | Self::SIGRT_4
            | Self::SIGRT_5
            | Self::SIGRT_6
            | Self::SIGRT_7
            | Self::SIGRT_8
            | Self::SIGRT_9
            | Self::SIGRT_10
            | Self::SIGRT_11
            | Self::SIGRT_12
            | Self::SIGRT_13
            | Self::SIGRT_14
            | Self::SIGRT_15
            | Self::SIGRT_16
            | Self::SIGRT_17
            | Self::SIGRT_18
            | Self::SIGRT_19
            | Self::SIGRT_20
            | Self::SIGRT_21
            | Self::SIGRT_22
            | Self::SIGRT_23
            | Self::SIGRT_24
            | Self::SIGRT_25
            | Self::SIGRT_26
            | Self::SIGRT_27
            | Self::SIGRT_28
            | Self::SIGRT_29
            | Self::SIGRTMAX => SignalAction::Terminate,
        }
    }

    /// Convert to Redox signal number
    pub fn to_redox(&self) -> Option<usize> {
        match self {
            Self::SIGHUP => Some(redox_syscall::SIGHUP),
            Self::SIGINT => Some(redox_syscall::SIGINT),
            Self::SIGQUIT => Some(redox_syscall::SIGQUIT),
            Self::SIGILL => Some(redox_syscall::SIGILL),
            Self::SIGTRAP => Some(redox_syscall::SIGTRAP),
            Self::SIGABRT => Some(redox_syscall::SIGABRT),
            Self::SIGBUS => Some(redox_syscall::SIGBUS),
            Self::SIGFPE => Some(redox_syscall::SIGFPE),
            Self::SIGKILL => Some(redox_syscall::SIGKILL),
            Self::SIGUSR1 => Some(redox_syscall::SIGUSR1),
            Self::SIGSEGV => Some(redox_syscall::SIGSEGV),
            Self::SIGUSR2 => Some(redox_syscall::SIGUSR2),
            Self::SIGPIPE => Some(redox_syscall::SIGPIPE),
            Self::SIGALRM => Some(redox_syscall::SIGALRM),
            Self::SIGTERM => Some(redox_syscall::SIGTERM),
            Self::SIGSTKFLT => None, // Not in Redox?
            Self::SIGCHLD => Some(redox_syscall::SIGCHLD),
            Self::SIGCONT => Some(redox_syscall::SIGCONT),
            Self::SIGSTOP => Some(redox_syscall::SIGSTOP),
            Self::SIGTSTP => Some(redox_syscall::SIGTSTP),
            Self::SIGTTIN => Some(redox_syscall::SIGTTIN),
            Self::SIGTTOU => Some(redox_syscall::SIGTTOU),
            Self::SIGURG => Some(redox_syscall::SIGURG),
            Self::SIGXCPU => Some(redox_syscall::SIGXCPU),
            Self::SIGXFSZ => Some(redox_syscall::SIGXFSZ),
            Self::SIGVTALRM => Some(redox_syscall::SIGVTALRM),
            Self::SIGPROF => Some(redox_syscall::SIGPROF),
            Self::SIGWINCH => Some(redox_syscall::SIGWINCH),
            Self::SIGIO => Some(redox_syscall::SIGIO),
            Self::SIGPWR => Some(redox_syscall::SIGPWR),
            Self::SIGSYS => Some(redox_syscall::SIGSYS),
            Self::SIGRTMIN
            | Self::SIGRT_1
            | Self::SIGRT_2
            | Self::SIGRT_3
            | Self::SIGRT_4
            | Self::SIGRT_5
            | Self::SIGRT_6
            | Self::SIGRT_7
            | Self::SIGRT_8
            | Self::SIGRT_9
            | Self::SIGRT_10
            | Self::SIGRT_11
            | Self::SIGRT_12
            | Self::SIGRT_13
            | Self::SIGRT_14
            | Self::SIGRT_15
            | Self::SIGRT_16
            | Self::SIGRT_17
            | Self::SIGRT_18
            | Self::SIGRT_19
            | Self::SIGRT_20
            | Self::SIGRT_21
            | Self::SIGRT_22
            | Self::SIGRT_23
            | Self::SIGRT_24
            | Self::SIGRT_25
            | Self::SIGRT_26
            | Self::SIGRT_27
            | Self::SIGRT_28
            | Self::SIGRT_29
            | Self::SIGRTMAX => None, // No RT signals in basic map
        }
    }
}

/// Signal action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalAction {
    /// Terminate the process
    Terminate,
    /// Terminate with core dump
    CoreDump,
    /// Stop the process
    Stop,
    /// Continue the process
    Continue,
    /// Ignore the signal
    Ignore,
}

/// Signal handler
#[derive(Debug, Clone, Copy)]
pub enum SignalHandler {
    /// Default action
    Default,
    /// Ignore signal
    Ignore,
    /// Custom handler at address
    Handler(u64),
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::Default
    }
}

/// Signal action configuration (sigaction)
#[derive(Debug, Clone, Copy, Default)]
pub struct SigAction {
    /// Handler
    pub handler: SignalHandler,
    /// Flags
    pub flags: u64,
    /// Restorer function
    pub restorer: u64,
    /// Signal mask during handler
    pub mask: u64,
}

/// Linux sigaction structure (for syscall interface)
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct LinuxSigAction {
    pub sa_handler: u64,
    pub sa_flags: u64,
    pub sa_restorer: u64,
    pub sa_mask: u64,
}

/// Signal flags
pub mod sa_flags {
    pub const SA_NOCLDSTOP: u64 = 1 << 0;
    pub const SA_NOCLDWAIT: u64 = 1 << 1;
    pub const SA_SIGINFO: u64 = 1 << 2;
    pub const SA_ONSTACK: u64 = 1 << 27;
    pub const SA_RESTART: u64 = 1 << 28;
    pub const SA_NODEFER: u64 = 1 << 30;
    pub const SA_RESETHAND: u64 = 1 << 31;
    pub const SA_RESTORER: u64 = 1 << 26;
}

/// Pending signal
#[derive(Debug, Clone)]
pub struct PendingSignal {
    /// Signal number
    pub signal: Signal,
    /// Signal info
    pub info: SigInfo,
}

/// Signal info (siginfo_t)
#[derive(Debug, Clone, Default)]
pub struct SigInfo {
    /// Signal number
    pub si_signo: i32,
    /// Error number
    pub si_errno: i32,
    /// Signal code
    pub si_code: i32,
    /// Sending process ID
    pub si_pid: i32,
    /// Sending user ID
    pub si_uid: u32,
    /// Exit status or signal
    pub si_status: i32,
    /// User time consumed
    pub si_utime: u64,
    /// System time consumed
    pub si_stime: u64,
    /// Signal value
    pub si_value: u64,
    /// Fault address (for SIGSEGV, etc.)
    pub si_addr: u64,
}

/// Signal code values
pub mod si_code {
    // Common
    pub const SI_USER: i32 = 0; // Sent by kill()
    pub const SI_KERNEL: i32 = 128; // Sent by kernel
    pub const SI_QUEUE: i32 = -1; // Sent by sigqueue()
    pub const SI_TIMER: i32 = -2; // Timer expired
    pub const SI_MESGQ: i32 = -3; // Message queue
    pub const SI_ASYNCIO: i32 = -4; // AIO completion
    pub const SI_SIGIO: i32 = -5; // Queued SIGIO
    pub const SI_TKILL: i32 = -6; // Sent by tkill()

    // SIGILL
    pub const ILL_ILLOPC: i32 = 1; // Illegal opcode
    pub const ILL_ILLOPN: i32 = 2; // Illegal operand
    pub const ILL_ILLADR: i32 = 3; // Illegal addressing mode
    pub const ILL_ILLTRP: i32 = 4; // Illegal trap
    pub const ILL_PRVOPC: i32 = 5; // Privileged opcode
    pub const ILL_PRVREG: i32 = 6; // Privileged register
    pub const ILL_COPROC: i32 = 7; // Coprocessor error
    pub const ILL_BADSTK: i32 = 8; // Bad stack

    // SIGFPE
    pub const FPE_INTDIV: i32 = 1; // Integer divide by zero
    pub const FPE_INTOVF: i32 = 2; // Integer overflow
    pub const FPE_FLTDIV: i32 = 3; // FP divide by zero
    pub const FPE_FLTOVF: i32 = 4; // FP overflow
    pub const FPE_FLTUND: i32 = 5; // FP underflow
    pub const FPE_FLTRES: i32 = 6; // FP inexact result
    pub const FPE_FLTINV: i32 = 7; // Invalid FP operation
    pub const FPE_FLTSUB: i32 = 8; // Subscript out of range

    // SIGSEGV
    pub const SEGV_MAPERR: i32 = 1; // Address not mapped
    pub const SEGV_ACCERR: i32 = 2; // Invalid permissions

    // SIGBUS
    pub const BUS_ADRALN: i32 = 1; // Invalid address alignment
    pub const BUS_ADRERR: i32 = 2; // Non-existent address
    pub const BUS_OBJERR: i32 = 3; // Hardware error

    // SIGCHLD
    pub const CLD_EXITED: i32 = 1; // Child exited
    pub const CLD_KILLED: i32 = 2; // Child killed
    pub const CLD_DUMPED: i32 = 3; // Child dumped core
    pub const CLD_TRAPPED: i32 = 4; // Traced child trapped
    pub const CLD_STOPPED: i32 = 5; // Child stopped
    pub const CLD_CONTINUED: i32 = 6; // Child continued
}

/// Signal state for a process
/// Signal state for a process
pub struct SignalState {
    /// Signal handlers
    handlers: [SigAction; 64],
    /// Pending signals
    pending: VecDeque<PendingSignal>,
    /// Blocked signals (signal mask)
    blocked: u64,
    /// Alternate signal stack
    alt_stack: Option<SignalStack>,
}

impl Default for SignalState {
    fn default() -> Self {
        Self {
            handlers: [SigAction::default(); 64],
            pending: VecDeque::new(),
            blocked: 0,
            alt_stack: None,
        }
    }
}

/// Alternate signal stack
#[derive(Debug, Clone, Copy)]
pub struct SignalStack {
    pub ss_sp: u64,
    pub ss_size: u64,
    pub ss_flags: u32,
}

/// Signal stack flags
pub mod ss_flags {
    pub const SS_ONSTACK: u32 = 1;
    pub const SS_DISABLE: u32 = 2;
    pub const SS_AUTODISARM: u32 = 0x80000000;
}

impl SignalState {
    /// Set signal handler
    pub fn set_handler(&mut self, sig: Signal, action: SigAction) -> Option<SigAction> {
        if !sig.can_be_caught() {
            return None;
        }

        let idx = sig as usize - 1;
        if idx >= 64 {
            return None;
        }

        let old = self.handlers[idx];
        self.handlers[idx] = action;
        Some(old)
    }

    /// Get signal handler
    pub fn get_handler(&self, sig: Signal) -> SigAction {
        let idx = sig as usize - 1;
        if idx >= 64 {
            return SigAction::default();
        }
        self.handlers[idx]
    }

    /// Get blocked signals
    pub fn blocked(&self) -> u64 {
        self.blocked
    }

    /// Set blocked signals
    pub fn set_blocked(&mut self, mask: u64) {
        // SIGKILL and SIGSTOP cannot be blocked
        let unblockable = (1 << (Signal::SIGKILL as u32 - 1)) | (1 << (Signal::SIGSTOP as u32 - 1));
        self.blocked = mask & !unblockable;
    }

    /// Queue a signal
    pub fn queue_signal(&mut self, signal: Signal, info: SigInfo) {
        // Check if signal is blocked
        let sig_bit = 1u64 << (signal as u32 - 1);
        if self.blocked & sig_bit != 0 && signal.can_be_caught() {
            // Signal is blocked - keep it pending but don't deliver
        }

        self.pending.push_back(PendingSignal { signal, info });
    }

    /// Dequeue next pending signal
    pub fn dequeue_signal(&mut self) -> Option<PendingSignal> {
        // Find first signal that's not blocked
        let mut i = 0;
        while i < self.pending.len() {
            let sig = &self.pending[i];
            let sig_bit = 1u64 << (sig.signal as u32 - 1);

            if self.blocked & sig_bit == 0 || !sig.signal.can_be_caught() {
                return self.pending.remove(i);
            }
            i += 1;
        }
        None
    }

    /// Check if there are pending unblocked signals
    pub fn has_pending(&self) -> bool {
        for sig in &self.pending {
            let sig_bit = 1u64 << (sig.signal as u32 - 1);
            if self.blocked & sig_bit == 0 || !sig.signal.can_be_caught() {
                return true;
            }
        }
        false
    }

    /// Set signal mask
    pub fn set_mask(&mut self, mask: u64) -> u64 {
        let old = self.blocked;
        // Cannot block SIGKILL or SIGSTOP
        let mask =
            mask & !((1 << (Signal::SIGKILL as u32 - 1)) | (1 << (Signal::SIGSTOP as u32 - 1)));
        self.blocked = mask;
        old
    }

    /// Get signal mask
    pub fn get_mask(&self) -> u64 {
        self.blocked
    }

    /// Set alternate signal stack
    pub fn set_alt_stack(&mut self, stack: SignalStack) -> Option<SignalStack> {
        let old = self.alt_stack;
        self.alt_stack = Some(stack);
        old
    }

    /// Get alternate signal stack
    pub fn get_alt_stack(&self) -> Option<SignalStack> {
        self.alt_stack
    }
}

/// Send a signal to a process
pub fn send_signal(target_pid: u32, sig: Signal, sender_pid: u32, sender_uid: u32) -> SigInfo {
    SigInfo {
        si_signo: sig as i32,
        si_errno: 0,
        si_code: si_code::SI_USER,
        si_pid: sender_pid as i32,
        si_uid: sender_uid,
        si_status: 0,
        si_utime: 0,
        si_stime: 0,
        si_value: 0,
        si_addr: 0,
    }
}
/// Floating point state
#[derive(Debug, Clone, Copy)]
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

impl Default for FpState {
    fn default() -> Self {
        Self {
            cwd: 0,
            swd: 0,
            ftw: 0,
            fop: 0,
            rip: 0,
            rdp: 0,
            mxcsr: 0,
            mxcsr_mask: 0,
            st_space: [0; 32],
            xmm_space: [0; 64],
            reserved2: [0; 24],
        }
    }
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
    pub fpstate: u64, // Pointer to FpState
    pub __reserved1: [u64; 8],
}

/// Stack descriptor (Linux layout)
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct LinuxStackT {
    pub ss_sp: u64,
    pub ss_flags: i32,
    pub ss_size: usize,
}

/// User context
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct UContext {
    pub uc_flags: u64,
    pub uc_link: u64, // pointer to UContext
    pub uc_stack: LinuxStackT,
    pub uc_mcontext: MContext,
    pub uc_sigmask: u64,
    pub __fpregs_mem: FpState,
}
