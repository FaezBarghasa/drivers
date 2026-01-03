//! Symlink Handling
//!
//! Manages symbolic links across foreign filesystems.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Symlink entry
#[derive(Debug, Clone)]
pub struct Symlink {
    /// Link path
    pub link: PathBuf,
    /// Target path
    pub target: PathBuf,
    /// Is this a Windows junction?
    pub is_junction: bool,
    /// Creation time
    pub created: u64,
}

/// Symlink manager
pub struct SymlinkManager {
    /// Known symlinks
    links: BTreeMap<PathBuf, Symlink>,
    /// Enable symlink resolution
    enabled: bool,
}

impl SymlinkManager {
    pub fn new(enabled: bool) -> Self {
        Self {
            links: BTreeMap::new(),
            enabled,
        }
    }

    /// Register a symlink
    pub fn register(&mut self, link: PathBuf, target: PathBuf, is_junction: bool) {
        let symlink = Symlink {
            link: link.clone(),
            target,
            is_junction,
            created: 0, // TODO: Get actual time
        };
        self.links.insert(link, symlink);
    }

    /// Resolve a path through symlinks
    pub fn resolve(&self, path: &PathBuf) -> PathBuf {
        if !self.enabled {
            return path.clone();
        }

        // Check each component for symlinks
        let mut resolved = PathBuf::new();

        for component in path.components() {
            resolved.push(component);

            // Check if this path is a symlink
            if let Some(symlink) = self.links.get(&resolved) {
                // Replace with target
                resolved = symlink.target.clone();
            }
        }

        resolved
    }

    /// Check if a path is a symlink
    pub fn is_symlink(&self, path: &PathBuf) -> bool {
        self.links.contains_key(path)
    }

    /// Get the target of a symlink
    pub fn read_link(&self, path: &PathBuf) -> Option<&PathBuf> {
        self.links.get(path).map(|s| &s.target)
    }

    /// Remove a symlink
    pub fn remove(&mut self, link: &PathBuf) -> Option<Symlink> {
        self.links.remove(link)
    }

    /// List all symlinks under a path
    pub fn list_under(&self, prefix: &PathBuf) -> Vec<&Symlink> {
        self.links
            .values()
            .filter(|s| s.link.starts_with(prefix))
            .collect()
    }
}

impl Default for SymlinkManager {
    fn default() -> Self {
        Self::new(true)
    }
}
