//! BBRv3 Congestion Control Algorithm Implementation
//!
//! This crate implements the BBRv3 (Bottleneck Bandwidth and Round-trip propagation time v3)
//! congestion control algorithm, designed for low-latency, high-throughput TCP connections.
//!
//! BBRv3 is a model-based algorithm that:
//! - Estimates the bottleneck bandwidth (BtlBw) and minimum RTT (RTprop)
//! - Operates at Kleinrock's optimal operating point
//! - Incorporates loss and ECN signals for fairness with loss-based CCAs
//!
//! # State Machine
//!
//! The algorithm cycles through four states:
//! - **Startup**: Rapidly discover available bandwidth with exponential growth
//! - **Drain**: Clear any queue buildup from Startup
//! - **ProbeBw**: Steady-state bandwidth probing with 8-phase cycle
//! - **ProbeRtt**: Periodically measure true minimum RTT

#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use core::cmp::{max, min};
use core::fmt;
use core::time::Duration;

// =============================================================================
// BBRv3 Constants (updated from BBRv2)
// =============================================================================

/// Startup pacing gain (reduced from 2.89 in BBRv1/v2)
const BBR_STARTUP_PACING_GAIN: f64 = 2.77;

/// Startup cwnd gain (reduced from 2.89 in BBRv1/v2)
const BBR_STARTUP_CWND_GAIN: f64 = 2.0;

/// Drain pacing gain (more aggressive drainage in v3)
const BBR_DRAIN_PACING_GAIN: f64 = 0.35;

/// ProbeBw gains: UP, DOWN, CRUISE phases
/// v3 uses a simplified 8-phase cycle for bandwidth probing
const BBR_PROBE_BW_GAINS: [f64; 8] = [
    1.25, // UP: probe for more bandwidth
    0.75, // DOWN: drain any queue
    1.0,  // CRUISE
    1.0,  // CRUISE
    1.0,  // CRUISE
    1.0,  // CRUISE
    1.0,  // CRUISE
    1.0,  // CRUISE
];

/// ProbeRtt pacing gain - minimal to allow queue drainage
const BBR_PROBE_RTT_PACING_GAIN: f64 = 1.0;

/// ProbeRtt cwnd gain - target 4 packets inflight
const BBR_PROBE_RTT_CWND_GAIN: f64 = 0.5;

/// Default cwnd gain during normal operation
const BBR_DEFAULT_CWND_GAIN: f64 = 2.0;

/// Maximum loss rate threshold (2% as per BBRv3 spec)
const BBR_LOSS_THRESH: f64 = 0.02;

/// ECN threshold - same as loss threshold
const BBR_ECN_THRESH: f64 = 0.02;

/// Minimum RTT probe interval (5 seconds for v3, was 10 seconds in v1/v2)
const BBR_PROBE_RTT_INTERVAL_US: u64 = 5_000_000;

/// Duration of ProbeRtt state (200ms)
const BBR_PROBE_RTT_DURATION_US: u64 = 200_000;

/// Number of rounds to wait for bandwidth growth in Startup
const BBR_STARTUP_FULL_BW_ROUNDS: u32 = 3;

/// Minimum bandwidth growth rate threshold (25%)
const BBR_STARTUP_FULL_BW_THRESH: f64 = 1.25;

/// Bandwidth filter window size (10 RTTs)
const BBR_BW_FILTER_LEN: usize = 10;

/// RTT filter window size (use minimum over this many samples)
const BBR_RTT_FILTER_LEN: usize = 10;

/// Minimum cwnd in packets
const BBR_MIN_CWND_PACKETS: u64 = 4;

/// Default MSS (Maximum Segment Size)
const BBR_DEFAULT_MSS: u64 = 1460;

// =============================================================================
// BBRv3 State Machine States
// =============================================================================

/// BBRv3 state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrState {
    /// Initial state: exponentially grow to find available bandwidth
    Startup,
    /// Drain any queue that built up during Startup
    Drain,
    /// Steady state: probe for bandwidth with 8-phase cycle
    ProbeBw,
    /// Periodically probe for minimum RTT
    ProbeRtt,
}

impl fmt::Display for BbrState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BbrState::Startup => write!(f, "STARTUP"),
            BbrState::Drain => write!(f, "DRAIN"),
            BbrState::ProbeBw => write!(f, "PROBE_BW"),
            BbrState::ProbeRtt => write!(f, "PROBE_RTT"),
        }
    }
}

// =============================================================================
// Windowed Filter for Bandwidth/RTT Estimation
// =============================================================================

/// A windowed min/max filter for RTT and bandwidth estimation
#[derive(Clone)]
struct WindowedFilter<T: Copy + Ord> {
    samples: VecDeque<(u64, T)>, // (timestamp_us, value)
    window_us: u64,
    is_max: bool,
}

impl<T: Copy + Ord + Default> WindowedFilter<T> {
    fn new(window_us: u64, is_max: bool) -> Self {
        Self {
            samples: VecDeque::with_capacity(16),
            window_us,
            is_max,
        }
    }

    fn update(&mut self, timestamp_us: u64, value: T) {
        // Remove expired samples
        let cutoff = timestamp_us.saturating_sub(self.window_us);
        while let Some(&(ts, _)) = self.samples.front() {
            if ts < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }

        // For max filter: remove samples smaller than new value
        // For min filter: remove samples larger than new value
        while let Some(&(_, v)) = self.samples.back() {
            let should_remove = if self.is_max { v <= value } else { v >= value };
            if should_remove {
                self.samples.pop_back();
            } else {
                break;
            }
        }

        self.samples.push_back((timestamp_us, value));
    }

    fn get(&self) -> Option<T> {
        self.samples.front().map(|&(_, v)| v)
    }

    fn reset(&mut self) {
        self.samples.clear();
    }
}

// =============================================================================
// BBRv3 Metrics for Monitoring
// =============================================================================

/// Metrics exposed for real-time monitoring
#[derive(Debug, Clone, Copy, Default)]
pub struct BbrMetrics {
    /// Current state of the algorithm
    pub state: u8, // 0=Startup, 1=Drain, 2=ProbeBw, 3=ProbeRtt
    /// Estimated bottleneck bandwidth (bytes/sec)
    pub btl_bw: u64,
    /// Minimum RTT (microseconds)
    pub min_rtt_us: u64,
    /// Current pacing rate (bytes/sec)
    pub pacing_rate: u64,
    /// Current congestion window (bytes)
    pub cwnd: u64,
    /// Bytes currently in flight
    pub inflight: u64,
    /// Total bytes delivered
    pub delivered: u64,
    /// Current loss rate (0-100, scaled by 100)
    pub loss_rate_pct: u8,
    /// Current ECN mark rate (0-100, scaled by 100)
    pub ecn_rate_pct: u8,
    /// ProbeBw cycle index (0-7)
    pub probe_bw_cycle: u8,
    /// Rounds since last bandwidth increase
    pub full_bw_cnt: u8,
    /// Whether full bandwidth has been reached
    pub full_bw_reached: bool,
}

impl BbrMetrics {
    /// Serialize metrics to a fixed-size byte buffer for scheme interface
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        buf[0] = self.state;
        buf[1..9].copy_from_slice(&self.btl_bw.to_le_bytes());
        buf[9..17].copy_from_slice(&self.min_rtt_us.to_le_bytes());
        buf[17..25].copy_from_slice(&self.pacing_rate.to_le_bytes());
        buf[25..33].copy_from_slice(&self.cwnd.to_le_bytes());
        buf[33..41].copy_from_slice(&self.inflight.to_le_bytes());
        buf[41..49].copy_from_slice(&self.delivered.to_le_bytes());
        buf[49] = self.loss_rate_pct;
        buf[50] = self.ecn_rate_pct;
        buf[51] = self.probe_bw_cycle;
        buf[52] = self.full_bw_cnt;
        buf[53] = if self.full_bw_reached { 1 } else { 0 };
        buf
    }

    /// Deserialize metrics from byte buffer
    pub fn from_bytes(buf: &[u8; 64]) -> Self {
        Self {
            state: buf[0],
            btl_bw: u64::from_le_bytes(buf[1..9].try_into().unwrap()),
            min_rtt_us: u64::from_le_bytes(buf[9..17].try_into().unwrap()),
            pacing_rate: u64::from_le_bytes(buf[17..25].try_into().unwrap()),
            cwnd: u64::from_le_bytes(buf[25..33].try_into().unwrap()),
            inflight: u64::from_le_bytes(buf[33..41].try_into().unwrap()),
            delivered: u64::from_le_bytes(buf[41..49].try_into().unwrap()),
            loss_rate_pct: buf[49],
            ecn_rate_pct: buf[50],
            probe_bw_cycle: buf[51],
            full_bw_cnt: buf[52],
            full_bw_reached: buf[53] != 0,
        }
    }
}

// =============================================================================
// BBRv3 Core Implementation
// =============================================================================

/// BBRv3 Congestion Control Algorithm
pub struct Bbr {
    // -------------------------------------------------------------------------
    // Core State
    // -------------------------------------------------------------------------
    state: BbrState,
    mss: u64,

    // -------------------------------------------------------------------------
    // Path Model
    // -------------------------------------------------------------------------
    /// Maximum bandwidth filter (windowed max)
    bw_filter: WindowedFilter<u64>,
    /// Current bandwidth estimate (bytes/sec)
    btl_bw: u64,

    /// Minimum RTT filter (windowed min)
    rtt_filter: WindowedFilter<u64>,
    /// Minimum RTT estimate (microseconds)
    min_rtt_us: u64,
    /// Timestamp when min_rtt was last updated
    min_rtt_stamp_us: u64,

    // -------------------------------------------------------------------------
    // Pacing/Cwnd
    // -------------------------------------------------------------------------
    pacing_gain: f64,
    cwnd_gain: f64,
    pacing_rate: u64,
    cwnd: u64,

    // -------------------------------------------------------------------------
    // ProbeBw State
    // -------------------------------------------------------------------------
    probe_bw_cycle_idx: usize,
    probe_bw_cycle_stamp_us: u64,

    // -------------------------------------------------------------------------
    // Startup State
    // -------------------------------------------------------------------------
    full_bw_reached: bool,
    full_bw_cnt: u32,
    full_bw: u64,

    // -------------------------------------------------------------------------
    // ProbeRtt State
    // -------------------------------------------------------------------------
    probe_rtt_done_stamp_us: Option<u64>,
    probe_rtt_round_done: bool,
    prior_cwnd: u64,
    idle_restart: bool,

    // -------------------------------------------------------------------------
    // Round Counting
    // -------------------------------------------------------------------------
    round_start: bool,
    round_count: u64,
    next_round_delivered: u64,

    // -------------------------------------------------------------------------
    // Delivery Tracking
    // -------------------------------------------------------------------------
    delivered: u64,
    delivered_time_us: u64,

    // -------------------------------------------------------------------------
    // Loss/ECN Tracking
    // -------------------------------------------------------------------------
    lost: u64,
    ecn_marked: u64,
    prior_delivered: u64,
    prior_lost: u64,
    prior_ecn: u64,

    // -------------------------------------------------------------------------
    // Inflight Tracking
    // -------------------------------------------------------------------------
    inflight: u64,

    // -------------------------------------------------------------------------
    // Timestamp (must be provided externally)
    // -------------------------------------------------------------------------
    now_us: u64,
}

impl Bbr {
    /// Creates a new BBRv3 instance with default MSS
    pub fn new() -> Self {
        Self::with_mss(BBR_DEFAULT_MSS)
    }

    /// Creates a new BBRv3 instance with custom MSS
    pub fn with_mss(mss: u64) -> Self {
        let bw_window_us = 10 * 1_000_000; // 10 seconds for BW filter
        let rtt_window_us = BBR_PROBE_RTT_INTERVAL_US; // Same as probe interval

        Self {
            state: BbrState::Startup,
            mss,

            bw_filter: WindowedFilter::new(bw_window_us, true),
            btl_bw: 0,
            rtt_filter: WindowedFilter::new(rtt_window_us, false),
            min_rtt_us: u64::MAX,
            min_rtt_stamp_us: 0,

            pacing_gain: BBR_STARTUP_PACING_GAIN,
            cwnd_gain: BBR_STARTUP_CWND_GAIN,
            pacing_rate: 0,
            cwnd: BBR_MIN_CWND_PACKETS * mss,

            probe_bw_cycle_idx: 0,
            probe_bw_cycle_stamp_us: 0,

            full_bw_reached: false,
            full_bw_cnt: 0,
            full_bw: 0,

            probe_rtt_done_stamp_us: None,
            probe_rtt_round_done: false,
            prior_cwnd: 0,
            idle_restart: false,

            round_start: false,
            round_count: 0,
            next_round_delivered: 0,

            delivered: 0,
            delivered_time_us: 0,

            lost: 0,
            ecn_marked: 0,
            prior_delivered: 0,
            prior_lost: 0,
            prior_ecn: 0,

            inflight: 0,
            now_us: 0,
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /// Returns the current state of the algorithm
    pub fn state(&self) -> BbrState {
        self.state
    }

    /// Returns the current pacing rate in bytes per second
    pub fn pacing_rate(&self) -> u64 {
        self.pacing_rate
    }

    /// Returns the current congestion window in bytes
    pub fn cwnd(&self) -> u64 {
        self.cwnd
    }

    /// Returns the estimated bottleneck bandwidth in bytes per second
    pub fn btl_bw(&self) -> u64 {
        self.btl_bw
    }

    /// Returns the minimum RTT in microseconds
    pub fn min_rtt_us(&self) -> u64 {
        if self.min_rtt_us == u64::MAX {
            0
        } else {
            self.min_rtt_us
        }
    }

    /// Returns the minimum RTT as a Duration
    pub fn min_rtt(&self) -> Duration {
        if self.min_rtt_us == u64::MAX {
            Duration::ZERO
        } else {
            Duration::from_micros(self.min_rtt_us)
        }
    }

    /// Gets the current metrics for monitoring
    pub fn metrics(&self) -> BbrMetrics {
        BbrMetrics {
            state: self.state as u8,
            btl_bw: self.btl_bw,
            min_rtt_us: self.min_rtt_us(),
            pacing_rate: self.pacing_rate,
            cwnd: self.cwnd,
            inflight: self.inflight,
            delivered: self.delivered,
            loss_rate_pct: self.loss_rate_pct(),
            ecn_rate_pct: self.ecn_rate_pct(),
            probe_bw_cycle: self.probe_bw_cycle_idx as u8,
            full_bw_cnt: min(self.full_bw_cnt, 255) as u8,
            full_bw_reached: self.full_bw_reached,
        }
    }

    /// Calculate the inter-packet gap for pacing (in microseconds)
    pub fn pacing_delay_us(&self, packet_size: u64) -> u64 {
        if self.pacing_rate == 0 {
            return 0;
        }
        // delay_us = (packet_size * 1_000_000) / pacing_rate
        (packet_size as u128 * 1_000_000 / self.pacing_rate as u128) as u64
    }

    /// Called when data is sent
    pub fn on_send(&mut self, bytes_sent: u64, now_us: u64) {
        self.now_us = now_us;
        self.inflight = self.inflight.saturating_add(bytes_sent);
    }

    /// Main ACK handler - called when an ACK is received
    ///
    /// # Arguments
    /// * `bytes_acked` - Number of bytes acknowledged
    /// * `rtt_us` - Round-trip time for this ACK in microseconds
    /// * `inflight` - Current bytes in flight after this ACK
    /// * `now_us` - Current timestamp in microseconds
    pub fn on_ack(&mut self, bytes_acked: u64, rtt_us: u64, inflight: u64, now_us: u64) {
        self.now_us = now_us;
        self.inflight = inflight;
        self.delivered = self.delivered.saturating_add(bytes_acked);
        self.delivered_time_us = now_us;

        // Update round counting
        self.update_round();

        // Update path model
        self.update_model(rtt_us, bytes_acked);

        // Check for ProbeRtt (always check regardless of current state)
        self.check_probe_rtt();

        if self.state == BbrState::ProbeRtt {
            self.handle_probe_rtt();
        } else {
            // Run state machine
            match self.state {
                BbrState::Startup => self.check_startup_done(),
                BbrState::Drain => self.check_drain(),
                BbrState::ProbeBw => self.update_probe_bw_cycle(),
                BbrState::ProbeRtt => {} // Handled above
            }
        }

        // Update pacing rate and cwnd
        self.set_pacing_rate();
        self.set_cwnd();
    }

    /// Called on packet loss
    pub fn on_loss(&mut self, bytes_lost: u64) {
        self.lost = self.lost.saturating_add(bytes_lost);
        self.inflight = self.inflight.saturating_sub(bytes_lost);

        // Check if loss rate exceeds threshold
        let loss_rate = self.loss_rate();
        if loss_rate > BBR_LOSS_THRESH {
            self.handle_loss_exceeded();
        }
    }

    /// Called on ECN congestion signal
    pub fn on_ecn(&mut self, bytes_ecn_marked: u64) {
        self.ecn_marked = self.ecn_marked.saturating_add(bytes_ecn_marked);

        // Check if ECN rate exceeds threshold
        let ecn_rate = self.ecn_rate();
        if ecn_rate > BBR_ECN_THRESH {
            self.handle_ecn_exceeded();
        }
    }

    /// Combined congestion event handler (loss or ECN)
    pub fn on_congestion_event(&mut self) {
        // Reduce bandwidth estimate by 20% on congestion
        self.btl_bw = (self.btl_bw as f64 * 0.8) as u64;
        self.set_pacing_rate();
        self.set_cwnd();
    }

    // =========================================================================
    // Path Model Updates
    // =========================================================================

    fn update_round(&mut self) {
        if self.delivered >= self.next_round_delivered {
            self.next_round_delivered = self.delivered;
            self.round_count += 1;
            self.round_start = true;
        } else {
            self.round_start = false;
        }
    }

    fn update_model(&mut self, rtt_us: u64, bytes_acked: u64) {
        // Update RTT estimate
        if rtt_us > 0 {
            self.rtt_filter.update(self.now_us, rtt_us);
            if let Some(min_rtt) = self.rtt_filter.get() {
                if min_rtt < self.min_rtt_us || self.min_rtt_us == u64::MAX {
                    self.min_rtt_us = min_rtt;
                    self.min_rtt_stamp_us = self.now_us;
                }
            }
        }

        // Update bandwidth estimate
        // BW = bytes_acked / rtt_us * 1_000_000 (to get bytes/sec)
        if rtt_us > 0 && bytes_acked > 0 {
            let bw = (bytes_acked as u128 * 1_000_000 / rtt_us as u128) as u64;
            self.bw_filter.update(self.now_us, bw);
            if let Some(max_bw) = self.bw_filter.get() {
                self.btl_bw = max_bw;
            }
        }
    }

    // =========================================================================
    // State Machine Transitions
    // =========================================================================

    fn enter_startup(&mut self) {
        self.state = BbrState::Startup;
        self.pacing_gain = BBR_STARTUP_PACING_GAIN;
        self.cwnd_gain = BBR_STARTUP_CWND_GAIN;
    }

    fn check_startup_done(&mut self) {
        self.check_startup_full_bw();
        self.check_startup_high_loss();

        if self.full_bw_reached {
            self.enter_drain();
        }
    }

    fn check_startup_full_bw(&mut self) {
        if self.full_bw_reached || !self.round_start {
            return;
        }

        // Check if bandwidth has grown by at least 25%
        let growth_target = (self.full_bw as f64 * BBR_STARTUP_FULL_BW_THRESH) as u64;
        if self.btl_bw >= growth_target {
            self.full_bw = self.btl_bw;
            self.full_bw_cnt = 0;
            return;
        }

        // Bandwidth hasn't grown, increment counter
        self.full_bw_cnt += 1;
        if self.full_bw_cnt >= BBR_STARTUP_FULL_BW_ROUNDS {
            self.full_bw_reached = true;
        }
    }

    fn check_startup_high_loss(&mut self) {
        // Exit startup if loss rate exceeds 2%
        if self.loss_rate() > BBR_LOSS_THRESH {
            self.full_bw_reached = true;
        }
    }

    fn enter_drain(&mut self) {
        self.state = BbrState::Drain;
        self.pacing_gain = BBR_DRAIN_PACING_GAIN;
        self.cwnd_gain = BBR_STARTUP_CWND_GAIN; // Keep high cwnd initially
    }

    fn check_drain(&mut self) {
        // Exit drain when inflight drops to BDP
        let bdp = self.bdp();
        if self.inflight <= bdp {
            self.enter_probe_bw();
        }
    }

    fn enter_probe_bw(&mut self) {
        self.state = BbrState::ProbeBw;
        self.probe_bw_cycle_idx = 0;
        self.probe_bw_cycle_stamp_us = self.now_us;
        self.advance_probe_bw_cycle_phase();
    }

    fn update_probe_bw_cycle(&mut self) {
        if !self.round_start {
            return;
        }

        // Advance to next phase on each round
        self.probe_bw_cycle_idx = (self.probe_bw_cycle_idx + 1) % BBR_PROBE_BW_GAINS.len();
        self.advance_probe_bw_cycle_phase();
    }

    fn advance_probe_bw_cycle_phase(&mut self) {
        self.pacing_gain = BBR_PROBE_BW_GAINS[self.probe_bw_cycle_idx];
        self.cwnd_gain = BBR_DEFAULT_CWND_GAIN;
    }

    fn check_probe_rtt(&mut self) {
        // Enter ProbeRtt if we haven't probed RTT recently
        if self.state != BbrState::ProbeRtt {
            let time_since_probe = self.now_us.saturating_sub(self.min_rtt_stamp_us);
            if time_since_probe > BBR_PROBE_RTT_INTERVAL_US {
                self.enter_probe_rtt();
            }
        }
    }

    fn enter_probe_rtt(&mut self) {
        self.state = BbrState::ProbeRtt;
        self.pacing_gain = BBR_PROBE_RTT_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_RTT_CWND_GAIN;
        self.prior_cwnd = self.cwnd;
        self.probe_rtt_done_stamp_us = None;
        self.probe_rtt_round_done = false;
    }

    fn handle_probe_rtt(&mut self) {
        // Start the probe RTT timer once inflight is low enough
        if self.probe_rtt_done_stamp_us.is_none() {
            let target_inflight = BBR_MIN_CWND_PACKETS * self.mss;
            if self.inflight <= target_inflight {
                self.probe_rtt_done_stamp_us = Some(self.now_us + BBR_PROBE_RTT_DURATION_US);
                self.probe_rtt_round_done = false;
            }
        }

        // Check if probe RTT is complete
        if let Some(done_stamp) = self.probe_rtt_done_stamp_us {
            if self.round_start {
                self.probe_rtt_round_done = true;
            }
            if self.now_us >= done_stamp && self.probe_rtt_round_done {
                self.min_rtt_stamp_us = self.now_us;
                self.restore_cwnd();
                self.exit_probe_rtt();
            }
        }
    }

    fn exit_probe_rtt(&mut self) {
        if self.full_bw_reached {
            self.enter_probe_bw();
        } else {
            self.enter_startup();
        }
    }

    fn restore_cwnd(&mut self) {
        self.cwnd = max(self.cwnd, self.prior_cwnd);
    }

    // =========================================================================
    // Loss/ECN Handling
    // =========================================================================

    fn loss_rate(&self) -> f64 {
        let interval_delivered = self.delivered.saturating_sub(self.prior_delivered);
        let interval_lost = self.lost.saturating_sub(self.prior_lost);

        if interval_delivered == 0 {
            return 0.0;
        }

        interval_lost as f64 / (interval_delivered + interval_lost) as f64
    }

    fn loss_rate_pct(&self) -> u8 {
        min(100, (self.loss_rate() * 100.0) as u8)
    }

    fn ecn_rate(&self) -> f64 {
        let interval_delivered = self.delivered.saturating_sub(self.prior_delivered);
        let interval_ecn = self.ecn_marked.saturating_sub(self.prior_ecn);

        if interval_delivered == 0 {
            return 0.0;
        }

        interval_ecn as f64 / interval_delivered as f64
    }

    fn ecn_rate_pct(&self) -> u8 {
        min(100, (self.ecn_rate() * 100.0) as u8)
    }

    fn handle_loss_exceeded(&mut self) {
        // Record prior values for next interval
        self.prior_delivered = self.delivered;
        self.prior_lost = self.lost;

        // Reduce bandwidth estimate
        self.btl_bw = (self.btl_bw as f64 * 0.7) as u64;
    }

    fn handle_ecn_exceeded(&mut self) {
        self.prior_delivered = self.delivered;
        self.prior_ecn = self.ecn_marked;

        // More gentle reduction for ECN
        self.btl_bw = (self.btl_bw as f64 * 0.85) as u64;
    }

    // =========================================================================
    // Pacing Rate & Cwnd Calculation
    // =========================================================================

    fn set_pacing_rate(&mut self) {
        let rate = (self.btl_bw as f64 * self.pacing_gain) as u64;
        // Smooth pacing rate changes
        if self.state == BbrState::Startup && rate > self.pacing_rate {
            self.pacing_rate = rate;
        } else {
            // Apply some smoothing in other states
            self.pacing_rate = (self.pacing_rate as f64 * 0.75 + rate as f64 * 0.25) as u64;
        }
    }

    fn set_cwnd(&mut self) {
        let bdp = self.bdp();
        let target = (bdp as f64 * self.cwnd_gain) as u64;
        let min_cwnd = BBR_MIN_CWND_PACKETS * self.mss;

        self.cwnd = max(min_cwnd, target);

        // Don't allow cwnd to shrink too fast in ProbeRtt
        if self.state != BbrState::ProbeRtt {
            // Allow cwnd to grow freely but limit shrinkage
            if target < self.cwnd {
                self.cwnd = max(target, (self.cwnd as f64 * 0.9) as u64);
            }
        } else {
            // In ProbeRtt, reduce to minimum
            self.cwnd = max(min_cwnd, bdp / 2);
        }
    }

    fn bdp(&self) -> u64 {
        if self.min_rtt_us == u64::MAX || self.min_rtt_us == 0 {
            return BBR_MIN_CWND_PACKETS * self.mss;
        }

        // BDP = btl_bw (bytes/sec) * min_rtt (sec)
        // = btl_bw * min_rtt_us / 1_000_000
        let bdp = (self.btl_bw as u128 * self.min_rtt_us as u128 / 1_000_000) as u64;
        max(bdp, BBR_MIN_CWND_PACKETS * self.mss)
    }
}

impl Default for Bbr {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Bbr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BBRv3")
            .field("state", &self.state)
            .field("btl_bw", &self.btl_bw)
            .field("min_rtt_us", &self.min_rtt_us())
            .field("pacing_rate", &self.pacing_rate)
            .field("cwnd", &self.cwnd)
            .field("inflight", &self.inflight)
            .field("delivered", &self.delivered)
            .field("round_count", &self.round_count)
            .field("probe_bw_cycle", &self.probe_bw_cycle_idx)
            .field("full_bw_reached", &self.full_bw_reached)
            .finish()
    }
}

impl fmt::Display for Bbr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BBRv3[{}] BW={:.2}Mbps RTT={:.2}ms Pace={:.2}Mbps CWND={}KB",
            self.state,
            self.btl_bw as f64 / 125000.0,      // bytes/s to Mbps
            self.min_rtt_us() as f64 / 1000.0,  // us to ms
            self.pacing_rate as f64 / 125000.0, // bytes/s to Mbps
            self.cwnd / 1024
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_initial_state() {
        let bbr = Bbr::new();
        assert_eq!(bbr.state(), BbrState::Startup);
        assert_eq!(bbr.btl_bw(), 0);
        assert_eq!(bbr.pacing_rate(), 0);
    }

    #[test]
    fn test_startup_to_drain_transition() {
        let mut bbr = Bbr::new();

        // Simulate several rounds with stable bandwidth
        for i in 0..10 {
            bbr.on_ack(10000, 10000, 5000, i * 100000);
        }

        // After bandwidth stabilizes, should transition to Drain
        assert!(
            bbr.full_bw_reached || bbr.state() != BbrState::Startup,
            "Should exit Startup eventually"
        );
    }

    #[test]
    fn test_pacing_delay_calculation() {
        let mut bbr = Bbr::new();

        // Set up a known pacing rate (1 Gbps = 125,000,000 bytes/sec)
        bbr.pacing_rate = 125_000_000;

        // 1500 byte packet should take 12us at 1 Gbps
        let delay = bbr.pacing_delay_us(1500);
        assert_eq!(delay, 12);
    }

    #[test]
    fn test_metrics_serialization() {
        let mut bbr = Bbr::new();
        bbr.on_ack(1000, 10000, 500, 0);

        let metrics = bbr.metrics();
        let bytes = metrics.to_bytes();
        let decoded = BbrMetrics::from_bytes(&bytes);

        assert_eq!(decoded.state, metrics.state);
        assert_eq!(decoded.btl_bw, metrics.btl_bw);
        assert_eq!(decoded.pacing_rate, metrics.pacing_rate);
    }
}
