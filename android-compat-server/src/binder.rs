//! Binder IPC Bridge
//!
//! Translates Android Binder IPC to Redox IPC mechanisms.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

/// Binder transaction codes
pub mod transaction {
    pub const FIRST_CALL: u32 = 0x00000001;
    pub const LAST_CALL: u32 = 0x00ffffff;
    pub const PING: u32 = 0x5f504e47; // '_PNG'
    pub const DUMP: u32 = 0x5f444d50; // '_DMP'
    pub const INTERFACE: u32 = 0x5f4e5446; // '_NTF'
}

/// Binder transaction flags
pub mod flags {
    pub const ONE_WAY: u32 = 0x01;
    pub const ROOT_OBJECT: u32 = 0x04;
    pub const STATUS_CODE: u32 = 0x08;
    pub const ACCEPT_FDS: u32 = 0x10;
}

/// Binder handle type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BinderHandle(pub u32);

impl BinderHandle {
    /// Context manager (servicemanager) handle
    pub const CONTEXT_MANAGER: BinderHandle = BinderHandle(0);
}

/// Binder transaction data
#[derive(Debug, Clone)]
pub struct BinderTransaction {
    pub target: BinderHandle,
    pub code: u32,
    pub flags: u32,
    pub sender_pid: u32,
    pub sender_euid: u32,
    pub data: Vec<u8>,
    pub offsets: Vec<usize>,
}

impl BinderTransaction {
    pub fn new(target: BinderHandle, code: u32) -> Self {
        Self {
            target,
            code,
            flags: 0,
            sender_pid: 0,
            sender_euid: 0,
            data: Vec::new(),
            offsets: Vec::new(),
        }
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn one_way(mut self) -> Self {
        self.flags |= flags::ONE_WAY;
        self
    }
}

/// Binder reply data
#[derive(Debug, Clone)]
pub struct BinderReply {
    pub status: i32,
    pub data: Vec<u8>,
}

impl BinderReply {
    pub fn success(data: Vec<u8>) -> Self {
        Self { status: 0, data }
    }

    pub fn error(status: i32) -> Self {
        Self {
            status,
            data: Vec::new(),
        }
    }
}

/// Parcel writer for serializing transaction data
pub struct Parcel {
    data: Vec<u8>,
    objects: Vec<usize>,
}

impl Parcel {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            objects: Vec::new(),
        }
    }

    pub fn write_i32(&mut self, value: i32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_string(&mut self, s: &str) {
        // Android strings are length-prefixed UTF-16
        let utf16: Vec<u16> = s.encode_utf16().collect();
        self.write_i32(utf16.len() as i32);
        for c in utf16 {
            self.data.extend_from_slice(&c.to_le_bytes());
        }
        // Null terminator
        self.data.extend_from_slice(&[0u8, 0u8]);
        // Padding to 4-byte boundary
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
    }

    pub fn write_interface_token(&mut self, interface: &str) {
        // Strict mode policy
        self.write_i32(0x010000); // STRICT_MODE_PENALTY_GATHER
                                  // Interface name
        self.write_string(interface);
    }

    pub fn write_binder(&mut self, handle: BinderHandle) {
        self.objects.push(self.data.len());
        // Flat binder object
        self.write_u32(0x73622a85); // BINDER_TYPE_HANDLE
        self.write_u32(0); // flags
        self.write_u32(handle.0); // handle
        self.write_u32(0); // cookie low
        self.write_u32(0); // cookie high
    }

    pub fn to_bytes(self) -> (Vec<u8>, Vec<usize>) {
        (self.data, self.objects)
    }
}

impl Default for Parcel {
    fn default() -> Self {
        Self::new()
    }
}

/// Parcel reader for deserializing reply data
pub struct ParcelReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ParcelReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn read_i32(&mut self) -> Option<i32> {
        if self.pos + 4 > self.data.len() {
            return None;
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Some(i32::from_le_bytes(bytes))
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        self.read_i32().map(|v| v as u32)
    }

    pub fn read_string(&mut self) -> Option<String> {
        let len = self.read_i32()? as usize;
        if len == 0 {
            return Some(String::new());
        }

        let byte_len = len * 2;
        if self.pos + byte_len > self.data.len() {
            return None;
        }

        let utf16: Vec<u16> = self.data[self.pos..self.pos + byte_len]
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        self.pos += byte_len + 2; // +2 for null terminator
                                  // Align to 4 bytes
        self.pos = (self.pos + 3) & !3;

        Some(String::from_utf16_lossy(&utf16))
    }
}

/// Registered binder service
struct BinderService {
    name: String,
    handle: BinderHandle,
    owner_pid: u32,
}

/// Service Manager - maintains registry of system services
pub struct ServiceManager {
    services: RwLock<BTreeMap<String, BinderService>>,
    next_handle: AtomicU32,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            services: RwLock::new(BTreeMap::new()),
            next_handle: AtomicU32::new(1),
        }
    }

    /// Get a service by name
    pub fn get_service(&self, name: &str) -> Option<BinderHandle> {
        self.services.read().unwrap().get(name).map(|s| s.handle)
    }

    /// Add a service
    pub fn add_service(&self, name: String, owner_pid: u32) -> BinderHandle {
        let handle = BinderHandle(self.next_handle.fetch_add(1, Ordering::Relaxed));

        let service = BinderService {
            name: name.clone(),
            handle,
            owner_pid,
        };

        self.services.write().unwrap().insert(name, service);
        handle
    }

    /// List all services
    pub fn list_services(&self) -> Vec<String> {
        self.services.read().unwrap().keys().cloned().collect()
    }

    /// Handle a transaction to the service manager
    pub fn handle_transaction(&self, tx: &BinderTransaction) -> BinderReply {
        match tx.code {
            1 => {
                // GET_SERVICE
                let mut reader = ParcelReader::new(&tx.data);
                // Skip interface token
                let _ = reader.read_i32();
                let _ = reader.read_string();

                if let Some(name) = reader.read_string() {
                    if let Some(handle) = self.get_service(&name) {
                        let mut reply = Parcel::new();
                        reply.write_binder(handle);
                        let (data, _) = reply.to_bytes();
                        return BinderReply::success(data);
                    }
                }
                BinderReply::error(-1) // Service not found
            }
            2 => {
                // CHECK_SERVICE
                // Same as GET_SERVICE but doesn't block
                self.handle_transaction(&BinderTransaction {
                    code: 1,
                    ..tx.clone()
                })
            }
            3 => {
                // ADD_SERVICE
                let mut reader = ParcelReader::new(&tx.data);
                let _ = reader.read_i32();
                let _ = reader.read_string();

                if let Some(name) = reader.read_string() {
                    let handle = self.add_service(name, tx.sender_pid);
                    let mut reply = Parcel::new();
                    reply.write_i32(0); // Success
                    let (data, _) = reply.to_bytes();
                    return BinderReply::success(data);
                }
                BinderReply::error(-1)
            }
            4 => {
                // LIST_SERVICES
                let services = self.list_services();
                let mut reply = Parcel::new();
                reply.write_i32(services.len() as i32);
                for name in services {
                    reply.write_string(&name);
                }
                let (data, _) = reply.to_bytes();
                BinderReply::success(data)
            }
            _ => BinderReply::error(-1), // Unknown transaction
        }
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
