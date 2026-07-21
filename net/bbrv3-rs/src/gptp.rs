#![forbid(unsafe_code)]

//! # IEEE 802.1AS gPTP Time Synchronization Protocol
//!
//! Sub-microsecond Automotive Ethernet clock synchronization across Electronic Control Units (ECUs).
//! Leverages hardware NIC ingress/egress timestamps $t_1, t_2, t_3, t_4$ to continuously compute
//! propagation delay and clock offset.
//!
//! ## Mathematical & Synchronization Model
//! Given hardware timestamps $t_1$ (Sync egress), $t_2$ (Sync ingress), $t_3$ (Pdelay_Req egress), $t_4$ (Pdelay_Req ingress):
//! $$\text{Delay} = \frac{(t_4 - t_3) + (t_2 - t_1)}{2}$$
//! $$\text{ClockOffset} = (t_2 - t_1) - \text{Delay}$$

use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// IEEE 802.1AS gPTP Clock Synchronizer Engine.
pub struct GPtpClockSynchronizer {
    pub current_clock_offset_ns: AtomicI64,
    pub path_delay_ns: AtomicU64,
    pub total_sync_messages: AtomicU64,
}

impl GPtpClockSynchronizer {
    /// Creates a new `GPtpClockSynchronizer`.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub const fn new() -> Self {
        Self {
            current_clock_offset_ns: AtomicI64::new(0),
            path_delay_ns: AtomicU64::new(0),
            total_sync_messages: AtomicU64::new(0),
        }
    }

    /// Processes hardware NIC timestamps to update ECU clock offset and path delay.
    ///
    /// # Mathematical Model
    /// $$\text{Delay} = \frac{(t_4 - t_3) + (t_2 - t_1)}{2}$$
    /// $$\text{Offset} = (t_2 - t_1) - \text{Delay}$$
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn process_hardware_timestamps(&self, t1_ns: u64, t2_ns: u64, t3_ns: u64, t4_ns: u64) -> (i64, u64) {
        let diff21 = (t2_ns as i64).saturating_sub(t1_ns as i64);
        let diff43 = (t4_ns as i64).saturating_sub(t3_ns as i64);

        let delay = ((diff43.max(0) + diff21.max(0)) / 2) as u64;
        let offset = diff21.saturating_sub(delay as i64);

        self.path_delay_ns.store(delay, Ordering::Release);
        self.current_clock_offset_ns.store(offset, Ordering::Release);
        self.total_sync_messages.fetch_add(1, Ordering::Relaxed);

        (offset, delay)
    }
}

/// Global gPTP clock synchronizer instance.
pub static GPTP_SYNCHRONIZER: GPtpClockSynchronizer = GPtpClockSynchronizer::new();
