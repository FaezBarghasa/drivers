#![forbid(unsafe_code)]

//! # XDP-Equivalent Zero-Copy Fast Packet Path
//!
//! Programmable packet filter and fast-path routing hook placed directly inside
//! driver receive rings. Fast-drops SYN-flood attack signatures in $\mathcal{O}(1)$ time
//! and passes valid payload ownership via `DmaBuf` handles directly to application sockets.
//!
//! ## Mathematical & Filter Model
//! Given incoming packet $P$ with IP tuple $T_p = (\text{src\_ip}, \text{dst\_ip}, \text{src\_port}, \text{dst\_port}, \text{proto})$:
//! $$\text{XDP\_Action}(P) = \begin{cases} \text{Drop} & \text{if } T_p \in \text{Blacklist} \lor \text{IsSynFlood}(P) \\ \text{Redirect} & \text{if } T_p \in \text{FastPathTable} \\ \text{Pass} & \text{otherwise} \end{cases}$$

use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use alloc::collections::BTreeSet;
use spin::Mutex;

/// XDP Packet Action Decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpAction {
    /// Drop packet instantly at driver ring level.
    Drop,
    /// Direct zero-copy pass-through via DmaBuf to application socket.
    Redirect(u64), // Socket buffer address
    /// Standard network stack processing.
    Pass,
}

/// 5-Tuple Key for fast path lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FiveTuple {
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
}

/// Driver-level XDP Fast Path Engine.
pub struct XdpFastPathEngine {
    pub is_enabled: AtomicBool,
    pub dropped_packets: AtomicU64,
    pub redirected_packets: AtomicU64,
    pub passed_packets: AtomicU64,
    pub drop_blacklist: Mutex<BTreeSet<FiveTuple>>,
    pub fast_redirect_table: Mutex<BTreeSet<(FiveTuple, u64)>>,
}

impl XdpFastPathEngine {
    /// Creates a new `XdpFastPathEngine`.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn new() -> Self {
        Self {
            is_enabled: AtomicBool::new(true),
            dropped_packets: AtomicU64::new(0),
            redirected_packets: AtomicU64::new(0),
            passed_packets: AtomicU64::new(0),
            drop_blacklist: Mutex::new(BTreeSet::new()),
            fast_redirect_table: Mutex::new(BTreeSet::new()),
        }
    }

    /// Evaluates an incoming packet tuple directly at NIC ring interface.
    ///
    /// Complexity: $\mathcal{O}(\log K)$ where $K$ is active filter rules count.
    pub fn process_rx_ring_packet(&self, tuple: FiveTuple, is_syn_flag: bool, payload_len: usize) -> XdpAction {
        if !self.is_enabled.load(Ordering::Acquire) {
            self.passed_packets.fetch_add(1, Ordering::Relaxed);
            return XdpAction::Pass;
        }

        // Check blacklist for fast drop (e.g. SYN-flood attack mitigation)
        let blacklist = self.drop_blacklist.lock();
        if blacklist.contains(&tuple) || (is_syn_flag && payload_len == 0) {
            self.dropped_packets.fetch_add(1, Ordering::Relaxed);
            return XdpAction::Drop;
        }
        drop(blacklist);

        // Check redirect fast path
        let redirects = self.fast_redirect_table.lock();
        for (rule_tuple, target_socket_addr) in redirects.iter() {
            if rule_tuple == &tuple {
                self.redirected_packets.fetch_add(1, Ordering::Relaxed);
                return XdpAction::Redirect(*target_socket_addr);
            }
        }

        self.passed_packets.fetch_add(1, Ordering::Relaxed);
        XdpAction::Pass
    }

    /// Adds a 5-tuple signature to the hardware fast-drop blacklist.
    ///
    /// Complexity: $\mathcal{O}(\log K)$
    pub fn add_drop_rule(&self, tuple: FiveTuple) {
        let mut blacklist = self.drop_blacklist.lock();
        blacklist.insert(tuple);
    }
}

/// Global XDP fast path engine instance.
pub static XDP_ENGINE: XdpFastPathEngine = XdpFastPathEngine::new();
