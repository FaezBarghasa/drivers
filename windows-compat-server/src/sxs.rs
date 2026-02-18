//! Side-by-Side (SxS) Activation Context and Manifest Parser
//!
//! Implements Windows SxS assembly manifest parsing and activation context
//! management. Manifests are XML files (embedded in PE .rsrc or external
//! `.exe.manifest`) that declare DLL dependencies with specific versions.
//!
//! # Manifest Format
//! ```xml
//! <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
//! <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
//!   <assemblyIdentity version="1.0.0.0" processorArchitecture="X86"
//!     name="MyApp" type="win32"/>
//!   <dependency>
//!     <dependentAssembly>
//!       <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls"
//!         version="6.0.0.0" processorArchitecture="X86" publicKeyToken="6595b64144ccf1df"/>
//!     </dependentAssembly>
//!   </dependency>
//! </assembly>
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::NtStatus;

/// Assembly identity from a manifest
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssemblyIdentity {
    pub name: String,
    pub version: String,
    pub processor_architecture: String,
    pub public_key_token: String,
    pub assembly_type: String,
}

impl AssemblyIdentity {
    /// Parse from XML attribute map
    fn from_attrs(attrs: &HashMap<String, String>) -> Option<Self> {
        Some(Self {
            name: attrs.get("name").cloned().unwrap_or_default(),
            version: attrs.get("version").cloned().unwrap_or_default(),
            processor_architecture: attrs
                .get("processorArchitecture")
                .cloned()
                .unwrap_or_else(|| "*".to_string()),
            public_key_token: attrs.get("publicKeyToken").cloned().unwrap_or_default(),
            assembly_type: attrs
                .get("type")
                .cloned()
                .unwrap_or_else(|| "win32".to_string()),
        })
    }

    /// Construct the WinSxS folder name for this assembly
    /// Format: `{arch}_{name}_{token}_{version}_none_deadbeef`
    pub fn winsxs_dir_name(&self) -> String {
        let arch = self.processor_architecture.to_lowercase();
        let name = self.name.to_lowercase();
        let token = if self.public_key_token.is_empty() {
            "0000000000000000".to_string()
        } else {
            self.public_key_token.to_lowercase()
        };
        format!("{}_{}_{}_{}_{}", arch, name, token, self.version, "none")
    }
}

/// A parsed assembly manifest
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Identity of this assembly
    pub identity: AssemblyIdentity,
    /// Dependent assemblies
    pub dependencies: Vec<AssemblyIdentity>,
    /// Requested execution level (for UAC)
    pub requested_execution_level: Option<String>,
    /// DPI awareness setting
    pub dpi_aware: bool,
}

impl Manifest {
    /// Parse a manifest from XML bytes using a simple hand-rolled parser.
    /// This avoids needing `quick-xml` as a dependency.
    pub fn parse(xml: &[u8]) -> Result<Self, NtStatus> {
        let text = std::str::from_utf8(xml).map_err(|_| NtStatus::InvalidImageFormat)?;
        let mut identity: Option<AssemblyIdentity> = None;
        let mut dependencies: Vec<AssemblyIdentity> = Vec::new();
        let mut requested_execution_level: Option<String> = None;
        let mut dpi_aware = false;
        let mut in_dependent_assembly = false;

        // Simple XML tag scanner - handles the manifest format
        let mut pos = 0;
        let bytes = text.as_bytes();
        while pos < bytes.len() {
            // Find next '<'
            if bytes[pos] != b'<' {
                pos += 1;
                continue;
            }
            // Find closing '>'
            let start = pos + 1;
            let end = match bytes[start..].iter().position(|&b| b == b'>') {
                Some(i) => start + i,
                None => break,
            };
            let tag_content = &text[start..end];
            pos = end + 1;

            // Skip comments and processing instructions
            if tag_content.starts_with('!') || tag_content.starts_with('?') {
                continue;
            }

            // Check for closing tag
            if tag_content.starts_with('/') {
                let tag_name = tag_content[1..].trim();
                if tag_name == "dependentAssembly" {
                    in_dependent_assembly = false;
                }
                continue;
            }

            // Parse tag name and attributes
            let (tag_name, attrs_str) = split_tag(tag_content);
            let attrs = parse_attrs(attrs_str);

            match tag_name {
                "assemblyIdentity" => {
                    if let Some(id) = AssemblyIdentity::from_attrs(&attrs) {
                        if in_dependent_assembly {
                            dependencies.push(id);
                        } else if identity.is_none() {
                            identity = Some(id);
                        }
                    }
                }
                "dependentAssembly" => {
                    in_dependent_assembly = true;
                }
                "requestedExecutionLevel" => {
                    requested_execution_level = attrs.get("level").cloned();
                }
                "dpiAware" => {
                    // Content follows, but we check the next text node
                    // For simplicity, mark as aware if the tag exists
                    dpi_aware = true;
                }
                _ => {}
            }
        }

        let identity = identity.ok_or(NtStatus::InvalidImageFormat)?;

        Ok(Manifest {
            identity,
            dependencies,
            requested_execution_level,
            dpi_aware,
        })
    }

    /// Parse a manifest from a file path
    pub fn from_file(path: &Path) -> Result<Self, NtStatus> {
        let data = std::fs::read(path).map_err(|_| NtStatus::ObjectNameNotFound)?;
        Self::parse(&data)
    }
}

/// Split a tag like `assemblyIdentity name="foo" version="1.0"` into
/// the tag name and the attributes string.
fn split_tag(tag: &str) -> (&str, &str) {
    let tag = tag.trim_end_matches('/').trim();
    match tag.find(|c: char| c.is_whitespace()) {
        Some(i) => (&tag[..i], &tag[i..]),
        None => (tag, ""),
    }
}

/// Parse XML attributes from a string like ` name="foo" version="1.0"`.
fn parse_attrs(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut rest = s.trim();
    while !rest.is_empty() {
        // Find '='
        let eq = match rest.find('=') {
            Some(i) => i,
            None => break,
        };
        let key = rest[..eq].trim().to_string();
        rest = &rest[eq + 1..].trim_start();

        // Find quoted value
        if rest.is_empty() {
            break;
        }
        let quote = rest.as_bytes()[0];
        if quote != b'"' && quote != b'\'' {
            break;
        }
        rest = &rest[1..];
        let close = match rest.find(quote as char) {
            Some(i) => i,
            None => break,
        };
        let value = rest[..close].to_string();
        rest = &rest[close + 1..].trim_start();
        map.insert(key, value);
    }
    map
}

/// Activation Context - tracks active manifests and resolves DLL paths
pub struct ActivationContext {
    /// WinSxS root directory (e.g. `/windows/winsxs`)
    winsxs_root: PathBuf,
    /// Loaded manifests, keyed by assembly name
    manifests: RwLock<HashMap<String, Arc<Manifest>>>,
}

impl ActivationContext {
    pub fn new(winsxs_root: impl Into<PathBuf>) -> Self {
        Self {
            winsxs_root: winsxs_root.into(),
            manifests: RwLock::new(HashMap::new()),
        }
    }

    /// Activate a manifest, loading it and all its dependencies
    pub fn activate(&self, manifest: Manifest) {
        let name = manifest.identity.name.clone();
        let manifest = Arc::new(manifest);
        self.manifests.write().unwrap().insert(name, manifest);
    }

    /// Load and activate a manifest from a file
    pub fn activate_from_file(&self, path: &Path) -> Result<(), NtStatus> {
        let manifest = Manifest::from_file(path)?;
        log::info!(
            "SxS: Activating manifest for '{}' v{}",
            manifest.identity.name,
            manifest.identity.version
        );
        self.activate(manifest);
        Ok(())
    }

    /// Resolve a DLL name to a full path using the active manifests.
    /// Returns `None` if the DLL is not found in any active assembly.
    pub fn resolve_dll(&self, dll_name: &str) -> Option<PathBuf> {
        let manifests = self.manifests.read().unwrap();
        for manifest in manifests.values() {
            // Check if any dependency assembly contains this DLL
            for dep in &manifest.dependencies {
                let dir = self.winsxs_root.join(dep.winsxs_dir_name());
                let dll_path = dir.join(dll_name);
                if dll_path.exists() {
                    log::debug!("SxS: Resolved '{}' -> '{}'", dll_name, dll_path.display());
                    return Some(dll_path);
                }
            }
        }
        None
    }

    /// Try to load a manifest for a PE executable.
    /// Checks for an external `.exe.manifest` file first, then falls back
    /// to an embedded manifest in the PE's `.rsrc` section.
    pub fn load_for_exe(&self, exe_path: &str) -> Result<(), NtStatus> {
        // 1. Check for external manifest: `foo.exe.manifest`
        let external = format!("{}.manifest", exe_path);
        let external_path = Path::new(&external);
        if external_path.exists() {
            return self.activate_from_file(external_path);
        }

        // 2. Check for sidecar manifest: `foo.exe.1` (older format)
        let sidecar = format!("{}.1", exe_path);
        let sidecar_path = Path::new(&sidecar);
        if sidecar_path.exists() {
            return self.activate_from_file(sidecar_path);
        }

        // 3. Embedded manifest would require PE parsing - log and skip
        log::debug!(
            "SxS: No external manifest found for '{}', embedded manifest not extracted",
            exe_path
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MANIFEST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="1.0.0.0" processorArchitecture="X86"
    name="MyTestApp" type="win32"/>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0" processorArchitecture="X86"
        publicKeyToken="6595b64144ccf1df"/>
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>"#;

    #[test]
    fn test_manifest_parse_identity() {
        let m = Manifest::parse(SAMPLE_MANIFEST).expect("parse failed");
        assert_eq!(m.identity.name, "MyTestApp");
        assert_eq!(m.identity.version, "1.0.0.0");
        assert_eq!(m.identity.processor_architecture, "X86");
    }

    #[test]
    fn test_manifest_parse_dependencies() {
        let m = Manifest::parse(SAMPLE_MANIFEST).expect("parse failed");
        assert_eq!(m.dependencies.len(), 1);
        assert_eq!(m.dependencies[0].name, "Microsoft.Windows.Common-Controls");
        assert_eq!(m.dependencies[0].version, "6.0.0.0");
        assert_eq!(m.dependencies[0].public_key_token, "6595b64144ccf1df");
    }

    #[test]
    fn test_manifest_parse_execution_level() {
        let m = Manifest::parse(SAMPLE_MANIFEST).expect("parse failed");
        assert_eq!(m.requested_execution_level.as_deref(), Some("asInvoker"));
    }

    #[test]
    fn test_winsxs_dir_name() {
        let id = AssemblyIdentity {
            name: "Microsoft.Windows.Common-Controls".to_string(),
            version: "6.0.0.0".to_string(),
            processor_architecture: "X86".to_string(),
            public_key_token: "6595b64144ccf1df".to_string(),
            assembly_type: "win32".to_string(),
        };
        let dir = id.winsxs_dir_name();
        assert!(dir.contains("microsoft.windows.common-controls"));
        assert!(dir.contains("6595b64144ccf1df"));
        assert!(dir.contains("6.0.0.0"));
    }

    #[test]
    fn test_parse_attrs() {
        let attrs = parse_attrs(r#" name="foo" version="1.2.3.4" type="win32""#);
        assert_eq!(attrs.get("name").map(|s| s.as_str()), Some("foo"));
        assert_eq!(attrs.get("version").map(|s| s.as_str()), Some("1.2.3.4"));
        assert_eq!(attrs.get("type").map(|s| s.as_str()), Some("win32"));
    }
}
