//! Filesystem Mapper
//!
//! Maps foreign filesystem structures to Redox.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::ForeignOs;

/// Mount point definition
#[derive(Debug, Clone)]
pub struct MountPoint {
    /// Source path (in foreign format)
    pub source: String,
    /// Target path (in Redox format)
    pub target: String,
    /// Foreign OS type
    pub os: ForeignOs,
    /// Read-only mount
    pub read_only: bool,
}

/// Filesystem mapper
pub struct FsMapper {
    /// Mount points
    mounts: BTreeMap<String, MountPoint>,
}

impl FsMapper {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
        }
    }

    /// Add a mount point
    pub fn add_mount(&mut self, mount: MountPoint) {
        self.mounts.insert(mount.target.clone(), mount);
    }

    /// Remove a mount point
    pub fn remove_mount(&mut self, target: &str) -> Option<MountPoint> {
        self.mounts.remove(target)
    }

    /// Find the mount point for a path
    pub fn find_mount(&self, path: &str) -> Option<&MountPoint> {
        // Find the longest matching prefix
        let mut best_match: Option<&MountPoint> = None;
        let mut best_len = 0;

        for (target, mount) in &self.mounts {
            if path.starts_with(target) && target.len() > best_len {
                best_match = Some(mount);
                best_len = target.len();
            }
        }

        best_match
    }

    /// Resolve a path through mount points
    pub fn resolve(&self, path: &str) -> Option<PathBuf> {
        let mount = self.find_mount(path)?;
        let relative = path.strip_prefix(&mount.target)?;
        Some(PathBuf::from(&mount.source).join(relative.trim_start_matches('/')))
    }

    /// List all mount points
    pub fn list_mounts(&self) -> Vec<&MountPoint> {
        self.mounts.values().collect()
    }
}

impl Default for FsMapper {
    fn default() -> Self {
        Self::new()
    }
}
