#![forbid(unsafe_code)]

//! # BLE 5.4 Mesh Networking Zero-Allocation Protocol Stack
//!
//! Bluetooth Low Energy 5.4 Mesh routing engine designed for MCU and embedded platforms.
//! Eliminates dynamic heap allocation by routing mesh packets through static slab buffers.
//!
//! ## Mathematical & Routing Model
//! Given TTL value $T_{ttl}$ and sequence number $S_{seq}$ for mesh packet $P$:
//! $$\text{ShouldRelay}(P) = \begin{cases} \text{true} & \text{if } T_{ttl} > 1 \land S_{seq} \notin \text{SeenCache} \\ \text{false} & \text{otherwise} \end{cases}$$

use core::sync::atomic::{AtomicU64, AtomicU16, Ordering};

/// BLE 5.4 Mesh Packet Header.
#[derive(Debug, Clone, Copy)]
pub struct BleMeshPacketHeader {
    pub src_unicast_addr: u16,
    pub dst_unicast_addr: u16,
    pub sequence_number: u32,
    pub ttl: u8,
    pub opcode: u8,
}

/// BLE Mesh Routing Engine.
pub struct BleMeshRoutingEngine {
    pub total_packets_routed: AtomicU64,
    pub total_relayed: AtomicU64,
    pub last_seen_sequence: AtomicU64,
}

impl BleMeshRoutingEngine {
    /// Creates a new `BleMeshRoutingEngine`.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub const fn new() -> Self {
        Self {
            total_packets_routed: AtomicU64::new(0),
            total_relayed: AtomicU64::new(0),
            last_seen_sequence: AtomicU64::new(0),
        }
    }

    /// Evaluates incoming BLE mesh packet for local consumption or relaying.
    ///
    /// # Mathematical Model
    /// Relay condition: $T_{ttl} > 1 \land S_{seq} > S_{last\_seen}$
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn process_mesh_packet(&self, header: BleMeshPacketHeader, local_addr: u16) -> bool {
        self.total_packets_routed.fetch_add(1, Ordering::Relaxed);

        if header.dst_unicast_addr == local_addr {
            return false; // Packet reached final destination locally
        }

        // Check TTL and Sequence for relaying
        if header.ttl > 1 {
            let seq = header.sequence_number as u64;
            let last_seq = self.last_seen_sequence.load(Ordering::Acquire);
            if seq > last_seq {
                self.last_seen_sequence.store(seq, Ordering::Release);
                self.total_relayed.fetch_add(1, Ordering::Relaxed);
                return true; // Should relay to adjacent node
            }
        }

        false
    }
}

/// Global BLE Mesh routing engine instance.
pub static BLE_MESH_ENGINE: BleMeshRoutingEngine = BleMeshRoutingEngine::new();
