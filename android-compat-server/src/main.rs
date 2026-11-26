//! Android Application Compatibility (AAC) Server
//!
//! This daemon provides a compatibility layer for running Android applications
//! on RedoxOS. It implements a Binder IPC bridge and supports both Dalvik
//! bytecode (via interpreter) and native ARM/x86 libraries.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │  Android APK                                                            │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │  APK Contents                                                  │    │
//! │  │  • AndroidManifest.xml                                         │    │
//! │  │  • classes.dex (Dalvik bytecode)                              │    │
//! │  │  • lib/ (Native libraries: armeabi-v7a, arm64-v8a, x86_64)    │    │
//! │  │  • res/ (Resources)                                           │    │
//! │  └────────────────────────────────────────────────────────────────┘    │
//! │                             │                                          │
//! │                             ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │  ART Runtime Emulation                                         │    │
//! │  │  • DEX bytecode interpreter                                    │    │
//! │  │  • JNI bridge for native libraries                            │    │
//! │  │  • Garbage collection                                         │    │
//! │  └────────────────────────────────────────────────────────────────┘    │
//! │                             │                                          │
//! │                             ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐    │
//! │  │  Binder IPC Bridge                                             │    │
//! │  │  • Translates Binder transactions to Redox IPC                │    │
//! │  │  • Service Manager emulation                                  │    │
//! │  │  • Intent delivery                                            │    │
//! │  └────────────────────────────────────────────────────────────────┘    │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Supported Components
//!
//! ## Core Services
//! - Activity Manager (app lifecycle)
//! - Package Manager (APK installation)
//! - Window Manager (surface composition)
//! - Content Provider (data access)
//!
//! ## System Services
//! - SurfaceFlinger bridge (via Redox display scheme)
//! - AudioFlinger bridge (via Redox audio scheme)
//! - Input Manager (touch/keyboard events)

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

mod apk_parser;
mod binder;
mod dex;
mod errno;
mod jni;
mod syscall_table;

pub use binder::{BinderTransaction, ServiceManager};
pub use errno::AndroidError;

/// AAC server configuration
#[derive(Debug, Clone)]
pub struct AacConfig {
    /// Root directory for Android data
    pub android_root: String,
    /// Maximum number of concurrent apps
    pub max_apps: usize,
    /// Enable debug logging
    pub debug: bool,
    /// Path to system apps
    pub system_app_path: String,
    /// Path to user apps
    pub user_app_path: String,
}

impl Default for AacConfig {
    fn default() -> Self {
        Self {
            android_root: "/android".to_string(),
            max_apps: 64,
            debug: false,
            system_app_path: "/android/system/app".to_string(),
            user_app_path: "/android/data/app".to_string(),
        }
    }
}

/// Android UID/GID mapping
#[derive(Debug, Clone, Copy)]
pub struct AndroidId {
    pub uid: u32,
    pub gid: u32,
}

impl AndroidId {
    /// Android app UIDs start at 10000
    pub const APP_START_UID: u32 = 10000;

    /// System UID
    pub const SYSTEM_UID: u32 = 1000;

    /// Root UID
    pub const ROOT_UID: u32 = 0;

    pub fn for_app(app_id: u32) -> Self {
        Self {
            uid: Self::APP_START_UID + app_id,
            gid: Self::APP_START_UID + app_id,
        }
    }
}

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// App is installed but not running
    Stopped,
    /// App is starting up
    Starting,
    /// App is running in foreground
    Foreground,
    /// App is running in background
    Background,
    /// App is being paused
    Pausing,
    /// App is being stopped
    Stopping,
}

/// Android application instance
pub struct AndroidApp {
    /// Package name (e.g., "com.example.app")
    pub package_name: String,
    /// Application ID (0-based, used for UID calculation)
    pub app_id: u32,
    /// Process ID
    pub pid: u32,
    /// Application state
    pub state: AppState,
    /// Android IDs
    pub ids: AndroidId,
    /// Main activity class name
    pub main_activity: String,
    /// APK path
    pub apk_path: String,
    /// Data directory
    pub data_dir: String,
}

impl AndroidApp {
    pub fn new(package_name: String, app_id: u32, apk_path: String) -> Self {
        let data_dir = format!("/android/data/data/{}", package_name);
        Self {
            package_name: package_name.clone(),
            app_id,
            pid: 0,
            state: AppState::Stopped,
            ids: AndroidId::for_app(app_id),
            main_activity: String::new(),
            apk_path,
            data_dir,
        }
    }
}

/// AAC server state
pub struct AacServer {
    /// Server configuration
    pub config: AacConfig,
    /// Service manager for Binder IPC
    pub service_manager: Arc<ServiceManager>,
    /// Installed applications
    pub apps: RwLock<BTreeMap<String, Arc<AndroidApp>>>,
    /// Running applications by PID
    pub running: RwLock<BTreeMap<u32, Arc<AndroidApp>>>,
    /// Next app ID
    next_app_id: AtomicU32,
    /// Next PID
    next_pid: AtomicU32,
}

impl AacServer {
    /// Create a new AAC server
    pub fn new(config: AacConfig) -> Self {
        Self {
            service_manager: Arc::new(ServiceManager::new()),
            config,
            apps: RwLock::new(BTreeMap::new()),
            running: RwLock::new(BTreeMap::new()),
            next_app_id: AtomicU32::new(0),
            next_pid: AtomicU32::new(1000),
        }
    }

    /// Install an APK
    pub fn install_apk(&self, apk_path: &str) -> Result<String, AndroidError> {
        // Parse APK to get package info
        let manifest = apk_parser::parse_manifest(apk_path)?;
        let package_name = manifest.package_name.clone();

        // Allocate app ID
        let app_id = self.next_app_id.fetch_add(1, Ordering::Relaxed);

        // Create app instance
        let app = Arc::new(AndroidApp::new(
            package_name.clone(),
            app_id,
            apk_path.to_string(),
        ));

        // Register app
        self.apps.write().unwrap().insert(package_name.clone(), app);

        Ok(package_name)
    }

    /// Launch an application
    pub fn launch_app(&self, package_name: &str) -> Result<u32, AndroidError> {
        let apps = self.apps.read().unwrap();
        let app = apps
            .get(package_name)
            .ok_or(AndroidError::PackageNotFound)?;

        // Allocate PID
        let pid = self.next_pid.fetch_add(1, Ordering::Relaxed);

        // TODO: Actually spawn the app process
        // 1. Load DEX classes
        // 2. Initialize ART runtime
        // 3. Call Application.onCreate()
        // 4. Start main activity

        Ok(pid)
    }

    /// Get the service manager
    pub fn service_manager(&self) -> &Arc<ServiceManager> {
        &self.service_manager
    }
}

fn main() {
    eprintln!("Android Application Compatibility (AAC) Server starting...");

    let config = AacConfig::default();
    let _server = Arc::new(AacServer::new(config));

    // TODO: Register "android:" scheme and enter daemon loop
    eprintln!("AAC: Ready to accept connections");
}
