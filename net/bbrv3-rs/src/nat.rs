#![forbid(unsafe_code)]

//! # RFC3489 Full-Cone NAT & 5-Tuple FLOWOFFLOAD Engine
//!
//! Maintains bidirectional mappings for full-cone NAT routing and offloads established
//! TCP 5-tuples from firewall packet inspection directly into socket buffers.
//!
//! ## Mathematical Model
//! Given internal endpoint $E_{in} = (\text{IP}_{in}, \text{Port}_{in})$ and external mapping $E_{pub} = (\text{IP}_{pub}, \text{Port}_{pub})$:
//! $$\text{ForwardMap}(E_{in}) = E_{pub}, \quad \text{ReverseMap}(E_{pub}) = E_{in}$$
//!
//! For any external source $E_{ext}$, incoming packet to $E_{pub}$ maps to $E_{in}$ without filtering.

use core::sync::atomic::{AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use spin::Mutex;
use crate::xdp::FiveTuple;

/// Full-Cone NAT Mapping Pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Endpoint {
    pub ip: u32,
    pub port: u16,
}

/// Netfilter & FLOWOFFLOAD Engine.
pub struct FlowOffloadNatEngine {
    pub total_natted_packets: AtomicU64,
    pub total_offloaded_packets: AtomicU64,
    pub forward_nat_table: Mutex<BTreeMap<Endpoint, Endpoint>>,
    pub reverse_nat_table: Mutex<BTreeMap<Endpoint, Endpoint>>,
    pub flow_offload_set: Mutex<BTreeMap<FiveTuple, u64>>, // Tuple -> Socket Handle
}

impl FlowOffloadNatEngine {
    /// Creates a new `FlowOffloadNatEngine`.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn new() -> Self {
        Self {
            total_natted_packets: AtomicU64::new(0),
            total_offloaded_packets: AtomicU64::new(0),
            forward_nat_table: Mutex::new(BTreeMap::new()),
            reverse_nat_table: Mutex::new(BTreeMap::new()),
            flow_offload_set: Mutex::new(BTreeMap::new()),
        }
    }

    /// Registers a Full-Cone NAT mapping.
    ///
    /// Complexity: $\mathcal{O}(\log N)$
    pub fn register_full_cone_mapping(&self, internal: Endpoint, public: Endpoint) {
        let mut fwd = self.forward_nat_table.lock();
        let mut rev = self.reverse_nat_table.lock();
        fwd.insert(internal, public);
        rev.insert(public, internal);
    }

    /// Translates an incoming public endpoint back to internal target.
    ///
    /// Complexity: $\mathcal{O}(\log N)$
    pub fn translate_inbound_public(&self, public: Endpoint) -> Option<Endpoint> {
        let rev = self.reverse_nat_table.lock();
        if let Some(&internal) = rev.get(&public) {
            self.total_natted_packets.fetch_add(1, Ordering::Relaxed);
            Some(internal)
        } else {
            None
        }
    }

    /// Registers an established TCP stream in the FLOWOFFLOAD bypass table.
    ///
    /// Complexity: $\mathcal{O}(\log N)$
    pub fn register_flow_offload(&self, tuple: FiveTuple, socket_handle: u64) {
        let mut flows = self.flow_offload_set.lock();
        flows.insert(tuple, socket_handle);
    }

    /// Checks if an incoming packet matches an offloaded flow.
    ///
    /// Complexity: $\mathcal{O}(\log N)$
    pub fn check_flow_offload(&self, tuple: &FiveTuple) -> Option<u64> {
        let flows = self.flow_offload_set.lock();
        if let Some(&handle) = flows.get(tuple) {
            self.total_offloaded_packets.fetch_add(1, Ordering::Relaxed);
            Some(handle)
        } else {
            None
        }
    }
}

/// Global FLOWOFFLOAD & NAT engine instance.
pub static NAT_FLOW_ENGINE: FlowOffloadNatEngine = FlowOffloadNatEngine::new();
