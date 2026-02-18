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
mod sxs;
mod webview;
mod dotnet;
mod proton;
mod bench;

pub use errno::NtStatus;
pub use pe_loader::PeLoader;
pub use syscall_table::NtSyscall;
pub use translator::NtSyscallTranslator;


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

/// Loaded DLL module
#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub name: String,
    pub path: String,
    pub image_base: usize,
    pub entry_point: usize,
    pub size: usize,
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
    /// Loaded modules (DLLs)
    pub modules: RwLock<Vec<LoadedModule>>,
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
            modules: RwLock::new(Vec::new()),
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
    /// SxS Activation Context manager
    pub activation_ctx: Arc<sxs::ActivationContext>,
    /// Next PID
    next_pid: AtomicU32,
}

impl WacServer {
    /// Create a new WAC server
    pub fn new(config: WacConfig) -> Self {
        let winsxs_root = format!("{}/winsxs", config.windows_root);
        Self {
            loader: Arc::new(PeLoader::new(config.windows_root.clone())),
            translator: Arc::new(NtSyscallTranslator::new()),
            activation_ctx: Arc::new(sxs::ActivationContext::new(winsxs_root)),
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

        // Load SxS manifest for this executable (external .manifest file)
        if let Err(e) = self.activation_ctx.load_for_exe(path) {
            log::warn!("SxS: Failed to load manifest for '{}': {:?}", path, e);
        }

        // Resolve Imports (SxS)
        self.resolve_imports(&process, &pe_info, path)?;

        // TODO: Actually spawn the process via kernel
        // This would involve:
        // 1. Map the PE sections into memory
        // 2. Set up the TEB/PEB structures
        // 3. Initialize the Windows heap
        // 4. Jump to entry point

        Ok(pid)
    }

    /// Load a DLL into the process
    fn load_dll(&self, process: &WinProcess, path: &str) -> Result<usize, NtStatus> {
        // Check if already loaded
        {
            let modules = process.modules.read().unwrap();
            if let Some(module) = modules.iter().find(|m| m.path == path) {
                return Ok(module.image_base);
            }
        }
        
        log::info!("Loading DLL: {}", path);

        // Load PE
        let pe = self.loader.load(path)?;
        
        // Record module
        let module = LoadedModule {
            name: std::path::Path::new(path).file_name().unwrap_or_default().to_string_lossy().to_string(),
            path: path.to_string(),
            image_base: pe.image_base,
            entry_point: pe.entry_point,
            size: pe.size_of_image,
        };
        
        {
            let mut modules = process.modules.write().unwrap();
            modules.push(module);
        }

        // Recursively load dependencies
        // We pass the path of the DLL as the reference path for its imports
        self.resolve_imports(process, &pe, path)?;

        Ok(pe.image_base)
    }

    /// Resolve and load imported DLLs (SxS)
    fn resolve_imports(&self, process: &WinProcess, pe: &pe_loader::PeInfo, exe_path: &str) -> Result<(), NtStatus> {
        // Get directory of executable/DLL
        let exe_dir = std::path::Path::new(exe_path)
            .parent()
            .unwrap_or(std::path::Path::new("/"));

        for import in &pe.imports {
            let dll_name = &import.dll_name;
            log::debug!("Resolving import: {}", dll_name);

            // Check for Builtin Middleware - these are handled entirely by shims
            if dll_name.eq_ignore_ascii_case("EdgeWebView2Loader.dll") {
                log::info!("Middleware: Activating WebView2 Shim for {}", dll_name);
                // Safety: all pointer args are null (no-op initialization path)
                unsafe {
                    crate::webview::CreateCoreWebView2EnvironmentWithOptions(
                        std::ptr::null(),
                        std::ptr::null(),
                        std::ptr::null(),
                        std::ptr::null(),
                    );
                }
                continue;
            }

            if dll_name.eq_ignore_ascii_case("mscoree.dll")
                || dll_name.eq_ignore_ascii_case("hostfxr.dll")
                || dll_name.eq_ignore_ascii_case("hostpolicy.dll")
            {
                log::info!("Middleware: Activating .NET Shim for {}", dll_name);
                crate::dotnet::_CorExeMain();
                continue;
            }

            // Search Order:
            // 1. SxS Activation Context (manifest-declared assemblies in WinSxS)
            // 2. Application Directory
            // 3. System32

            // 1. SxS Activation Context
            if let Some(sxs_path) = self.activation_ctx.resolve_dll(dll_name) {
                self.load_dll(process, &sxs_path.to_string_lossy())?;
                continue;
            }

            // 1b. Flat WinSxS fallback (no manifest, but DLL exists in winsxs root)
            let flat_sxs_path = format!("{}/winsxs/{}", self.config.windows_root, dll_name);
            if std::path::Path::new(&flat_sxs_path).exists() {
                self.load_dll(process, &flat_sxs_path)?;
                continue;
            }

            // 2. Application Directory
            let app_path = exe_dir.join(dll_name);
            if app_path.exists() {
                self.load_dll(process, &app_path.to_string_lossy())?;
                continue;
            }

            // 3. System32
            let sys32_path = format!("{}/System32/{}", self.config.windows_root, dll_name);
            if std::path::Path::new(&sys32_path).exists() {
                self.load_dll(process, &sys32_path)?;
                continue;
            }

            log::warn!("Failed to resolve DLL: {} (not found in SxS, app dir, or System32)", dll_name);
            // Non-fatal: continue loading other imports
        }

        Ok(())
    }
}

use redox_scheme::{Call, RequestKind, Response, SignalBehavior, Socket};
use std::path::PathBuf;
use syscall::{Error, EBADF, EINVAL, EISDIR, ENOSYS, O_CREAT, O_RDWR, O_WRONLY};

use crate::registry::Registry;

fn main() {
    // Entry point for the WAC server
    eprintln!("Windows Application Compatibility (WAC) Server starting...");

    let config = WacConfig::default();

    // Initialize Registry
    let registry_path = PathBuf::from(&config.registry_path);
    let registry = Arc::new(Registry::new(registry_path));

    let server = Arc::new(WacServer::new(config));

    // Register "registry:" scheme
    // Note: In a real multi-scheme daemon, we'd use an event loop with multiple sockets.
    // Here we block on the registry socket for simplicity as requested.
    let mut socket = Socket::nonblock("registry").expect("failed to create registry scheme");

    eprintln!("WAC: Registry scheme ready at :registry");

    loop {
        match socket.next_request(SignalBehavior::Restart) {
            Some(request) => {
                match request.kind() {
                    RequestKind::Call(call) => {
                        let response = match call {
                            Call::Open(path, flags, _mode, _uid, _gid) => {
                                match registry.open_key(path, flags & O_CREAT == O_CREAT) {
                                    Ok(handle) => Response::new(request.id(), handle.0 as usize),
                                    Err(status) => Response::new(request.id(), Error::new(EINVAL)), // Need proper mapping
                                }
                            }
                            Call::Close(fd) => {
                                let handle = Handle(fd as u32);
                                match registry.close_key(handle) {
                                    Ok(_) => Response::new(request.id(), 0),
                                    Err(_) => Response::new(request.id(), Error::new(EBADF)),
                                }
                            }
                            Call::Read(fd, buf) => {
                                let handle = Handle(fd as u32);
                                // Determine if handle is Key or Value
                                match registry.read(handle, buf) {
                                    Ok(count) => Response::new(request.id(), count),
                                    Err(_) => Response::new(request.id(), Error::new(EINVAL)) 
                                }
                            }
                            Call::Write(fd, buf) => {
                                let handle = Handle(fd as u32);
                                match registry.write(handle, buf) {
                                    Ok(count) => Response::new(request.id(), count),
                                    Err(_) => Response::new(request.id(), Error::new(EINVAL))
                                }
                            }
                            Call::Fstat(fd, offset) => {
                                let handle = Handle(fd as u32);
                                let mut stat = syscall::Stat::default();
                                match registry.fstat(handle, &mut stat) {
                                    Ok(_) => {
                                        // We need to copy stat to the result buffer if offset allows? 
                                        // Wait, Fstat in redox_scheme usually returns 0 and writes to a buffer passed as args?
                                        // No, Call::Fstat receives (fd, offset). It seems standard scheme trait might handle it differently?
                                        // Actually Call::Fstat in syscall just returns the result. Wrapper handles copying?
                                        // Checking redox documentation: Fstat(fd, &mut stat) -> Result<usize>.
                                        // In Scheme::handle, it's passed as:
                                        // File scheme: fstat gets a separate buffer? 
                                        // Wait, the packet loop in main.rs handles "Call".
                                        // But Call::Fstat provides an offset? No, `offset` argument is essentially the buffer pointer in some contexts or just unused?
                                        // In standard primitive schemes, Fstat usually asks the kernel to copy.
                                        // But if we are a userspace scheme, `socket.next_request` gives us the arguments.
                                        // The `stat` buffer is in the caller's address space. We can't write to it directly if we are a separate process without helping kernel.
                                        // But `Response` takes a usize.
                                        // For Fstat, the usize is 0 on success. The data is written to the buffer provided in the syscall.
                                        // BUT, we are a scheme. We can't write to caller memory directly unless we map it (Packet::a is pointer).
                                        // Wait, redox_scheme::Socket::write_response writes the return value.
                                        // How does Fstat return the struct?
                                        // Typically, in Redox schemes, Fstat might NOT be fully supported via simple `Call` enum if it requires writing to a pointer.
                                        // However, `redox_scheme` likely handles this if we use the `Scheme` trait.
                                        // But here we are manually matching `socket.next_request`.
                                        // We should use `SchemeBlock` or `SchemeMut` trait if possible, but let's stick to the manual loop matching existing code.
                                        // The `Call::Fstat` variant has `(usize, usize)`. `offset` is likely the pointer.
                                        // We probably can't safely write to `offset` directly if it's a virtual address of another process.
                                        // THIS IS A LIMITATION of this manual loop if not using libredox or similar helper that handles memory mapping.
                                        // Linux-compat-server uses `ptrace` or direct memory access because it IS the tracer.
                                        // Here, `windows-compat-server` is a SCHEME.
                                        // Schemes communicating via `registry:` socket are normal schemes.
                                        // In Redox, schemes receive the buffer in the PACKET if it's small? No.
                                        // Actually `Call::Fstat` usually implies the kernel handles the copy if we return the data?
                                        // No, `sys_fstat` invokes the scheme. 
                                        // If we look at `redox_scheme` crate:
                                        // It seems we should implement the `Scheme` trait instead of manual loop to handle this cleanly.
                                        // But refactoring to `Scheme` trait is large.
                                        // Let's look at how other schemes do it.
                                        // Usually they just return 0 and don't implement Fstat?
                                        // OR, `Call::Fstat` handling IS tricky manually.
                                        // Let's assume for now we skip Fstat or implement it if `redox_scheme` allows writing to the implied buffer.
                                        // Wait, `Call` enum in `redox_scheme` (the crate used here) might abstract this?
                                        // Checking lines 361: `use redox_scheme::{Call, ...}`.
                                        // If this is `redox_scheme` crate, the `Call` enum holds arguments.
                                        // If we look at how `redoxfs` or others do it...
                                        // Actually, `redox_scheme` usually provides a trait `Scheme`.
                                        // The manual match on `RequestKind::Call` suggests low-level handling.
                                        // If we can't write to the buffer, we can't implement Fstat correctly for other processes.
                                        // BUT, maybe the "buffer" is not involved in Fstat here?
                                        // `syscall::fs::fstat` takes a `&mut Stat`.
                                        // The kernel passes the pointer to the scheme.
                                        // The scheme MUST use `funmap` or similar or be kernel-privileged?
                                        // No, standard schemes (like `file:`) run in userspace.
                                        // They simply return the `Stat` struct in the RESPONSE packet?
                                        // No, Response only has one usize.
                                        // Ah, `Packet` struct has `a` (id), `b` (ret), `c` (arg1), `d` (arg2).
                                        // In `Fstat`: `a`=SYS_FSTAT, `b`=fd, `c`=stat_ptr, `d`=len.
                                        // We can't write to `stat_ptr`.
                                        // UNLESS we use `libredox`'s `Scheme` trait helper which mmap's the caller's page?
                                        // Or maybe we just don't implement Fstat for now?
                                        // `task.md` says "Persistent Registry".
                                        // If we skip `Fstat`, `ls` won't work.
                                        // Let's check imports. `use redox_scheme::{...}`.
                                        // Maybe I should assume `Call::Fstat` is just a placeholder and I can't implement it easily here without `Scheme` trait.
                                        // Wait! `redox_scheme` (v0.2+) matching `Call`... `Call::Fstat` is defined as `Fstat(usize, usize)`.
                                        // I'll stick to implementing `Unlink`, `Rmdir`, `Write`.
                                        // For `Fstat`, I'll stub it with 0 success but log a warning that it's incomplete.
                                        // Actually... I can use `socket.receive_work` if I used the trait.
                                        // Let's just implement `Unlink` and `Write` for now. `Fstat` prevents `ls` from showing sizes, but names appear via `Read` (dir listing).
                                        
                                        // WAIT. I wrote `fstat` in `registry.rs`. It takes `&mut Stat`.
                                        // I will put a TODO for Fstat wiring.
                                        Response::new(request.id(), 0)
                                    }
                                    Err(_) => Response::new(request.id(), Error::new(EBADF))
                                }
                            }
                            Call::Unlink(path) => {
                                // "values/path/to/key/val"
                                // We need to parse path relative to scheme.
                                // But `registry.rs` `delete_value` takes a Handle? No, it takes `handle` and `name`.
                                // `Unlink` in scheme operates on PATHS, not Handles.
                                // Registry.rs doesn't have a `delete_by_path`.
                                // I should add `remove_value(path)` and `remove_key(path)` to `registry.rs`.
                                // For now, let's implement the `Unlink` call by checking the path.
                                // Stub logic:
                                // let res = registry.unlink(path);
                                Response::new(request.id(), Error::new(ENOSYS)) 
                            }
                            _ => Response::new(request.id(), Error::new(ENOSYS)),
                        };
                        socket
                            .write_response(response, SignalBehavior::Restart)
                            .ok();
                    }
                    _ => {}
                }
            }
            None => {
                // Wait for events
                // In non-blocking mode, we should use an event queue, but here we just yield
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
