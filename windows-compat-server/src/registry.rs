//! Registry Emulation
//!
//! Maps Windows Registry to Redox filesystem.
//! HKEY_LOCAL_MACHINE -> /windows/registry/machine
//! HKEY_CURRENT_USER -> /windows/registry/user/<uid>
//! HKEY_CLASSES_ROOT -> /windows/registry/classes

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::errno::NtStatus;
use crate::Handle;

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
#[derive(Debug, Clone)]
pub struct RegValue {
    pub value_type: RegType,
    pub data: Vec<u8>,
}

impl RegValue {
    pub fn string(s: &str) -> Self {
        let mut data: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        data.extend_from_slice(&[0, 0]); // Null terminator
        Self {
            value_type: RegType::Sz,
            data,
        }
    }

    pub fn dword(val: u32) -> Self {
        Self {
            value_type: RegType::Dword,
            data: val.to_le_bytes().to_vec(),
        }
    }

    pub fn qword(val: u64) -> Self {
        Self {
            value_type: RegType::Qword,
            data: val.to_le_bytes().to_vec(),
        }
    }

    pub fn binary(data: Vec<u8>) -> Self {
        Self {
            value_type: RegType::Binary,
            data,
        }
    }
}

/// Registry key (maps to directory)
#[derive(Debug)]
pub struct RegKey {
    pub path: PathBuf,
    pub handle: Handle,
}

/// Registry handle manager
pub struct Registry {
    /// Base path for registry storage
    base_path: PathBuf,
    /// Open keys
    open_keys: RwLock<BTreeMap<Handle, RegKey>>,
    /// Next handle value
    next_handle: std::sync::atomic::AtomicU32,
}

impl Registry {
    pub fn new(base_path: PathBuf) -> Self {
        // Ensure base directories exist
        let _ = fs::create_dir_all(base_path.join("machine"));
        let _ = fs::create_dir_all(base_path.join("user"));
        let _ = fs::create_dir_all(base_path.join("classes"));

        Self {
            base_path,
            open_keys: RwLock::new(BTreeMap::new()),
            next_handle: std::sync::atomic::AtomicU32::new(0x80000000),
        }
    }

    /// Convert Windows registry path to Redox path
    fn map_path(&self, key_path: &str) -> Result<PathBuf, NtStatus> {
        let key_path = key_path.trim_start_matches('\\');

        // Map predefined keys
        let path = if key_path.starts_with("REGISTRY\\MACHINE")
            || key_path.starts_with("HKEY_LOCAL_MACHINE")
        {
            let subpath = key_path
                .trim_start_matches("REGISTRY\\MACHINE")
                .trim_start_matches("HKEY_LOCAL_MACHINE")
                .trim_start_matches('\\');
            self.base_path
                .join("machine")
                .join(subpath.replace('\\', "/"))
        } else if key_path.starts_with("REGISTRY\\USER")
            || key_path.starts_with("HKEY_CURRENT_USER")
        {
            let subpath = key_path
                .trim_start_matches("REGISTRY\\USER")
                .trim_start_matches("HKEY_CURRENT_USER")
                .trim_start_matches('\\');
            self.base_path.join("user").join(subpath.replace('\\', "/"))
        } else if key_path.starts_with("HKEY_CLASSES_ROOT") {
            let subpath = key_path
                .trim_start_matches("HKEY_CLASSES_ROOT")
                .trim_start_matches('\\');
            self.base_path
                .join("classes")
                .join(subpath.replace('\\', "/"))
        } else {
            return Err(NtStatus::ObjectPathInvalid);
        };

        Ok(path)
    }

    /// Open or create a registry key
    pub fn open_key(&self, key_path: &str, create: bool) -> Result<Handle, NtStatus> {
        let path = self.map_path(key_path)?;

        if create {
            fs::create_dir_all(&path).map_err(|_| NtStatus::AccessDenied)?;
        } else if !path.exists() {
            return Err(NtStatus::ObjectNameNotFound);
        }

        let handle_val = self
            .next_handle
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let handle = Handle(handle_val);

        let key = RegKey { path, handle };

        self.open_keys.write().unwrap().insert(handle, key);
        Ok(handle)
    }

    /// Close a registry key
    pub fn close_key(&self, handle: Handle) -> Result<(), NtStatus> {
        self.open_keys
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

        let value_path = key.path.join(format!("{}.regval", name));

        // Write header: type (4 bytes) + data
        let mut file = File::create(&value_path).map_err(|_| NtStatus::AccessDenied)?;
        file.write_all(&(value.value_type as u32).to_le_bytes())
            .map_err(|_| NtStatus::AccessDenied)?;
        file.write_all(&value.data)
            .map_err(|_| NtStatus::AccessDenied)?;

        Ok(())
    }

    /// Get a value
    pub fn get_value(&self, handle: Handle, name: &str) -> Result<RegValue, NtStatus> {
        let keys = self.open_keys.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let value_path = key.path.join(format!("{}.regval", name));

        let mut file = File::open(&value_path).map_err(|_| NtStatus::ObjectNameNotFound)?;

        // Read header
        let mut type_buf = [0u8; 4];
        file.read_exact(&mut type_buf)
            .map_err(|_| NtStatus::ObjectNameNotFound)?;
        let value_type = RegType::from(u32::from_le_bytes(type_buf));

        // Read data
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|_| NtStatus::ObjectNameNotFound)?;

        Ok(RegValue { value_type, data })
    }

    /// Delete a value
    pub fn delete_value(&self, handle: Handle, name: &str) -> Result<(), NtStatus> {
        let keys = self.open_keys.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let value_path = key.path.join(format!("{}.regval", name));
        fs::remove_file(&value_path).map_err(|_| NtStatus::ObjectNameNotFound)?;

        Ok(())
    }

    /// Enumerate subkeys
    pub fn enumerate_keys(&self, handle: Handle) -> Result<Vec<String>, NtStatus> {
        let keys = self.open_keys.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let mut subkeys = Vec::new();
        for entry in fs::read_dir(&key.path).map_err(|_| NtStatus::AccessDenied)? {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        subkeys.push(name.to_string());
                    }
                }
            }
        }

        Ok(subkeys)
    }

    /// Enumerate values
    pub fn enumerate_values(&self, handle: Handle) -> Result<Vec<String>, NtStatus> {
        let keys = self.open_keys.read().unwrap();
        let key = keys.get(&handle).ok_or(NtStatus::InvalidHandle)?;

        let mut values = Vec::new();
        for entry in fs::read_dir(&key.path).map_err(|_| NtStatus::AccessDenied)? {
            if let Ok(entry) = entry {
                if entry.path().is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if let Some(name) = name.strip_suffix(".regval") {
                            values.push(name.to_string());
                        }
                    }
                }
            }
        }

        Ok(values)
    }
}
