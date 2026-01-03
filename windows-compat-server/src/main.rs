//! Windows Application Compatibility (WAC) Server
//!
//! This daemon provides a compatibility layer for running unmodified Windows
//! PE/COFF binaries on RedoxOS. It intercepts Windows NT syscalls and translates
//! them to their Redox equivalents.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │  Windows Binary (.exe)                                                  │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │  PE/COFF Header + Sections                                     │    │
//! │  │  • DOS Header + PE Signature                                   │    │
//! │  │  • Optional Header (Entry Point, Image Base)                   │    │
//! │  │  • Section Table (.text, .data, .rdata, .idata)               │    │
//! │  └────────────────────────────────────────────────────────────────┘    │
//! │                             │                                          │
//! │                             ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │  NT Syscall Translator                                         │    │
//! │  │  • Intercepts int 0x2e / syscall instruction                   │    │
//! │  │  • Maps NT syscall numbers to Redox equivalents               │    │
//! │  │  • Translates data structures (UNICODE_STRING, HANDLE, etc)   │    │
//! │  └────────────────────────────────────────────────────────────────┘    │
//! │                             │                                          │
//! │                             ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │  Redox Kernel Interface                                        │    │
//! │  │  • Native syscalls via scheme protocol                         │    │
//! │  │  • File mapping: C:\Windows => file:/windows                  │    │
//! │  └────────────────────────────────────────────────────────────────┘    │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Supported NT Syscalls (Initial Implementation)
//!
//! ## File I/O
//! - `NtCreateFile`, `NtOpenFile`, `NtClose`
//! - `NtReadFile`, `NtWriteFile`
//! - `NtQueryInformationFile`, `NtSetInformationFile`
//! - `NtQueryDirectoryFile`
//!
//! ## Process/Thread
//! - `NtCreateProcess`, `NtTerminateProcess`
//! - `NtCreateThread`, `NtTerminateThread`
//! - `NtQueryInformationProcess`, `NtSetInformationProcess`
//!
//! ## Memory
//! - `NtAllocateVirtualMemory`, `NtFreeVirtualMemory`
//! - `NtProtectVirtualMemory`, `NtQueryVirtualMemory`
//! - `NtMapViewOfSection`, `NtUnmapViewOfSection`
//!
//! ## Registry (via file mapping)
//! - `NtOpenKey`, `NtCreateKey`, `NtQueryValueKey`

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

mod errno;
mod ntdll;
mod pe_loader;
mod registry;
mod syscall_table;
mod translator;

pub use errno::NtStatus;
pub use pe_loader::PeLoader;
pub use syscall_table::NtSyscall;
pub use translator::NtSyscallTranslator;

/// WAC server configuration
#[derive(Debug, Clone)]
pub struct WacConfig {
    /// Root directory for Windows filesystem mapping
    pub windows_root: String,
    /// Maximum number of concurrent Windows processes
    pub max_processes: usize,
    /// Enable debug logging
    pub debug: bool,
    /// Registry hive path
    pub registry_path: String,
}

impl Default for WacConfig {
    fn default() -> Self {
        Self {
            windows_root: "/windows".to_string(),
            max_processes: 256,
            debug: false,
            registry_path: "/windows/registry".to_string(),
        }
    }
}

/// Handle type for Windows resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(pub u32);

impl Handle {
    pub const INVALID: Handle = Handle(0xFFFF_FFFF);

    pub fn is_valid(&self) -> bool {
        self.0 != Self::INVALID.0
    }
}

/// Windows process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Created,
    Running,
    Suspended,
    Terminated,
}

/// Windows process representation
pub struct WinProcess {
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// Process state
    pub state: ProcessState,
    /// Image base address
    pub image_base: usize,
    /// Entry point address
    pub entry_point: usize,
    /// Handle table (Windows handles -> Redox file descriptors)
    pub handles: RwLock<BTreeMap<Handle, usize>>,
    /// Exit code
    pub exit_code: AtomicU32,
}

impl WinProcess {
    pub fn new(pid: u32, ppid: u32, image_base: usize, entry_point: usize) -> Self {
        Self {
            pid,
            ppid,
            state: ProcessState::Created,
            image_base,
            entry_point,
            handles: RwLock::new(BTreeMap::new()),
            exit_code: AtomicU32::new(0),
        }
    }

    /// Allocate a new handle
    pub fn alloc_handle(&self, fd: usize) -> Handle {
        let mut handles = self.handles.write().unwrap();
        let handle_value = (handles.len() as u32 + 4) << 2; // Windows handles are 4-byte aligned
        let handle = Handle(handle_value);
        handles.insert(handle, fd);
        handle
    }

    /// Get the Redox fd for a Windows handle
    pub fn get_fd(&self, handle: Handle) -> Option<usize> {
        self.handles.read().unwrap().get(&handle).copied()
    }

    /// Close a handle
    pub fn close_handle(&self, handle: Handle) -> bool {
        self.handles.write().unwrap().remove(&handle).is_some()
    }
}

/// WAC server state
pub struct WacServer {
    /// Server configuration
    pub config: WacConfig,
    /// PE loader
    pub loader: Arc<PeLoader>,
    /// Syscall translator
    pub translator: Arc<NtSyscallTranslator>,
    /// Active processes
    pub processes: RwLock<BTreeMap<u32, Arc<WinProcess>>>,
    /// Next PID
    next_pid: AtomicU32,
}

impl WacServer {
    /// Create a new WAC server
    pub fn new(config: WacConfig) -> Self {
        Self {
            loader: Arc::new(PeLoader::new(config.windows_root.clone())),
            translator: Arc::new(NtSyscallTranslator::new()),
            config,
            processes: RwLock::new(BTreeMap::new()),
            next_pid: AtomicU32::new(1),
        }
    }

    /// Allocate a new PID
    pub fn alloc_pid(&self) -> u32 {
        self.next_pid.fetch_add(1, Ordering::Relaxed)
    }

    /// Register a new process
    pub fn register_process(&self, process: Arc<WinProcess>) {
        self.processes.write().unwrap().insert(process.pid, process);
    }

    /// Get a process by PID
    pub fn get_process(&self, pid: u32) -> Option<Arc<WinProcess>> {
        self.processes.read().unwrap().get(&pid).cloned()
    }

    /// Remove a process
    pub fn remove_process(&self, pid: u32) -> Option<Arc<WinProcess>> {
        self.processes.write().unwrap().remove(&pid)
    }

    /// Get the syscall translator
    pub fn translator(&self) -> &Arc<NtSyscallTranslator> {
        &self.translator
    }

    /// Execute a Windows PE binary
    pub fn exec(&self, path: &str, args: &[String], env: &[String]) -> Result<u32, NtStatus> {
        // Load the PE file
        let pe_info = self.loader.load(path)?;

        // Allocate PID
        let pid = self.alloc_pid();

        // Create process structure
        let process = Arc::new(WinProcess::new(
            pid,
            0, // Parent PID (init)
            pe_info.image_base,
            pe_info.entry_point,
        ));

        // Register the process
        self.register_process(process.clone());

        // TODO: Actually spawn the process via kernel
        // This would involve:
        // 1. Map the PE sections into memory
        // 2. Set up the TEB/PEB structures
        // 3. Initialize the Windows heap
        // 4. Jump to entry point

        Ok(pid)
    }
}

fn main() {
    // Entry point for the WAC server
    // TODO: Implement daemon mode similar to linux-compat-server
    eprintln!("Windows Application Compatibility (WAC) Server starting...");

    let config = WacConfig::default();
    let _server = Arc::new(WacServer::new(config));

    // TODO: Register "windows:" scheme and enter daemon loop
    eprintln!("WAC: Ready to accept connections");
}
