//! FS-LINK: Foreign Filesystem Mapper
//!
//! Maps foreign filesystem layouts (Windows, Linux, Android) to RedoxFS.
//! Provides transparent path translation for compatibility layers.
//!
//! # Path Mappings
//!
//! ## Windows
//! - `C:\` → `/windows/c/`
//! - `C:\Windows` → `/windows/c/Windows`
//! - `C:\Users\<user>` → `/windows/c/Users/<user>`
//! - `%USERPROFILE%` → Current user's home
//!
//! ## Linux
//! - `/` → `/linux/`
//! - `/home/<user>` → `/linux/home/<user>`
//! - `/usr` → `/linux/usr`
//! - `/etc` → `/linux/etc`
//!
//! ## Android
//! - `/system` → `/android/system`
//! - `/data` → `/android/data`
//! - `/storage/emulated/0` → `/android/sdcard`
//! - `/sdcard` → `/android/sdcard`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

mod mapper;
mod path;
mod symlink;

pub use mapper::{FsMapper, MountPoint};
pub use path::PathTranslator;

/// FS-LINK configuration
#[derive(Debug, Clone)]
pub struct FsLinkConfig {
    /// Windows root
    pub windows_root: String,
    /// Linux root
    pub linux_root: String,
    /// Android root
    pub android_root: String,
    /// Enable symlink support
    pub enable_symlinks: bool,
    /// Enable case-insensitive matching (for Windows)
    pub case_insensitive: bool,
}

impl Default for FsLinkConfig {
    fn default() -> Self {
        Self {
            windows_root: "/windows".to_string(),
            linux_root: "/linux".to_string(),
            android_root: "/android".to_string(),
            enable_symlinks: true,
            case_insensitive: true,
        }
    }
}

/// Foreign OS type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignOs {
    Windows,
    Linux,
    Android,
    Redox,
}

/// FS-LINK service
pub struct FsLink {
    config: FsLinkConfig,
    /// Active mount points
    mounts: RwLock<BTreeMap<String, MountPoint>>,
    /// Environment variable mappings
    env_vars: RwLock<BTreeMap<String, String>>,
}

impl FsLink {
    pub fn new(config: FsLinkConfig) -> Self {
        let mut env_vars = BTreeMap::new();

        // Setup default Windows environment variables
        env_vars.insert(
            "SystemRoot".to_string(),
            format!("{}/c/Windows", config.windows_root),
        );
        env_vars.insert("SystemDrive".to_string(), "C:".to_string());
        env_vars.insert(
            "TEMP".to_string(),
            format!("{}/c/Windows/Temp", config.windows_root),
        );
        env_vars.insert(
            "TMP".to_string(),
            format!("{}/c/Windows/Temp", config.windows_root),
        );
        env_vars.insert(
            "windir".to_string(),
            format!("{}/c/Windows", config.windows_root),
        );
        env_vars.insert(
            "ProgramFiles".to_string(),
            format!("{}/c/Program Files", config.windows_root),
        );
        env_vars.insert(
            "ProgramFiles(x86)".to_string(),
            format!("{}/c/Program Files (x86)", config.windows_root),
        );
        env_vars.insert(
            "CommonProgramFiles".to_string(),
            format!("{}/c/Program Files/Common Files", config.windows_root),
        );

        Self {
            config,
            mounts: RwLock::new(BTreeMap::new()),
            env_vars: RwLock::new(env_vars),
        }
    }

    /// Translate a foreign path to a Redox path
    pub fn translate(&self, os: ForeignOs, path: &str) -> PathBuf {
        match os {
            ForeignOs::Windows => self.translate_windows(path),
            ForeignOs::Linux => self.translate_linux(path),
            ForeignOs::Android => self.translate_android(path),
            ForeignOs::Redox => PathBuf::from(path),
        }
    }

    /// Translate a Windows path
    fn translate_windows(&self, path: &str) -> PathBuf {
        // Expand environment variables
        let path = self.expand_windows_env(path);

        // Handle UNC paths: \\server\share -> /windows/unc/server/share
        if path.starts_with("\\\\") {
            let unc_path = path.trim_start_matches("\\\\").replace('\\', "/");
            return PathBuf::from(format!("{}/unc/{}", self.config.windows_root, unc_path));
        }

        // Handle drive letters: C:\ -> /windows/c/
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive = path.chars().next().unwrap().to_ascii_lowercase();
            let rest = path[2..].trim_start_matches('\\').replace('\\', "/");
            return PathBuf::from(format!("{}/{}/{}", self.config.windows_root, drive, rest));
        }

        // Relative path - treat as current directory
        PathBuf::from(format!(
            "{}/{}",
            self.config.windows_root,
            path.replace('\\', "/")
        ))
    }

    /// Expand Windows environment variables
    fn expand_windows_env(&self, path: &str) -> String {
        let mut result = path.to_string();
        let env_vars = self.env_vars.read().unwrap();

        // Expand %VAR% style variables
        for (key, value) in env_vars.iter() {
            let pattern = format!("%{}%", key);
            if result.contains(&pattern) {
                result = result.replace(&pattern, value);
            }
        }

        result
    }

    /// Translate a Linux path
    fn translate_linux(&self, path: &str) -> PathBuf {
        if path.starts_with('/') {
            // Absolute path
            PathBuf::from(format!("{}{}", self.config.linux_root, path))
        } else {
            // Relative path
            PathBuf::from(format!("{}/{}", self.config.linux_root, path))
        }
    }

    /// Translate an Android path
    fn translate_android(&self, path: &str) -> PathBuf {
        // Handle special Android paths
        let translated = if path.starts_with("/storage/emulated/0") {
            path.replacen("/storage/emulated/0", "/sdcard", 1)
        } else if path.starts_with("/storage/self/primary") {
            path.replacen("/storage/self/primary", "/sdcard", 1)
        } else {
            path.to_string()
        };

        if translated.starts_with('/') {
            PathBuf::from(format!("{}{}", self.config.android_root, translated))
        } else {
            PathBuf::from(format!("{}/{}", self.config.android_root, translated))
        }
    }

    /// Reverse translate: Redox path to foreign path
    pub fn reverse_translate(&self, os: ForeignOs, redox_path: &str) -> Option<String> {
        match os {
            ForeignOs::Windows => {
                if let Some(rest) = redox_path.strip_prefix(&self.config.windows_root) {
                    // /windows/c/path -> C:\path
                    if rest.len() >= 2 && rest.starts_with('/') {
                        let drive = rest.chars().nth(1)?.to_ascii_uppercase();
                        let path_rest = &rest[2..];
                        return Some(format!("{}:{}", drive, path_rest.replace('/', "\\")));
                    }
                }
                None
            }
            ForeignOs::Linux => redox_path
                .strip_prefix(&self.config.linux_root)
                .map(|s| s.to_string()),
            ForeignOs::Android => redox_path
                .strip_prefix(&self.config.android_root)
                .map(|s| s.to_string()),
            ForeignOs::Redox => Some(redox_path.to_string()),
        }
    }

    /// Mount a foreign filesystem
    pub fn mount(&self, source: &str, target: &str, os: ForeignOs) -> Result<(), FsLinkError> {
        let mount = MountPoint {
            source: source.to_string(),
            target: target.to_string(),
            os,
            read_only: false,
        };

        self.mounts
            .write()
            .unwrap()
            .insert(target.to_string(), mount);
        Ok(())
    }

    /// Unmount a filesystem
    pub fn unmount(&self, target: &str) -> Result<(), FsLinkError> {
        self.mounts
            .write()
            .unwrap()
            .remove(target)
            .ok_or(FsLinkError::NotMounted)?;
        Ok(())
    }

    /// Set an environment variable
    pub fn set_env(&self, key: &str, value: &str) {
        self.env_vars
            .write()
            .unwrap()
            .insert(key.to_string(), value.to_string());
    }

    /// Get an environment variable
    pub fn get_env(&self, key: &str) -> Option<String> {
        self.env_vars.read().unwrap().get(key).cloned()
    }
}

/// FS-LINK errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsLinkError {
    NotMounted,
    AlreadyMounted,
    InvalidPath,
    PermissionDenied,
    IoError,
}

fn main() {
    eprintln!("FS-LINK: Foreign Filesystem Mapper starting...");

    let config = FsLinkConfig::default();
    let _fs_link = FsLink::new(config);

    // TODO: Register "fslink:" scheme and provide path translation services
    eprintln!("FS-LINK: Ready");
}
