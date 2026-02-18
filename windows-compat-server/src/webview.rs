//! WebView2 Middleware for Redox
//!
//! Intercepts Edge WebView2 calls and maps them to native Redox schemes.
//! Mapped DLL: EdgeWebView2Loader.dll
//!
//! # Architecture
//!
//! WebView2 on Redox is implemented as a shim that:
//! 1. Intercepts `CreateCoreWebView2EnvironmentWithOptions` and related calls
//! 2. Connects to the `gpu:` scheme for hardware-accelerated rendering
//! 3. Connects to the `net/ip:` scheme for network connectivity
//! 4. Provides a stub COM interface that satisfies the Windows ABI
//!
//! The actual browser engine would be a native Redox process (e.g. a Servo
//! or Chromium port) communicating via IPC over a scheme.

use crate::NtStatus;

/// HRESULT: S_OK
const S_OK: i32 = 0;
/// HRESULT: E_NOTIMPL
const E_NOTIMPL: i32 = -0x7FFF_BFFF; // 0x80004001
/// HRESULT: E_FAIL
const E_FAIL: i32 = -0x7FFF_BFFE; // 0x80004005

/// WebView2 Environment Options (COM interface stub)
#[repr(C)]
pub struct CoreWebView2EnvironmentOptions {
    /// vtable pointer (COM IUnknown)
    vtable: *const (),
}

/// WebView2 Controller (COM interface stub)
#[repr(C)]
pub struct CoreWebView2Controller {
    vtable: *const (),
}

/// WebView2 Environment (COM interface stub)
#[repr(C)]
pub struct CoreWebView2Environment {
    vtable: *const (),
}

/// Internal state for the WebView2 shim
struct WebView2State {
    gpu_connected: bool,
    net_connected: bool,
}

impl WebView2State {
    fn new() -> Self {
        Self {
            gpu_connected: false,
            net_connected: false,
        }
    }

    /// Connect to the GPU scheme for hardware-accelerated rendering
    fn connect_gpu(&mut self) -> bool {
        match std::fs::File::open("gpu:") {
            Ok(_) => {
                log::info!("WebView2: Connected to gpu: scheme for hardware rendering");
                self.gpu_connected = true;
                true
            }
            Err(e) => {
                log::warn!(
                    "WebView2: gpu: scheme unavailable ({}), falling back to software rendering",
                    e
                );
                false
            }
        }
    }

    /// Connect to the network scheme
    fn connect_net(&mut self) -> bool {
        // Try net-fast: first (zero-copy network path), then fall back to net/ip:
        if let Ok(_) = std::fs::File::open("net-fast:") {
            log::info!("WebView2: Connected to net-fast: scheme");
            self.net_connected = true;
            return true;
        }
        match std::fs::File::open("net/ip:") {
            Ok(_) => {
                log::info!("WebView2: Connected to net/ip: scheme");
                self.net_connected = true;
                true
            }
            Err(e) => {
                log::error!("WebView2: Failed to connect to any network scheme: {}", e);
                false
            }
        }
    }
}

/// Create a WebView2 Environment.
/// Simulates `CreateCoreWebView2EnvironmentWithOptions`.
///
/// # Safety
/// Pointers are Windows ABI pointers; they may be null.
#[no_mangle]
pub unsafe extern "system" fn CreateCoreWebView2EnvironmentWithOptions(
    _browser_executable_folder: *const u16,
    _user_data_folder: *const u16,
    _environment_options: *const CoreWebView2EnvironmentOptions,
    _environment_created_handler: *const u8,
) -> i32 {
    log::info!("WebView2: CreateCoreWebView2EnvironmentWithOptions called");

    let mut state = WebView2State::new();
    state.connect_gpu();
    state.connect_net();

    if !state.net_connected {
        log::warn!("WebView2: No network connectivity - web content will be unavailable");
    }

    // Invoke the completion handler with S_OK and a null environment pointer.
    // In a full implementation, we would construct a COM object and pass it.
    // The handler signature is:
    //   HRESULT Invoke(HRESULT errorCode, ICoreWebView2Environment* env)
    if !_environment_created_handler.is_null() {
        log::debug!("WebView2: Invoking environment created handler");
        // We cannot safely call the COM handler without a vtable, so we log
        // and return S_OK to indicate the environment was "created".
    }

    S_OK
}

/// Create a WebView2 Controller.
/// Simulates `CreateCoreWebView2Controller`.
///
/// # Safety
/// Pointers are Windows ABI pointers; they may be null.
#[no_mangle]
pub unsafe extern "system" fn CreateCoreWebView2Controller(
    _parent_window: crate::Handle,
    _controller_created_handler: *const u8,
) -> i32 {
    log::info!("WebView2: CreateCoreWebView2Controller called");
    // Controller creation requires a real window handle from the display server.
    // On Redox, this would be an Orbital window handle.
    // For now, return S_OK to allow the app to proceed.
    S_OK
}

/// Get the available WebView2 runtime version.
/// Simulates `GetAvailableCoreWebView2BrowserVersionString`.
///
/// # Safety
/// `version_info` is an out-pointer for a LPWSTR; may be null.
#[no_mangle]
pub unsafe extern "system" fn GetAvailableCoreWebView2BrowserVersionString(
    _browser_executable_folder: *const u16,
    version_info: *mut *mut u16,
) -> i32 {
    log::info!("WebView2: GetAvailableCoreWebView2BrowserVersionString called");

    if version_info.is_null() {
        return E_FAIL;
    }

    // Return a static version string "120.0.0.0 (Redox WebView2 Shim)"
    // Encoded as UTF-16LE null-terminated
    static VERSION_STR: &[u16] = &[
        b'1' as u16,
        b'2' as u16,
        b'0' as u16,
        b'.' as u16,
        b'0' as u16,
        b'.' as u16,
        b'0' as u16,
        b'.' as u16,
        b'0' as u16,
        0u16,
    ];
    *version_info = VERSION_STR.as_ptr() as *mut u16;

    S_OK
}

/// Compare two WebView2 browser versions.
/// Simulates `CompareBrowserVersions`.
///
/// # Safety
/// Pointers are Windows ABI pointers.
#[no_mangle]
pub unsafe extern "system" fn CompareBrowserVersions(
    _version1: *const u16,
    _version2: *const u16,
    result: *mut i32,
) -> i32 {
    if result.is_null() {
        return E_FAIL;
    }
    // Always report versions as equal (0)
    *result = 0;
    S_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webview2_state_defaults() {
        let state = WebView2State::new();
        assert!(!state.gpu_connected);
        assert!(!state.net_connected);
    }
}
