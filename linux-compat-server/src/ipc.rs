//! IPC mechanisms for LAC server
//!
//! This module provides IPC between the LAC server and native Redox servers.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::errno::LinuxErrno;

/// IPC client for communicating with Redox schemes
pub struct SchemeClient {
    /// Scheme path
    path: String,
    /// Open file handles
    handles: HashMap<usize, File>,
    /// Next handle ID
    next_handle: usize,
}

impl SchemeClient {
    /// Create a new scheme client
    pub fn new(scheme: &str) -> Self {
        Self {
            path: format!("{}:", scheme),
            handles: HashMap::new(),
            next_handle: 0,
        }
    }

    /// Open a path on this scheme
    pub fn open(&mut self, path: &str, flags: i32) -> Result<usize, LinuxErrno> {
        let full_path = format!("{}{}", self.path, path);

        let file = std::fs::OpenOptions::new()
            .read((flags & 0o3) != 1) // O_WRONLY = 1
            .write((flags & 0o3) != 0) // O_RDONLY = 0
            .create((flags & 0o100) != 0) // O_CREAT
            .truncate((flags & 0o1000) != 0) // O_TRUNC
            .append((flags & 0o2000) != 0) // O_APPEND
            .open(&full_path)
            .map_err(|_| LinuxErrno::ENOENT)?;

        let handle = self.next_handle;
        self.next_handle += 1;
        self.handles.insert(handle, file);

        Ok(handle)
    }

    /// Close a handle
    pub fn close(&mut self, handle: usize) -> Result<(), LinuxErrno> {
        if self.handles.remove(&handle).is_some() {
            Ok(())
        } else {
            Err(LinuxErrno::EBADF)
        }
    }

    /// Read from a handle
    pub fn read(&mut self, handle: usize, buf: &mut [u8]) -> Result<usize, LinuxErrno> {
        if let Some(file) = self.handles.get_mut(&handle) {
            file.read(buf).map_err(|_| LinuxErrno::EIO)
        } else {
            Err(LinuxErrno::EBADF)
        }
    }

    /// Write to a handle
    pub fn write(&mut self, handle: usize, buf: &[u8]) -> Result<usize, LinuxErrno> {
        if let Some(file) = self.handles.get_mut(&handle) {
            file.write(buf).map_err(|_| LinuxErrno::EIO)
        } else {
            Err(LinuxErrno::EBADF)
        }
    }
}

/// File system IPC client
pub struct FsClient {
    client: SchemeClient,
}

impl FsClient {
    /// Create a new file system client
    pub fn new() -> Self {
        Self {
            client: SchemeClient::new("file"),
        }
    }

    /// Open a file
    pub fn open(&mut self, path: &str, flags: i32, mode: u32) -> Result<usize, LinuxErrno> {
        self.client.open(path, flags)
    }

    /// Close a file
    pub fn close(&mut self, handle: usize) -> Result<(), LinuxErrno> {
        self.client.close(handle)
    }

    /// Read from a file
    pub fn read(&mut self, handle: usize, buf: &mut [u8]) -> Result<usize, LinuxErrno> {
        self.client.read(handle, buf)
    }

    /// Write to a file
    pub fn write(&mut self, handle: usize, buf: &[u8]) -> Result<usize, LinuxErrno> {
        self.client.write(handle, buf)
    }

    /// Check if a path exists
    pub fn exists(&self, path: &str) -> bool {
        Path::new(&format!("file:{}", path)).exists()
    }

    /// Check file access
    pub fn access(&self, path: &str, mode: i32) -> Result<(), LinuxErrno> {
        let full_path = format!("file:{}", path);

        if !Path::new(&full_path).exists() {
            return Err(LinuxErrno::ENOENT);
        }

        // Would check actual permissions
        Ok(())
    }

    /// Create a directory
    pub fn mkdir(&self, path: &str, mode: u32) -> Result<(), LinuxErrno> {
        std::fs::create_dir(format!("file:{}", path)).map_err(|_| LinuxErrno::EIO)
    }

    /// Remove a directory
    pub fn rmdir(&self, path: &str) -> Result<(), LinuxErrno> {
        std::fs::remove_dir(format!("file:{}", path)).map_err(|_| LinuxErrno::EIO)
    }

    /// Remove a file
    pub fn unlink(&self, path: &str) -> Result<(), LinuxErrno> {
        std::fs::remove_file(format!("file:{}", path)).map_err(|_| LinuxErrno::EIO)
    }
}

impl Default for FsClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Network IPC client
pub struct NetClient {
    client: SchemeClient,
}

impl NetClient {
    /// Create a new network client (TCP)
    pub fn new_tcp() -> Self {
        Self {
            client: SchemeClient::new("tcp"),
        }
    }

    /// Create a new network client (UDP)
    pub fn new_udp() -> Self {
        Self {
            client: SchemeClient::new("udp"),
        }
    }

    /// Connect to an address
    pub fn connect(&mut self, addr: &str) -> Result<usize, LinuxErrno> {
        self.client.open(addr, 2) // O_RDWR
    }

    /// Listen on an address
    pub fn listen(&mut self, addr: &str) -> Result<usize, LinuxErrno> {
        self.client.open(addr, 0) // O_RDONLY for listening
    }

    /// Accept a connection
    pub fn accept(&mut self, _listen_handle: usize) -> Result<usize, LinuxErrno> {
        // Would accept a connection on the listen socket
        Err(LinuxErrno::ENOSYS)
    }

    /// Send data
    pub fn send(&mut self, handle: usize, buf: &[u8]) -> Result<usize, LinuxErrno> {
        self.client.write(handle, buf)
    }

    /// Receive data
    pub fn recv(&mut self, handle: usize, buf: &mut [u8]) -> Result<usize, LinuxErrno> {
        self.client.read(handle, buf)
    }

    /// Close a socket
    pub fn close(&mut self, handle: usize) -> Result<(), LinuxErrno> {
        self.client.close(handle)
    }
}

/// Proc filesystem client (for /proc emulation)
pub struct ProcClient {
    client: SchemeClient,
}

impl ProcClient {
    /// Create a new proc client
    pub fn new() -> Self {
        Self {
            client: SchemeClient::new("proc"),
        }
    }

    /// Read process info
    pub fn read_proc(&mut self, pid: u32, entry: &str) -> Result<Vec<u8>, LinuxErrno> {
        let path = format!("{}/{}", pid, entry);
        let handle = self.client.open(&path, 0)?;

        let mut data = Vec::new();
        let mut buf = [0u8; 4096];

        loop {
            let n = self.client.read(handle, &mut buf)?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
        }

        self.client.close(handle)?;
        Ok(data)
    }

    /// Get process status
    pub fn get_status(&mut self, pid: u32) -> Result<ProcStatus, LinuxErrno> {
        let data = self.read_proc(pid, "status")?;
        // Would parse status file
        Ok(ProcStatus::default())
    }
}

impl Default for ProcClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Process status from /proc/[pid]/status
#[derive(Debug, Default)]
pub struct ProcStatus {
    pub name: String,
    pub state: char,
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub vm_size: u64,
    pub vm_rss: u64,
    pub threads: u32,
}

/// Pipe IPC
pub struct PipeIpc {
    /// Next pipe ID
    next_id: usize,
    /// Active pipes (id -> (read_buffer, write_buffer))
    pipes: HashMap<usize, PipeBuffer>,
}

/// Pipe buffer
struct PipeBuffer {
    data: std::collections::VecDeque<u8>,
    capacity: usize,
    closed_read: bool,
    closed_write: bool,
}

impl PipeBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            data: std::collections::VecDeque::with_capacity(capacity),
            capacity,
            closed_read: false,
            closed_write: false,
        }
    }
}

impl PipeIpc {
    /// Create a new pipe IPC manager
    pub fn new() -> Self {
        Self {
            next_id: 0,
            pipes: HashMap::new(),
        }
    }

    /// Create a pipe, returns (read_end_id, write_end_id)
    pub fn create_pipe(&mut self) -> (usize, usize) {
        let id = self.next_id;
        self.next_id += 2; // Two ends

        self.pipes.insert(id, PipeBuffer::new(65536));

        (id, id + 1) // read end, write end
    }

    /// Write to a pipe
    pub fn write(&mut self, write_end: usize, data: &[u8]) -> Result<usize, LinuxErrno> {
        let pipe_id = write_end - 1; // Write end is id + 1

        let pipe = self.pipes.get_mut(&pipe_id).ok_or(LinuxErrno::EBADF)?;

        if pipe.closed_read {
            return Err(LinuxErrno::EPIPE);
        }

        let available = pipe.capacity - pipe.data.len();
        let to_write = data.len().min(available);

        pipe.data.extend(&data[..to_write]);

        Ok(to_write)
    }

    /// Read from a pipe
    pub fn read(&mut self, read_end: usize, buf: &mut [u8]) -> Result<usize, LinuxErrno> {
        let pipe = self.pipes.get_mut(&read_end).ok_or(LinuxErrno::EBADF)?;

        if pipe.data.is_empty() {
            if pipe.closed_write {
                return Ok(0); // EOF
            }
            return Err(LinuxErrno::EAGAIN);
        }

        let to_read = buf.len().min(pipe.data.len());
        for i in 0..to_read {
            buf[i] = pipe.data.pop_front().unwrap();
        }

        Ok(to_read)
    }

    /// Close a pipe end
    pub fn close(&mut self, end: usize) -> Result<(), LinuxErrno> {
        // Determine if read or write end
        if end % 2 == 0 {
            // Read end
            if let Some(pipe) = self.pipes.get_mut(&end) {
                pipe.closed_read = true;
            }
        } else {
            // Write end
            let pipe_id = end - 1;
            if let Some(pipe) = self.pipes.get_mut(&pipe_id) {
                pipe.closed_write = true;
            }
        }

        // Clean up if both ends closed
        let pipe_id = if end % 2 == 0 { end } else { end - 1 };
        if let Some(pipe) = self.pipes.get(&pipe_id) {
            if pipe.closed_read && pipe.closed_write {
                self.pipes.remove(&pipe_id);
            }
        }

        Ok(())
    }
}

impl Default for PipeIpc {
    fn default() -> Self {
        Self::new()
    }
}

/// Unix socket emulation
pub struct UnixSocketClient {
    /// Socket path to buffer mapping
    sockets: HashMap<String, UnixSocket>,
    /// Next socket ID
    next_id: usize,
}

struct UnixSocket {
    id: usize,
    path: String,
    listening: bool,
    connected: bool,
    read_buffer: std::collections::VecDeque<u8>,
    write_buffer: std::collections::VecDeque<u8>,
}

impl UnixSocketClient {
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
            next_id: 0,
        }
    }

    /// Create a Unix socket
    pub fn socket(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Bind a socket to a path
    pub fn bind(&mut self, id: usize, path: &str) -> Result<(), LinuxErrno> {
        self.sockets.insert(
            path.to_string(),
            UnixSocket {
                id,
                path: path.to_string(),
                listening: false,
                connected: false,
                read_buffer: std::collections::VecDeque::new(),
                write_buffer: std::collections::VecDeque::new(),
            },
        );
        Ok(())
    }

    /// Listen on a socket
    pub fn listen(&mut self, path: &str) -> Result<(), LinuxErrno> {
        if let Some(socket) = self.sockets.get_mut(path) {
            socket.listening = true;
            Ok(())
        } else {
            Err(LinuxErrno::EBADF)
        }
    }

    /// Connect to a socket
    pub fn connect(&mut self, path: &str) -> Result<usize, LinuxErrno> {
        if self.sockets.contains_key(path) {
            let id = self.socket();
            // Would establish connection
            Ok(id)
        } else {
            Err(LinuxErrno::ENOENT)
        }
    }
}

impl Default for UnixSocketClient {
    fn default() -> Self {
        Self::new()
    }
}
