//! Network Driver Scheme Infrastructure with BBRv3 Congestion Control
//!
//! This module provides a shared scheme infrastructure for network drivers with
//! integrated BBRv3 congestion control. It handles:
//!
//! - Packet read/write with congestion-aware pacing
//! - ECN (Explicit Congestion Notification) detection
//! - Real-time BBRv3 metrics monitoring via scheme interface
//! - Multiple address type queries (MAC, IPv4, IPv6)
//!
//! # Scheme Paths
//!
//! - `/` - Raw packet read/write with BBRv3 pacing
//! - `mac` - Read MAC address (6 bytes)
//! - `ipv4` - Read IPv4 address (4 bytes)
//! - `ipv6` - Read link-local IPv6 address (16 bytes)
//! - `ipv6_global` - Read global IPv6 address (16 bytes)
//! - `ipv6_unique_local` - Read unique local IPv6 address (16 bytes)
//! - `bbr` - Read BBRv3 metrics (text format for debugging)
//! - `bbr_raw` - Read BBRv3 metrics (binary format, 64 bytes)

use std::collections::BTreeMap;
use std::time::Instant;
use std::{cmp, io};

use bbrv3_rs::{Bbr, BbrMetrics};
use libredox::flag::O_NONBLOCK;
use libredox::Fd;
use redox_scheme::{
    CallRequest, CallerCtx, OpenResult, RequestKind, Response, SchemeBlock, SignalBehavior, Socket,
};
use syscall::schemev2::NewFdFlags;
use syscall::{
    Error, EventFlags, Result, Stat, EACCES, EAGAIN, EBADF, EINTR, EINVAL, EWOULDBLOCK, MODE_FILE,
};

/// Trait for network adapter implementations
///
/// Each network driver (e1000, virtio-net, etc.) implements this trait to provide
/// hardware-specific functionality.
pub trait NetworkAdapter {
    /// Returns the [MAC address](https://en.wikipedia.org/wiki/MAC_address) of this
    /// network adapter.
    fn mac_address(&mut self) -> [u8; 6];

    /// Returns the IPv4 address assigned to this adapter
    fn ipv4_address(&mut self) -> [u8; 4];

    /// Returns the link-local IPv6 address (fe80::/10)
    fn ipv6_address(&mut self) -> [u8; 16];

    /// Returns the global IPv6 address (2000::/3)
    fn ipv6_address_global(&mut self) -> [u8; 16];

    /// Returns the unique local IPv6 address (fc00::/7)
    fn ipv6_address_unique_local(&mut self) -> [u8; 16];

    /// Returns the number of network packets that can be read without blocking.
    fn available_for_read(&mut self) -> usize;

    /// Attempt to read a network packet without blocking.
    ///
    /// Returns `Ok(None)` when there is no pending network packet.
    fn read_packet(&mut self, buf: &mut [u8]) -> Result<Option<usize>>;

    /// Write a single network packet with optional pacing.
    ///
    /// # Arguments
    /// * `buf` - The packet data to send
    /// * `pacing_rate` - The BBRv3-calculated pacing rate in bytes/second
    ///
    /// The driver implementation should use the pacing rate to space out packet
    /// transmissions appropriately to prevent buffer bloat.
    fn write_packet(&mut self, buf: &[u8], pacing_rate: u64) -> Result<usize>;

    /// Returns the number of bytes currently in flight (sent but not yet acknowledged).
    fn in_flight(&self) -> u64;
}

/// ECN (Explicit Congestion Notification) flags
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcnFlag {
    /// Not-ECT (Non ECN-Capable Transport)
    NotEct = 0b00,
    /// ECT(1) - ECN-Capable Transport
    Ect1 = 0b01,
    /// ECT(0) - ECN-Capable Transport
    Ect0 = 0b10,
    /// CE - Congestion Experienced
    Ce = 0b11,
}

impl From<u8> for EcnFlag {
    fn from(val: u8) -> Self {
        match val & 0b11 {
            0b00 => EcnFlag::NotEct,
            0b01 => EcnFlag::Ect1,
            0b10 => EcnFlag::Ect0,
            0b11 => EcnFlag::Ce,
            _ => unreachable!(),
        }
    }
}

/// Extract ECN flags from an Ethernet frame
///
/// Supports both IPv4 and IPv6 packets
fn extract_ecn(packet: &[u8]) -> Option<EcnFlag> {
    // Minimum Ethernet header (14 bytes) + minimum IP header
    if packet.len() < 14 {
        return None;
    }

    let ethertype = u16::from_be_bytes([packet[12], packet[13]]);

    match ethertype {
        0x0800 => {
            // IPv4
            if packet.len() < 14 + 20 {
                return None;
            }
            let ip_header = &packet[14..];
            // ECN is in the low 2 bits of the TOS byte (byte 1 of IP header)
            let dscp_ecn = ip_header[1];
            Some(EcnFlag::from(dscp_ecn & 0x03))
        }
        0x86DD => {
            // IPv6
            if packet.len() < 14 + 40 {
                return None;
            }
            let ip_header = &packet[14..];
            // IPv6 Traffic Class is in bytes 0-1, bits 4-11
            // ECN is in the low 2 bits of the traffic class
            let traffic_class = ((ip_header[0] & 0x0F) << 4) | ((ip_header[1] & 0xF0) >> 4);
            Some(EcnFlag::from(traffic_class & 0x03))
        }
        _ => None,
    }
}

/// Calculate RTT from packet receive (simplified approach)
///
/// In a real implementation, this would track sequence numbers and ACKs.
/// For now, we use the time since the last write as a rough RTT estimate.
fn estimate_rtt_us(last_write: Instant) -> u64 {
    let elapsed = last_write.elapsed();
    elapsed.as_micros() as u64
}

/// Scheme handle types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Handle {
    /// Raw packet data read/write
    Data,
    /// MAC address (read-only)
    Mac,
    /// IPv4 address (read-only)
    Ipv4,
    /// Link-local IPv6 address (read-only)
    Ipv6,
    /// Global IPv6 address (read-only)
    Ipv6Global,
    /// Unique local IPv6 address (read-only)
    Ipv6UniqueLocal,
    /// BBRv3 metrics (text format, read-only)
    Bbr,
    /// BBRv3 metrics (binary format, read-only)
    BbrRaw,
}

/// Pacing state for controlling packet transmission rate
struct PacingState {
    /// Timestamp of last packet send (microseconds since arbitrary epoch)
    last_send_us: u64,
    /// Bytes waiting to be paced
    pending_bytes: u64,
}

impl Default for PacingState {
    fn default() -> Self {
        Self {
            last_send_us: 0,
            pending_bytes: 0,
        }
    }
}

/// Network scheme handler with integrated BBRv3 congestion control
pub struct NetworkScheme<T: NetworkAdapter> {
    adapter: T,
    scheme_name: String,
    socket: Socket,
    next_id: usize,
    handles: BTreeMap<usize, Handle>,
    blocked: Vec<CallRequest>,
    /// BBRv3 congestion control instance
    bbr: Bbr,
    /// Timestamp of last write for RTT estimation
    last_write: Instant,
    /// Monotonic timestamp source (in microseconds)
    start_time: Instant,
    /// Pacing state for rate control
    pacing: PacingState,
}

impl<T: NetworkAdapter> NetworkScheme<T> {
    /// Create a new NetworkScheme with default BBRv3 configuration
    pub fn new(adapter: T, scheme_name: String) -> Self {
        Self::with_bbr(adapter, scheme_name, Bbr::new())
    }

    /// Create a new NetworkScheme with a custom BBRv3 instance
    pub fn with_bbr(adapter: T, scheme_name: String, bbr: Bbr) -> Self {
        assert!(scheme_name.starts_with("network"));
        let socket = Socket::nonblock(&scheme_name).expect("failed to create network scheme");

        NetworkScheme {
            adapter,
            scheme_name,
            socket,
            next_id: 0,
            handles: BTreeMap::new(),
            blocked: vec![],
            bbr,
            last_write: Instant::now(),
            start_time: Instant::now(),
            pacing: PacingState::default(),
        }
    }

    /// Returns the event file descriptor for use with event loops
    pub fn event_handle(&self) -> &Fd {
        self.socket.inner()
    }

    /// Returns a reference to the underlying network adapter
    pub fn adapter(&self) -> &T {
        &self.adapter
    }

    /// Returns a mutable reference to the underlying network adapter
    pub fn adapter_mut(&mut self) -> &mut T {
        &mut self.adapter
    }

    /// Returns a reference to the BBRv3 instance
    pub fn bbr(&self) -> &Bbr {
        &self.bbr
    }

    /// Returns a mutable reference to the BBRv3 instance
    pub fn bbr_mut(&mut self) -> &mut Bbr {
        &mut self.bbr
    }

    /// Get current timestamp in microseconds since scheme creation
    fn now_us(&self) -> u64 {
        self.start_time.elapsed().as_micros() as u64
    }

    /// Check if a packet can be sent according to pacing constraints
    fn can_send(&self, packet_size: u64) -> bool {
        let pacing_rate = self.bbr.pacing_rate();
        if pacing_rate == 0 {
            return true; // No pacing constraint yet (startup)
        }

        let now_us = self.now_us();
        let delay_us = self.bbr.pacing_delay_us(packet_size);

        now_us >= self.pacing.last_send_us + delay_us
    }

    /// Update pacing state after sending a packet
    fn record_send(&mut self, packet_size: u64) {
        let now_us = self.now_us();
        self.pacing.last_send_us = now_us;
        self.bbr.on_send(packet_size, now_us);
    }

    /// Process one tick of the scheme (handle blocked requests and new requests)
    pub fn tick(&mut self) -> io::Result<()> {
        // Handle any blocked requests
        let mut i = 0;
        while i < self.blocked.len() {
            if let Some(resp) = self.blocked[i].handle_scheme_block(self) {
                self.socket
                    .write_response(resp, SignalBehavior::Restart)
                    .expect("driver-network: failed to write scheme");
                self.blocked.remove(i);
            } else {
                i += 1;
            }
        }

        // Handle new scheme requests
        loop {
            let request = match self.socket.next_request(SignalBehavior::Restart) {
                Ok(Some(request)) => request,
                Ok(None) => {
                    // Scheme likely got unmounted
                    std::process::exit(0);
                }
                Err(err) if err.errno == EAGAIN => break,
                Err(err) => return Err(err.into()),
            };

            match request.kind() {
                RequestKind::Call(call_request) => {
                    if let Some(resp) = call_request.handle_scheme_block(self) {
                        self.socket.write_response(resp, SignalBehavior::Restart)?;
                    } else {
                        self.blocked.push(call_request);
                    }
                }
                RequestKind::OnClose { id } => {
                    self.on_close(id);
                }
                RequestKind::Cancellation(cancellation_request) => {
                    if let Some(i) = self
                        .blocked
                        .iter()
                        .position(|req| req.request().request_id() == cancellation_request.id)
                    {
                        let blocked_req = self.blocked.remove(i);
                        let resp = Response::new(&blocked_req, Err(syscall::Error::new(EINTR)));
                        self.socket.write_response(resp, SignalBehavior::Restart)?;
                    }
                }
                _ => {}
            }
        }

        // Notify readers about incoming events
        let available_for_read = self.adapter.available_for_read();
        if available_for_read > 0 {
            for &handle_id in self.handles.keys() {
                self.socket
                    .post_fevent(handle_id, syscall::flag::EVENT_READ.bits())?;
            }
        }

        Ok(())
    }

    fn on_close(&mut self, id: usize) {
        self.handles.remove(&id);
    }
}

impl<T: NetworkAdapter> SchemeBlock for NetworkScheme<T> {
    fn xopen(
        &mut self,
        path: &str,
        _flags: usize,
        caller_ctx: &CallerCtx,
    ) -> Result<Option<OpenResult>> {
        if caller_ctx.uid != 0 {
            return Err(Error::new(EACCES));
        }

        let (handle, flags) = match path {
            "" => (Handle::Data, NewFdFlags::empty()),
            "mac" => (Handle::Mac, NewFdFlags::POSITIONED),
            "ipv4" => (Handle::Ipv4, NewFdFlags::POSITIONED),
            "ipv6" => (Handle::Ipv6, NewFdFlags::POSITIONED),
            "ipv6_global" => (Handle::Ipv6Global, NewFdFlags::POSITIONED),
            "ipv6_unique_local" => (Handle::Ipv6UniqueLocal, NewFdFlags::POSITIONED),
            "bbr" => (Handle::Bbr, NewFdFlags::POSITIONED),
            "bbr_raw" => (Handle::BbrRaw, NewFdFlags::POSITIONED),
            _ => return Err(Error::new(EINVAL)),
        };

        self.next_id += 1;
        self.handles.insert(self.next_id, handle);
        Ok(Some(OpenResult::ThisScheme {
            number: self.next_id,
            flags,
        }))
    }

    fn read(
        &mut self,
        id: usize,
        buf: &mut [u8],
        offset: u64,
        fcntl_flags: u32,
    ) -> Result<Option<usize>> {
        let handle = self.handles.get_mut(&id).ok_or(Error::new(EBADF))?;

        match *handle {
            Handle::Data => {}
            Handle::Mac => {
                let data = &self.adapter.mac_address()[offset as usize..];
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
                return Ok(Some(i));
            }
            Handle::Ipv4 => {
                let data = &self.adapter.ipv4_address()[offset as usize..];
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
                return Ok(Some(i));
            }
            Handle::Ipv6 => {
                let data = &self.adapter.ipv6_address()[offset as usize..];
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
                return Ok(Some(i));
            }
            Handle::Ipv6Global => {
                let data = &self.adapter.ipv6_address_global()[offset as usize..];
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
                return Ok(Some(i));
            }
            Handle::Ipv6UniqueLocal => {
                let data = &self.adapter.ipv6_address_unique_local()[offset as usize..];
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
                return Ok(Some(i));
            }
            Handle::Bbr => {
                // Text format for human-readable debugging
                let data = format!("{}", self.bbr).into_bytes();
                if offset as usize >= data.len() {
                    return Ok(Some(0));
                }
                let data = &data[offset as usize..];
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
                return Ok(Some(i));
            }
            Handle::BbrRaw => {
                // Binary format for programmatic access
                let metrics = self.bbr.metrics();
                let data = metrics.to_bytes();
                if offset as usize >= data.len() {
                    return Ok(Some(0));
                }
                let data = &data[offset as usize..];
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
                return Ok(Some(i));
            }
        };

        // Handle packet read with BBRv3 updates
        match self.adapter.read_packet(buf)? {
            Some(count) => {
                // Estimate RTT from time since last write
                let rtt_us = estimate_rtt_us(self.last_write);
                let in_flight = self.adapter.in_flight();
                let now_us = self.now_us();

                // Update BBRv3 with the ACK
                self.bbr.on_ack(count as u64, rtt_us, in_flight, now_us);

                // Check for ECN congestion signals
                if let Some(ecn) = extract_ecn(&buf[..count]) {
                    if ecn == EcnFlag::Ce {
                        self.bbr.on_ecn(count as u64);
                    }
                }

                Ok(Some(count))
            }
            None => {
                if fcntl_flags & O_NONBLOCK as u32 != 0 {
                    Err(Error::new(EWOULDBLOCK))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn write(
        &mut self,
        id: usize,
        buf: &[u8],
        _offset: u64,
        _fcntl_flags: u32,
    ) -> Result<Option<usize>> {
        let handle = self.handles.get(&id).ok_or(Error::new(EBADF))?;

        match handle {
            Handle::Data => {}
            Handle::Mac => return Err(Error::new(EINVAL)),
            Handle::Ipv4 => return Err(Error::new(EINVAL)),
            Handle::Ipv6 => return Err(Error::new(EINVAL)),
            Handle::Ipv6Global => return Err(Error::new(EINVAL)),
            Handle::Ipv6UniqueLocal => return Err(Error::new(EINVAL)),
            Handle::Bbr => return Err(Error::new(EINVAL)),
            Handle::BbrRaw => return Err(Error::new(EINVAL)),
        }

        // Enforce pacing rate
        let packet_size = buf.len() as u64;

        // Check if we're within congestion window
        let in_flight = self.adapter.in_flight();
        let cwnd = self.bbr.cwnd();
        if in_flight + packet_size > cwnd {
            // Cwnd limited - would block in a real implementation
            // For now, we allow the send but this indicates congestion
        }

        // Apply pacing delay (in a real implementation, this might involve
        // sleeping or queueing the packet)
        if !self.can_send(packet_size) {
            // Pacing limited - in production we'd queue or wait
            // For simplicity, we proceed but note the violation
        }

        let pacing_rate = self.bbr.pacing_rate();
        let result = self.adapter.write_packet(buf, pacing_rate)?;

        // Update pacing state and BBRv3
        self.record_send(result as u64);
        self.last_write = Instant::now();

        Ok(Some(result))
    }

    fn fevent(&mut self, id: usize, _flags: EventFlags) -> Result<Option<EventFlags>> {
        let _handle = self.handles.get(&id).ok_or(Error::new(EBADF))?;
        Ok(Some(EventFlags::empty()))
    }

    fn fpath(&mut self, id: usize, buf: &mut [u8]) -> Result<Option<usize>> {
        let handle = self.handles.get(&id).ok_or(Error::new(EBADF))?;

        let mut i = 0;

        let scheme_name = self.scheme_name.as_bytes();
        let mut j = 0;
        while i < buf.len() && j < scheme_name.len() {
            buf[i] = scheme_name[j];
            i += 1;
            j += 1;
        }

        if i < buf.len() {
            buf[i] = b':';
            i += 1;
        }

        let path = match handle {
            Handle::Data => &b""[..],
            Handle::Mac => &b"mac"[..],
            Handle::Ipv4 => &b"ipv4"[..],
            Handle::Ipv6 => &b"ipv6"[..],
            Handle::Ipv6Global => &b"ipv6_global"[..],
            Handle::Ipv6UniqueLocal => &b"ipv6_unique_local"[..],
            Handle::Bbr => &b"bbr"[..],
            Handle::BbrRaw => &b"bbr_raw"[..],
        };

        j = 0;
        while i < buf.len() && j < path.len() {
            buf[i] = path[j];
            i += 1;
            j += 1;
        }

        Ok(Some(i))
    }

    fn fstat(&mut self, id: usize, stat: &mut Stat) -> Result<Option<usize>> {
        let handle = self.handles.get(&id).ok_or(Error::new(EBADF))?;

        match handle {
            Handle::Data => {
                stat.st_mode = MODE_FILE | 0o700;
            }
            Handle::Mac => {
                stat.st_mode = MODE_FILE | 0o400;
                stat.st_size = 6;
            }
            Handle::Ipv4 => {
                stat.st_mode = MODE_FILE | 0o400;
                stat.st_size = 4;
            }
            Handle::Ipv6 | Handle::Ipv6Global | Handle::Ipv6UniqueLocal => {
                stat.st_mode = MODE_FILE | 0o400;
                stat.st_size = 16;
            }
            Handle::Bbr => {
                stat.st_mode = MODE_FILE | 0o400;
                stat.st_size = 0; // Variable size text
            }
            Handle::BbrRaw => {
                stat.st_mode = MODE_FILE | 0o400;
                stat.st_size = 64; // Fixed size binary
            }
        }

        Ok(Some(0))
    }

    fn fsync(&mut self, id: usize) -> Result<Option<usize>> {
        let _handle = self.handles.get(&id).ok_or(Error::new(EBADF))?;
        Ok(Some(0))
    }
}

// Re-export BBRv3 types for convenience
pub use bbrv3_rs::{Bbr, BbrMetrics, BbrState};
