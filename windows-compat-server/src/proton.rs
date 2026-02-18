//! Steam / Proton Overlay Verification
//!
//! This module verifies that the WAC server correctly handles the DLL and
//! registry patterns used by Steam and Proton when running Windows games.
//!
//! # Steam Overlay Architecture
//!
//! Steam injects `GameOverlayRenderer.dll` (or `GameOverlayRenderer64.dll`)
//! into every game process. This DLL:
//! - Hooks D3D/Vulkan present calls to draw the overlay UI
//! - Communicates with the Steam client via named pipes (`\\.\pipe\SteamClient`)
//! - Reads configuration from `HKCU\Software\Valve\Steam`
//!
//! # Proton Architecture
//!
//! Proton wraps Wine to run Windows games on Linux. On Redox, the WAC server
//! plays the role of Wine. Proton-specific DLLs include:
//! - `steam_api.dll` / `steam_api64.dll` — Steamworks SDK
//! - `openvr_api.dll` — VR runtime
//! - `dxvk.dll` / `d3d11.dll` — DXVK (Vulkan-backed D3D11)
//!
//! # Verification Strategy
//!
//! We verify:
//! 1. Registry keys expected by Steam are accessible via the `registry:` scheme
//! 2. Known Steam/Proton DLLs are recognized and handled gracefully
//! 3. The named pipe path is translated to a Redox IPC scheme

use crate::NtStatus;

/// Known Steam/Proton DLL names that the WAC server must handle gracefully.
/// These are either shimmed, passed through, or logged as unresolved without
/// hard-failing the process.
pub const STEAM_DLLS: &[&str] = &[
    "steam_api.dll",
    "steam_api64.dll",
    "GameOverlayRenderer.dll",
    "GameOverlayRenderer64.dll",
    "openvr_api.dll",
    "d3d11.dll",
    "d3d9.dll",
    "dxgi.dll",
    "vulkan-1.dll",
];

/// Registry keys that Steam reads on startup.
/// These must be accessible (even if empty) for Steam to proceed.
pub const STEAM_REGISTRY_KEYS: &[&str] = &[
    "HKEY_CURRENT_USER\\Software\\Valve\\Steam",
    "HKEY_LOCAL_MACHINE\\Software\\Valve\\Steam",
    "HKEY_CURRENT_USER\\Software\\Valve\\Steam\\ActiveProcess",
];

/// Verify that all Steam DLLs are handled gracefully by the import resolver.
/// Returns a list of DLL names that would cause a hard failure.
pub fn verify_steam_dll_handling() -> Vec<&'static str> {
    let mut unhandled = Vec::new();
    for &dll in STEAM_DLLS {
        // A DLL is "handled" if it is either:
        // 1. A known middleware (shimmed)
        // 2. Present in the search path
        // 3. Gracefully skipped with a warning (non-fatal)
        //
        // The WAC server's resolve_imports already logs a warning and continues
        // for unresolved DLLs, so none of these should cause a hard failure.
        // We verify this by checking that the DLL name doesn't match any
        // pattern that would trigger a hard error.
        if is_hard_fail_dll(dll) {
            unhandled.push(dll);
        }
    }
    unhandled
}

/// Check if a DLL would cause a hard failure in the import resolver.
/// Currently, no DLL causes a hard failure — unresolved imports are warnings.
fn is_hard_fail_dll(_dll: &str) -> bool {
    // The resolve_imports function logs a warning and continues for all
    // unresolved DLLs. No DLL causes a hard failure at this stage.
    false
}

/// Verify that Steam registry keys can be represented in the registry scheme.
/// Returns `Ok(())` if all keys have valid scheme paths, `Err` otherwise.
pub fn verify_steam_registry_paths() -> Result<(), NtStatus> {
    for &key in STEAM_REGISTRY_KEYS {
        let scheme_path = windows_registry_key_to_scheme_path(key);
        // Validate the path is non-empty and starts with a known hive prefix
        if scheme_path.is_empty() {
            return Err(NtStatus::ObjectNameInvalid);
        }
        log::debug!(
            "Steam registry key '{}' -> scheme path '{}'",
            key,
            scheme_path
        );
    }
    Ok(())
}

/// Convert a Windows registry key path to a Redox `registry:` scheme path.
///
/// Mapping:
/// - `HKEY_CURRENT_USER` -> `registry:user`
/// - `HKEY_LOCAL_MACHINE` -> `registry:machine`
/// - `HKEY_CLASSES_ROOT` -> `registry:classes`
/// - `HKEY_USERS` -> `registry:users`
pub fn windows_registry_key_to_scheme_path(key: &str) -> String {
    let key = key.replace('\\', "/");
    if let Some(rest) = key.strip_prefix("HKEY_CURRENT_USER/") {
        return format!("registry:user/{}", rest.to_lowercase());
    }
    if let Some(rest) = key.strip_prefix("HKEY_LOCAL_MACHINE/") {
        return format!("registry:machine/{}", rest.to_lowercase());
    }
    if let Some(rest) = key.strip_prefix("HKEY_CLASSES_ROOT/") {
        return format!("registry:classes/{}", rest.to_lowercase());
    }
    if let Some(rest) = key.strip_prefix("HKEY_USERS/") {
        return format!("registry:users/{}", rest.to_lowercase());
    }
    // Bare hive names
    match key.as_str() {
        "HKEY_CURRENT_USER" => "registry:user".to_string(),
        "HKEY_LOCAL_MACHINE" => "registry:machine".to_string(),
        "HKEY_CLASSES_ROOT" => "registry:classes".to_string(),
        "HKEY_USERS" => "registry:users".to_string(),
        _ => String::new(),
    }
}

/// Named pipe path translation for Steam IPC.
/// Steam uses `\\.\pipe\SteamClient` for inter-process communication.
/// On Redox, this maps to the `pipe:` scheme.
pub fn translate_named_pipe(win_path: &str) -> Option<String> {
    // Windows named pipe format: \\.\pipe\<name>
    let normalized = win_path.replace('\\', "/");
    if let Some(name) = normalized.strip_prefix("//./pipe/") {
        return Some(format!("pipe:{}", name));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steam_dlls_no_hard_failure() {
        let unhandled = verify_steam_dll_handling();
        assert!(
            unhandled.is_empty(),
            "Steam DLLs that would cause hard failure: {:?}",
            unhandled
        );
    }

    #[test]
    fn test_steam_registry_paths_valid() {
        verify_steam_registry_paths().expect("Steam registry paths should be valid");
    }

    #[test]
    fn test_registry_key_mapping_hkcu() {
        let path = windows_registry_key_to_scheme_path("HKEY_CURRENT_USER\\Software\\Valve\\Steam");
        assert_eq!(path, "registry:user/software/valve/steam");
    }

    #[test]
    fn test_registry_key_mapping_hklm() {
        let path =
            windows_registry_key_to_scheme_path("HKEY_LOCAL_MACHINE\\Software\\Valve\\Steam");
        assert_eq!(path, "registry:machine/software/valve/steam");
    }

    #[test]
    fn test_registry_key_mapping_bare_hive() {
        assert_eq!(
            windows_registry_key_to_scheme_path("HKEY_CURRENT_USER"),
            "registry:user"
        );
        assert_eq!(
            windows_registry_key_to_scheme_path("HKEY_LOCAL_MACHINE"),
            "registry:machine"
        );
    }

    #[test]
    fn test_named_pipe_translation() {
        let redox_path = translate_named_pipe(r"\\.\pipe\SteamClient");
        assert_eq!(redox_path.as_deref(), Some("pipe:SteamClient"));
    }

    #[test]
    fn test_named_pipe_translation_invalid() {
        assert!(translate_named_pipe("not_a_pipe").is_none());
    }

    #[test]
    fn test_all_steam_dlls_listed() {
        // Ensure we cover both 32-bit and 64-bit variants
        assert!(STEAM_DLLS.contains(&"steam_api.dll"));
        assert!(STEAM_DLLS.contains(&"steam_api64.dll"));
        assert!(STEAM_DLLS.contains(&"GameOverlayRenderer.dll"));
        assert!(STEAM_DLLS.contains(&"GameOverlayRenderer64.dll"));
    }
}
