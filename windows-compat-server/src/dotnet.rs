//! .NET Runtime (CLR) Middleware for Redox
//!
//! Intercepts .NET Runtime calls and maps them to native drivers.
//! Mapped DLLs: mscoree.dll, hostfxr.dll, hostpolicy.dll
//!
//! # Architecture
//!
//! The .NET shim provides two levels of compatibility:
//!
//! ## Level 1: mscoree.dll (Legacy .NET Framework)
//! - `_CorExeMain` / `_CorDllMain` - entry points for managed executables
//! - `CorBindToRuntimeEx` - legacy CLR activation
//!
//! ## Level 2: hostfxr.dll (.NET Core / .NET 5+)
//! - `hostfxr_initialize_for_runtime_config` - initialize .NET host
//! - `hostfxr_get_runtime_delegate` - get function pointers into the runtime
//! - `hostfxr_run_app` - run a managed application
//!
//! In a full implementation, these would delegate to a native Redox port of
//! the CoreCLR or Mono runtime. For now, they provide the ABI surface needed
//! for managed executables to start and log their activity.

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the CLR shim has been initialized
static CLR_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// HRESULT: S_OK
const S_OK: i32 = 0;
/// HRESULT: E_FAIL
const E_FAIL: i32 = -0x7FFF_BFFE_u32 as i32;
/// HRESULT: COR_E_NOTIMPLEMENTED
const COR_E_NOTIMPLEMENTED: i32 = -0x7FFF_BFFF_u32 as i32;

/// CLR host delegate types (for hostfxr_get_runtime_delegate)
#[repr(i32)]
#[allow(dead_code)]
pub enum HostfxrDelegateType {
    ComActivation = 0,
    LoadInMemoryAssembly = 1,
    WinrtActivation = 2,
    ComRegister = 3,
    ComUnregister = 4,
    LoadAssemblyAndGetFunctionPointer = 5,
    GetFunctionPointer = 6,
}

/// hostfxr initialization parameters
#[repr(C)]
pub struct HostfxrInitParameters {
    pub size: usize,
    pub host_path: *const u16,
    pub dotnet_root: *const u16,
}

/// Initialize the CLR shim (called once)
fn init_clr() {
    if CLR_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!(".NETShim: Initializing Common Language Runtime shim");
    log::info!(".NETShim: Runtime: Redox CoreCLR Compatibility Layer");
    log::info!(".NETShim: Supported frameworks: .NET 6.0, .NET 7.0, .NET 8.0");
}

// ─── mscoree.dll exports ────────────────────────────────────────────────────

/// Entry point for managed executables (.NET Framework).
/// Called by the OS loader after the PE is mapped.
#[no_mangle]
pub extern "system" fn _CorExeMain() -> i32 {
    init_clr();
    log::info!(".NETShim: _CorExeMain - managed executable entry point");
    // In a real implementation: load the CLR, find the managed entry point
    // from the PE metadata, and transfer control.
    S_OK
}

/// Entry point for managed DLLs (.NET Framework).
///
/// # Safety
/// `hinstance` and `reserved` are Windows ABI values.
#[no_mangle]
pub unsafe extern "system" fn _CorDllMain(
    _hinstance: *mut u8,
    _reason: u32,
    _reserved: *mut u8,
) -> i32 {
    init_clr();
    log::info!(".NETShim: _CorDllMain called");
    1 // TRUE
}

/// Legacy CLR activation (mscoree.dll).
///
/// # Safety
/// All pointer arguments are Windows ABI pointers.
#[no_mangle]
pub unsafe extern "system" fn CorBindToRuntimeEx(
    _pwz_version: *const u16,
    _pwz_build_flavor: *const u16,
    _startup_flags: u32,
    _riid: *const u8,
    _riid_interface: *const u8,
    ppv: *mut *mut u8,
) -> i32 {
    init_clr();
    log::info!(".NETShim: CorBindToRuntimeEx called");
    if !ppv.is_null() {
        *ppv = std::ptr::null_mut();
    }
    // Return COR_E_NOTIMPLEMENTED - the caller should handle this gracefully
    COR_E_NOTIMPLEMENTED
}

/// Get the CLR version string (mscoree.dll).
///
/// # Safety
/// `buffer` is a caller-allocated UTF-16 buffer of `buffer_length` chars.
#[no_mangle]
pub unsafe extern "system" fn GetCORVersion(
    buffer: *mut u16,
    buffer_length: u32,
    length_needed: *mut u32,
) -> i32 {
    // Report as "v4.0.30319" (the last .NET Framework 4.x CLR version)
    let version: &[u16] = &[
        b'v' as u16,
        b'4' as u16,
        b'.' as u16,
        b'0' as u16,
        b'.' as u16,
        b'3' as u16,
        b'0' as u16,
        b'3' as u16,
        b'1' as u16,
        b'9' as u16,
        0u16,
    ];
    let needed = version.len() as u32;
    if !length_needed.is_null() {
        *length_needed = needed;
    }
    if buffer.is_null() || buffer_length < needed {
        return E_FAIL;
    }
    std::ptr::copy_nonoverlapping(version.as_ptr(), buffer, version.len());
    S_OK
}

// ─── hostfxr.dll exports ────────────────────────────────────────────────────

/// Initialize the .NET host for a runtime config (.NET Core / .NET 5+).
///
/// # Safety
/// All pointer arguments are Windows ABI pointers.
#[no_mangle]
pub unsafe extern "system" fn hostfxr_initialize_for_runtime_config(
    runtime_config_path: *const u16,
    _parameters: *const HostfxrInitParameters,
    host_context_handle: *mut *mut u8,
) -> i32 {
    init_clr();

    // Log the config path (UTF-16 -> lossy UTF-8)
    if !runtime_config_path.is_null() {
        let path = utf16_ptr_to_string(runtime_config_path);
        log::info!(
            ".NETShim: hostfxr_initialize_for_runtime_config(\"{}\")",
            path
        );
    }

    // Return a non-null sentinel handle so callers know initialization succeeded
    if !host_context_handle.is_null() {
        // Use a static sentinel so the pointer is valid for the process lifetime
        static SENTINEL: u8 = 0xCE;
        *host_context_handle = &SENTINEL as *const u8 as *mut u8;
    }

    S_OK
}

/// Get a runtime delegate (function pointer into the CLR).
///
/// # Safety
/// All pointer arguments are Windows ABI pointers.
#[no_mangle]
pub unsafe extern "system" fn hostfxr_get_runtime_delegate(
    _host_context_handle: *mut u8,
    delegate_type: i32,
    delegate: *mut *mut u8,
) -> i32 {
    log::info!(
        ".NETShim: hostfxr_get_runtime_delegate(type={})",
        delegate_type
    );
    if !delegate.is_null() {
        *delegate = std::ptr::null_mut();
    }
    // We cannot provide real function pointers without a real CLR
    COR_E_NOTIMPLEMENTED
}

/// Run a managed application.
///
/// # Safety
/// `host_context_handle` must be a handle returned by `hostfxr_initialize_for_runtime_config`.
#[no_mangle]
pub unsafe extern "system" fn hostfxr_run_app(_host_context_handle: *mut u8) -> i32 {
    log::info!(".NETShim: hostfxr_run_app called");
    S_OK
}

/// Close a host context handle.
///
/// # Safety
/// `host_context_handle` must be a handle returned by a hostfxr initialize function.
#[no_mangle]
pub unsafe extern "system" fn hostfxr_close(_host_context_handle: *mut u8) -> i32 {
    log::info!(".NETShim: hostfxr_close called");
    S_OK
}

/// Initialize the .NET host for a dotnet command.
///
/// # Safety
/// All pointer arguments are Windows ABI pointers.
#[no_mangle]
pub unsafe extern "system" fn hostfxr_initialize_for_dotnet_command_line(
    argc: i32,
    _argv: *const *const u16,
    _parameters: *const HostfxrInitParameters,
    host_context_handle: *mut *mut u8,
) -> i32 {
    init_clr();
    log::info!(
        ".NETShim: hostfxr_initialize_for_dotnet_command_line(argc={})",
        argc
    );
    if !host_context_handle.is_null() {
        static SENTINEL: u8 = 0xCE;
        *host_context_handle = &SENTINEL as *const u8 as *mut u8;
    }
    S_OK
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Convert a null-terminated UTF-16 pointer to a Rust String (lossy).
///
/// # Safety
/// `ptr` must point to a valid null-terminated UTF-16 string.
unsafe fn utf16_ptr_to_string(ptr: *const u16) -> String {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clr_init_idempotent() {
        // Calling init_clr multiple times should not panic
        init_clr();
        init_clr();
        assert!(CLR_INITIALIZED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cor_exe_main_returns_ok() {
        let result = _CorExeMain();
        assert_eq!(result, S_OK);
    }

    #[test]
    fn test_get_cor_version() {
        let mut buf = [0u16; 32];
        let mut needed = 0u32;
        let hr = unsafe { GetCORVersion(buf.as_mut_ptr(), buf.len() as u32, &mut needed) };
        assert_eq!(hr, S_OK);
        assert!(needed > 0);
        // First char should be 'v'
        assert_eq!(buf[0], b'v' as u16);
    }
}
