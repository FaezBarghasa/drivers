//! Registry Emulation
//!
//! Maps Windows Registry to Redox filesystem using sled database.
//! HKEY_LOCAL_MACHINE -> /windows/registry/machine
//! HKEY_CURRENT_USER -> /windows/registry/user/<uid>
//! HKEY_CLASSES_ROOT -> /windows/registry/classes

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;

use syscall::{Stat, MODE_DIR, MODE_FILE};

use crate::errno::NtStatus;
use crate::Handle; // Assuming Handle is u32 wrapper from main.rs or similar

/// Registry value types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RegType {
    None = 0,
    Sz = 1,       // String
    ExpandSz = 2, // Expandable string
    Binary = 3,   // Binary data
    Dword = 4,    // 32-bit number
    DwordBigEndian = 5,
    Link = 6,    // Symbolic link
    MultiSz = 7, // Multiple strings
    ResourceList = 8,
    FullResourceDescriptor = 9,
    ResourceRequirementsList = 10,
    Qword = 11, // 64-bit number
}

impl From<u32> for RegType {
    fn from(val: u32) -> Self {
        match val {
            1 => RegType::Sz,
            2 => RegType::ExpandSz,
            3 => RegType::Binary,
            4 => RegType::Dword,
            5 => RegType::DwordBigEndian,
            6 => RegType::Link,
            7 => RegType::MultiSz,
            11 => RegType::Qword,
            _ => RegType::None,
        }
    }
}

/// Registry value
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegValue {
    pub value_type: u32,
    pub data: Vec<u8>,
}

impl RegValue {
    pub fn string(s: &str) -> Self {
        let mut data: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        data.extend_from_slice(&[0, 0]); // Null terminator
        Self {
            value_type: RegType::Sz as u32,
            data,
        }
    }

    pub fn dword(val: u32) -> Self {
        Self {
            value_type: RegType::Dword as u32,
            data: val.to_le_bytes().to_vec(),
        }
    }

    pub fn qword(val: u64) -> Self {
        Self {
            value_type: RegType::Qword as u32,
            data: val.to_le_bytes().to_vec(),
        }
    }

    pub fn binary(data: Vec<u8>) -> Self {
        Self {
            value_type: RegType::Binary as u32,
            data,
        }
    }
}

/// Registry node type (handle target)
#[derive(Debug, Clone)]
pub enum RegNode {
    Key(String),           // Path to key
    Value(String, String), // Path to key, Value name
}

/// Registry handle manager using sled
pub struct Registry {
    /// Database
    db: sled::Db,
    /// Open handles (Handle -> Node info)
    open_handles: RwLock<BTreeMap<Handle, RegNode>>,
    /// Next handle value
    next_handle: std::sync::atomic::AtomicU32,
}

impl Registry {
    pub fn new(base_path: PathBuf) -> Self {
        let db = sled::open(base_path).expect("Failed to open registry database");

        Self {
            db,
            open_handles: RwLock::new(BTreeMap::new()),
            next_handle: std::sync::atomic::AtomicU32::new(0x80000000),
        }
    }

    /// Convert Windows registry path to normalized string key
    fn map_path(&self, key_path: &str) -> Result<String, NtStatus> {
        let key_path = key_path.trim_start_matches('\\');

        // Map predefined keys
        let path = if key_path.starts_with("REGISTRY\\MACHINE")
            || key_path.starts_with("HKEY_LOCAL_MACHINE")
        {
            let subpath = key_path
                .trim_start_matches("REGISTRY\\MACHINE")
                .trim_start_matches("HKEY_LOCAL_MACHINE")
                .trim_start_matches('\\');
            format!("machine/{}", subpath.replace('\\', "/"))
        } else if key_path.starts_with("REGISTRY\\USER")
            || key_path.starts_with("HKEY_CURRENT_USER")
        {
            let subpath = key_path
                .trim_start_matches("REGISTRY\\USER")
                .trim_start_matches("HKEY_CURRENT_USER")
                .trim_start_matches('\\');
            format!("user/{}", subpath.replace('\\', "/"))
        } else if key_path.starts_with("HKEY_CLASSES_ROOT") {
            let subpath = key_path
                .trim_start_matches("HKEY_CLASSES_ROOT")
                .trim_start_matches('\\');
            format!("classes/{}", subpath.replace('\\', "/"))
        } else {
            return Err(NtStatus::ObjectPathInvalid);
        };

        Ok(path.to_lowercase()) // Registry is case-insensitive usually
    }

    /// Open or create a registry key or value
    pub fn open(&self, path: &str, create: bool) -> Result<Handle, NtStatus> {
        let path = self.map_path(path)?;

        // Strategy:
        // 1. Check if it matches a Key
        // 2. Check if it matches a Value

        // For keys, we store a marker: "keys/<path>"
        let meta_key = format!("keys/{}", path);
        if self
            .db
            .contains_key(&meta_key)
            .map_err(|_| NtStatus::Unsuccessful)?
        {
            // It's a key
            let handle = self.alloc_handle();
            self.open_handles
                .write()
                .unwrap()
                .insert(handle, RegNode::Key(path));
            return Ok(handle);
        }

        // Check if it is a value: "values/<parent>/<name>"
        // The path passed to map_path is normalized.
        // We need to split the last component to check for value.
        if let Some(idx) = path.rfind('/') {
            let parent = &path[..idx];
            let name = &path[idx + 1..];

            let val_key = format!("values/{}/{}", parent, name);
            if self
                .db
                .contains_key(&val_key)
                .map_err(|_| NtStatus::Unsuccessful)?
            {
                // It's a value
                let handle = self.alloc_handle();
                self.open_handles
                    .write()
                    .unwrap()
                    .insert(handle, RegNode::Value(parent.to_string(), name.to_string()));
                return Ok(handle);
            }

            // If not found, and create is true, what do we create?
            // Scheme semantics: O_DIRECTORY -> create key?
            // Without O_DIRECTORY -> create value?
            // But map_path normalizes everything.

            // For now, if create is true, we fallback to creating a KEY if it looks like a directory semantics,
            // or logic in main.rs should hint.
            // But here we only have path.
            // Assumption: If it doesn't exist, and we want to create, we need to know what.

            // Simplification: We only support creating KEYs via `mkdir` (which calls Open?) no, mkdir calls mkdir.
            // Open(O_CREAT) with O_DIRECTORY creates key.
            // Open(O_CREAT) without O_DIRECTORY creates value.
            // We need to update signature of `open`.
        }

        Err(NtStatus::ObjectNameNotFound)
    }

    pub fn create_key(&self, path: &str) -> Result<Handle, NtStatus> {
        let path = self.map_path(path)?;
        let meta_key = format!("keys/{}", path);

        self.db
            .insert(&meta_key, b"created")
            .map_err(|_| NtStatus::Unsuccessful)?;

        let handle = self.alloc_handle();
        self.open_handles
            .write()
            .unwrap()
            .insert(handle, RegNode::Key(path));
        Ok(handle)
    }

    pub fn create_value(&self, path: &str) -> Result<Handle, NtStatus> {
        let path = self.map_path(path)?;
        if let Some(idx) = path.rfind('/') {
            let parent = &path[..idx];
            let name = &path[idx + 1..];

            // Ensure parent exists
            let parent_key = format!("keys/{}", parent);
            if !self
                .db
                .contains_key(&parent_key)
                .map_err(|_| NtStatus::Unsuccessful)?
            {
                return Err(NtStatus::ObjectNameNotFound); // Parent key must exist
            }

            // Create empty value if not exists
            let val_key = format!("values/{}/{}", parent, name);
            if !self
                .db
                .contains_key(&val_key)
                .map_err(|_| NtStatus::Unsuccessful)?
            {
                let default_val = RegValue::binary(vec![]);
                let encoded =
                    bincode::serialize(&default_val).map_err(|_| NtStatus::Unsuccessful)?;
                self.db
                    .insert(&val_key, encoded)
                    .map_err(|_| NtStatus::Unsuccessful)?;
            }

            let handle = self.alloc_handle();
            self.open_handles
                .write()
                .unwrap()
                .insert(handle, RegNode::Value(parent.to_string(), name.to_string()));
            Ok(handle)
        } else {
            Err(NtStatus::ObjectPathInvalid)
        }
    }

    fn alloc_handle(&self) -> Handle {
        let val = self
            .next_handle
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Handle(val)
    }

    /// Close a registry handle
    pub fn close(&self, handle: Handle) -> Result<(), NtStatus> {
        self.open_handles
            .write()
            .unwrap()
            .remove(&handle)
            .map(|_| ())
            .ok_or(NtStatus::InvalidHandle)
    }

    /// Set a value
    pub fn set_value(&self, handle: Handle, name: &str, value: RegValue) -> Result<(), NtStatus> {
        let keys = self.open_keys.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let db_key = format!("values/{}/{}", key.path, name.to_lowercase());

        let encoded = bincode::serialize(&value).map_err(|_| NtStatus::Unsuccessful)?;

        self.db
            .insert(db_key, encoded)
            .map_err(|_| NtStatus::Unsuccessful)?;

        Ok(())
    }

    /// Get a value
    pub fn get_value(&self, handle: Handle, name: &str) -> Result<RegValue, NtStatus> {
        let keys = self.open_keys.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let db_key = format!("values/{}/{}", key.path, name.to_lowercase());

        let data = self.db.get(db_key).map_err(|_| NtStatus::Unsuccessful)?;

        if let Some(bytes) = data {
            let val: RegValue = bincode::deserialize(&bytes).map_err(|_| NtStatus::Unsuccessful)?;
            Ok(val)
        } else {
            Err(NtStatus::ObjectNameNotFound)
        }
    }

    /// Delete a value by handle and name
    pub fn delete_value(&self, handle: Handle, name: &str) -> Result<(), NtStatus> {
        let keys = self.open_handles.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let db_key = format!("values/{}/{}", key.path, name.to_lowercase());
        self.db.remove(db_key).map_err(|_| NtStatus::Unsuccessful)?;

        Ok(())
    }

    /// Delete a key or value by path (scheme helper)
    pub fn unlink(&self, path: &str) -> Result<(), NtStatus> {
        let path = self.map_path(path)?;
        
        // Try value first
        if let Some(idx) = path.rfind('/') {
            let parent = &path[..idx];
            let name = &path[idx+1..];
            let val_key = format!("values/{}/{}", parent, name);
             if self.db.contains_key(&val_key).unwrap_or(false) {
                 self.db.remove(&val_key).map_err(|_| NtStatus::Unsuccessful)?;
                 return Ok(());
             }
        }
        
        // Try key (rmdir logic, but unlink might call it?)
        // In Redox, unlink is for files, rmdir for directories.
        // We'll stricter separation if we can, but Unlink often tries both in some FS implementations or returns EISDIR.
        // Let's return ObjectNameNotFound if not a value.
        Err(NtStatus::ObjectNameNotFound)
    }

    /// Remove directory (Key)
    pub fn rmdir(&self, path: &str) -> Result<(), NtStatus> {
        let path = self.map_path(path)?;
        let key_path = format!("keys/{}", path);
        
        // Check if exists
        if !self.db.contains_key(&key_path).unwrap_or(false) {
            return Err(NtStatus::ObjectNameNotFound);
        }
        
        // Check if empty?
        let prefix = format!("keys/{}/", path);
        if self.db.scan_prefix(&prefix).next().is_some() {
            return Err(NtStatus::Unsuccessful); // Not empty (Key has subkeys)
        }
        let val_prefix = format!("values/{}/", path);
        if self.db.scan_prefix(&val_prefix).next().is_some() {
             return Err(NtStatus::Unsuccessful); // Not empty (Key has values)
        }
        
        self.db.remove(&key_path).map_err(|_| NtStatus::Unsuccessful)?;
        Ok(())
    }

    /// Enumerate subkeys
    pub fn enumerate_keys(&self, handle: Handle) -> Result<Vec<String>, NtStatus> {
        let keys = self.open_keys.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let prefix = format!("keys/{}/", key.path);
        let mut subkeys = Vec::new();

        for item in self.db.scan_prefix(&prefix) {
            let (k, _) = item.map_err(|_| NtStatus::Unsuccessful)?;
            let k_str = std::str::from_utf8(&k).map_err(|_| NtStatus::Unsuccessful)?;

            // Extract direct child
            // k_str: keys/parent/child/grandchild
            // prefix: keys/parent/

            let relative = &k_str[prefix.len()..];
            if let Some(end) = relative.find('/') {
                // it's a grandchild, skip if we want direct children only?
                // Actually enumeration usually lists direct children.
                // This logic is simplified; a real impl might need better tree structure.
            } else {
                subkeys.push(relative.to_string());
            }
        }

        // simple dedup if needed, but here we assume keys are unique path strings
        Ok(subkeys)
    }

    /// Read from a handle (Directory listing for Key, Data for Value)
    pub fn read(&self, handle: Handle, buf: &mut [u8]) -> Result<usize, NtStatus> {
        let keys = self.open_handles.read().unwrap();
        let node = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        match node {
            RegNode::Key(path) => {
                // Directory listing: format as "keys\nvalues\n" ?
                // Or redox_scheme format?
                // Usually simply separate by newlines or NUL?
                // Let's use standard ls style: list keys then values.

                let prefix = format!("keys/{}/", path);
                let mut output = String::new();

                // List Subkeys
                for item in self.db.scan_prefix(&prefix) {
                    if let Ok((k, _)) = item {
                        if let Ok(k_str) = std::str::from_utf8(&k) {
                            let relative = &k_str[prefix.len()..];
                            if !relative.contains('/') {
                                output.push_str(relative);
                                output.push('\n');
                            }
                        }
                    }
                }

                // List Values
                let val_prefix = format!("values/{}/", path);
                for item in self.db.scan_prefix(&val_prefix) {
                    if let Ok((k, _)) = item {
                        if let Ok(k_str) = std::str::from_utf8(&k) {
                            let relative = &k_str[val_prefix.len()..];
                            output.push_str(relative);
                            output.push('\n');
                        }
                    }
                }

                let bytes = output.as_bytes();
                let len = std::cmp::min(buf.len(), bytes.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            RegNode::Value(parent, name) => {
                let db_key = format!("values/{}/{}", parent, name);
                if let Some(data) = self.db.get(&db_key).map_err(|_| NtStatus::Unsuccessful)? {
                    // We stored RegValue struct encoded.
                    // But Call::Read expects raw bytes of the content? Or the formatted value?
                    // If it's a file, we probably want the raw data.
                    // But RegValue has type info.
                    // Let's return the debug string representation for now or raw bytes if binary.

                    let val: RegValue =
                        bincode::deserialize(&data).map_err(|_| NtStatus::Unsuccessful)?;

                    // Helper to format
                    let content = match val.value_type {
                        1 => {
                            // SZ
                            // Convert UTF-16 to UTF-8
                            let u16s: Vec<u16> = val
                                .data
                                .chunks_exact(2)
                                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                                .collect();
                            String::from_utf16_lossy(&u16s)
                                .trim_matches('\0')
                                .to_string()
                        }
                        4 => {
                            // DWORD
                            if val.data.len() >= 4 {
                                let v = u32::from_le_bytes([
                                    val.data[0],
                                    val.data[1],
                                    val.data[2],
                                    val.data[3],
                                ]);
                                format!("{}", v)
                            } else {
                                "Invalid DWORD".to_string()
                            }
                        }
                        _ => format!("{:?}", val.data),
                    };

                    let bytes = content.as_bytes();
                    let len = std::cmp::min(buf.len(), bytes.len());
                    buf[..len].copy_from_slice(&bytes[..len]);
                    Ok(len)
                } else {
                    Ok(0)
                }
            }
        }
    /// Write to a handle (Value data)
    pub fn write(&self, handle: Handle, buf: &[u8]) -> Result<usize, NtStatus> {
        let keys = self.open_handles.read().unwrap();
        let node = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        match node {
            RegNode::Key(_) => Err(NtStatus::FileIsADirectory),
            RegNode::Value(parent, name) => {
                // Default to Binary type for raw writes
                let val = RegValue::binary(buf.to_vec());
                let db_key = format!("values/{}/{}", parent, name);
                
                let encoded = bincode::serialize(&val).map_err(|_| NtStatus::Unsuccessful)?;
                self.db.insert(db_key, encoded).map_err(|_| NtStatus::Unsuccessful)?;
                
                Ok(buf.len())
            }
        }
    }

    /// Get file statistics
    pub fn fstat(&self, handle: Handle, stat: &mut syscall::Stat) -> Result<usize, NtStatus> {
        let keys = self.open_handles.read().unwrap();
        let node = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        match node {
            RegNode::Key(_) => {
                stat.st_mode = syscall::MODE_DIR | 0o755;
                stat.st_size = 0;
                Ok(0)
            },
            RegNode::Value(parent, name) => {
                stat.st_mode = syscall::MODE_FILE | 0o644;
                let db_key = format!("values/{}/{}", parent, name);
                if let Some(data) = self.db.get(&db_key).map_err(|_| NtStatus::Unsuccessful)? {
                     // We need the ACTUAL size of the data, not the serialized struct?
                     // Or just the serialized size?
                     // Usually ls -l usage.
                     // Let's decode to get real data size.
                     if let Ok(val) = bincode::deserialize::<RegValue>(&data) {
                         stat.st_size = val.data.len() as u64;
                     } else {
                         stat.st_size = data.len() as u64; 
                     }
                } else {
                    stat.st_size = 0;
                }
                Ok(0)
            }
        }
    }
}
