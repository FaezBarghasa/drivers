//! Linux Application Compatibility (LAC) Server
//!
//! This daemon provides a compatibility layer for running unmodified Linux
//! binaries on RedoxOS. It intercepts Linux syscalls and translates them
//! to their Redox equivalents.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Linux Application                             │
//! │                  (unmodified ELF binary)                         │
//! └──────────────────────────┬──────────────────────────────────────┘
//!                            │ Linux syscall (int 0x80 / syscall)
//!                            ▼
//! ┌──────────────────────────────────────────────────────────────────┐
//! │              Linux Compatibility Server (lacd)                    │
//! │  ┌────────────────────────────────────────────────────────────┐  │
//! │  │  Syscall Interceptor                                       │  │
//! │  │  • Captures Linux syscalls via ptrace                      │  │
//! │  │  • Decodes syscall number and arguments                    │  │
//! │  └──────────────────────────┬─────────────────────────────────┘  │
//! │                             │                                     │
//! │  ┌──────────────────────────▼─────────────────────────────────┐  │
//! │  │  Syscall Translator                                        │  │
//! │  │  • Maps Linux syscall → Redox syscall                      │  │
//! │  │  • Translates argument formats                             │  │
//! │  │  • Handles errno mapping                                   │  │
//! │  └──────────────────────────┬─────────────────────────────────┘  │
//! │                             │                                     │
//! │  ┌──────────────────────────▼─────────────────────────────────┐  │
//! │  │  IPC Layer                                                 │  │
//! │  │  • Communicates with native Redox servers                  │  │
//! │  │  • File server, network server, etc.                       │  │
//! │  └──────────────────────────┬─────────────────────────────────┘  │
//! └──────────────────────────────┼──────────────────────────────────┘
//!                                │ Redox syscall
//!                                ▼
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                     Redox Kernel                                  │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Supported Syscalls (Initial Implementation)
//!
//! ## File I/O
//! - `open`, `openat`, `close`
//! - `read`, `write`, `lseek`
//! - `stat`, `fstat`, `lstat`
//! - `access`, `faccessat`
//! - `dup`, `dup2`, `dup3`
//! - `pipe`, `pipe2`
//!
//! ## Process Management
//! - `fork`, `vfork`, `clone`
//! - `execve`, `execveat`
//! - `exit`, `exit_group`
//! - `wait4`, `waitpid`
//! - `getpid`, `getppid`, `gettid`
//!
//! ## Memory Management
//! - `brk`, `sbrk`
//! - `mmap`, `munmap`, `mprotect`
//!
//! ## Signals
//! - `kill`, `tkill`, `tgkill`
//! - `sigaction`, `rt_sigaction`
//! - `sigprocmask`, `rt_sigprocmask`

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;

use event::{user_data, EventQueue};
use libredox::flag;
use redox_scheme::{RequestKind, SignalBehavior, Socket};

mod elf_loader;
mod errno;
mod ipc;
mod process;
mod signal;
mod syscall_table;
mod translator;

pub use errno::LinuxErrno;
pub use process::{Process, ProcessState};
pub use syscall_table::LinuxSyscall;
pub use translator::SyscallTranslator;

/// LAC server configuration
#[derive(Debug, Clone)]
pub struct LacConfig {
    /// Maximum number of processes
    pub max_processes: usize,
    /// Enable debug logging
    pub debug: bool,
    /// Default stack size for new processes
    pub default_stack_size: usize,
    /// Path prefix mapping (Linux path → Redox path)
    pub path_mappings: HashMap<String, String>,
}

impl Default for LacConfig {
    fn default() -> Self {
        let mut path_mappings = HashMap::new();
        // Map Linux paths to Redox equivalents
        path_mappings.insert("/".to_string(), "file:/".to_string());
        path_mappings.insert("/dev".to_string(), "file:/dev".to_string());
        path_mappings.insert("/proc".to_string(), "proc:".to_string());
        path_mappings.insert("/sys".to_string(), "sys:".to_string());
        path_mappings.insert("/tmp".to_string(), "file:/tmp".to_string());
        path_mappings.insert("/home".to_string(), "file:/home".to_string());

        Self {
            max_processes: 1024,
            debug: false,
            default_stack_size: 8 * 1024 * 1024, // 8 MB
            path_mappings,
        }
    }
}

/// LAC server state
pub struct LacServer {
    config: LacConfig,
    translator: Arc<SyscallTranslator>,
    processes: spin::RwLock<HashMap<u32, Arc<Process>>>,
    next_pid: std::sync::atomic::AtomicU32,
}

impl LacServer {
    /// Create a new LAC server
    pub fn new(config: LacConfig) -> Self {
        Self {
            translator: Arc::new(SyscallTranslator::new(config.path_mappings.clone())),
            config,
            processes: spin::RwLock::new(HashMap::new()),
            next_pid: std::sync::atomic::AtomicU32::new(1000),
        }
    }

    /// Allocate a new PID
    pub fn alloc_pid(&self) -> u32 {
        self.next_pid
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Register a new process
    pub fn register_process(&self, process: Arc<Process>) {
        let pid = process.pid();
        self.processes.write().insert(pid, process);
    }

    /// Get a process by PID
    pub fn get_process(&self, pid: u32) -> Option<Arc<Process>> {
        self.processes.read().get(&pid).cloned()
    }

    /// Remove a process
    pub fn remove_process(&self, pid: u32) -> Option<Arc<Process>> {
        self.processes.write().remove(&pid)
    }

    /// Get the syscall translator
    pub fn translator(&self) -> &Arc<SyscallTranslator> {
        &self.translator
    }

    /// Execute a Linux ELF binary
    pub fn exec(&self, path: &str, args: &[String], env: &[String]) -> Result<u32, LinuxErrno> {
        // Load the ELF binary
        let elf = elf_loader::load_elf(path)?;

        // Create a new process
        let pid = self.alloc_pid();
        let process = Arc::new(Process::new(pid, path.to_string()));

        // Set up the process memory space
        process.setup_memory(&elf)?;

        // Set up arguments and environment
        process.setup_stack(args, env)?;

        // Register the process
        self.register_process(process.clone());

        // Start execution
        process.start(elf.entry_point)?;

        Ok(pid)
    }
}

fn daemon(daemon: redox_daemon::Daemon) -> ! {
    common::setup_logging(
        "compat",
        "linux",
        "lacd",
        common::output_level(),
        common::file_level(),
    );

    log::info!("Linux Compatibility Server starting...");

    let config = LacConfig::default();
    let server = Arc::new(LacServer::new(config));

    // Create the LAC scheme
    let socket = Socket::nonblock("lac").expect("lacd: failed to create lac scheme");

    log::info!("LAC scheme registered at :lac");

    user_data! {
        enum Source {
            Scheme,
        }
    }

    let event_queue = EventQueue::<Source>::new().expect("lacd: failed to create event queue");

    event_queue
        .subscribe(
            socket.inner().as_raw_fd() as usize,
            Source::Scheme,
            event::EventFlags::READ,
        )
        .unwrap();

    libredox::call::setrens(0, 0).expect("lacd: failed to enter null namespace");

    daemon
        .ready()
        .expect("lacd: failed to mark daemon as ready");

    log::info!("LAC server ready");

    for event in event_queue.map(|e| e.expect("lacd: failed to get next event")) {
        match event.user_data {
            Source::Scheme => {
                loop {
                    let request = match socket.next_request(SignalBehavior::Restart) {
                        Ok(Some(request)) => request,
                        Ok(None) => {
                            std::process::exit(0);
                        }
                        Err(err) if err.errno == syscall::error::EAGAIN => break,
                        Err(err) => panic!("lacd: failed to read scheme: {err}"),
                    };

                    match request.kind() {
                        RequestKind::Call(call) => {
                            // Handle scheme calls
                            let response = handle_scheme_call(&server, call);
                            socket
                                .write_response(response, SignalBehavior::Restart)
                                .expect("lacd: failed to write response");
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    unreachable!()
}

fn handle_scheme_call(
    _server: &Arc<LacServer>,
    call: redox_scheme::Call,
) -> redox_scheme::Response {
    // Basic scheme handling - would be extended for full functionality
    call.error(syscall::error::ENOSYS)
}

fn main() {
    redox_daemon::Daemon::new(daemon).expect("lacd: failed to create daemon");
}
