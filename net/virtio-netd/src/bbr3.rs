//! BBRv3 TCP Congestion Control
//!
//! Implementation of Google's Bottleneck Bandwidth and RTT (BBR) version 3
//! congestion control algorithm for high-throughput, low-latency networking.
//!
//! # BBRv3 Key Features
//!
//! - Model-based congestion control (not loss-based)
//! - Explicit bandwidth and RTT estimation
//! - Pacing for smooth packet delivery
//! - Faster convergence than v1/v2
//! - Better coexistence with loss-based CCs
//!
//! # State Machine
//!
//! STARTUP → DRAIN → PROBE_BW ↔ PROBE_RTT
//!
//! - STARTUP: Exponential growth to find bandwidth
//! - DRAIN: Drain queue created during startup
//! - PROBE_BW: Steady-state bandwidth probing (cruise, refill, up, down)
//! - PROBE_RTT: Periodic RTT measurement

use std::time::{Duration, Instant};

/// BBR version
pub const BBR_VERSION: u32 = 3;

/// Pacing gain during startup (2/ln(2) ≈ 2.89)
pub const STARTUP_PACING_GAIN: f64 = 2.89;

/// Startup CWND gain
pub const STARTUP_CWND_GAIN: f64 = 2.0;

/// Drain pacing gain
pub const DRAIN_PACING_GAIN: f64 = 0.75;

/// Probe RTT duration
pub const PROBE_RTT_DURATION_MS: u64 = 200;

/// Probe RTT interval
pub const PROBE_RTT_INTERVAL_SEC: u64 = 10;

/// Minimum RTT filter window
pub const MIN_RTT_FILTER_LEN_SEC: u64 = 10;

/// BBR state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrState {
    /// Exponential bandwidth discovery
    Startup,
    /// Drain queue after startup
    Drain,
    /// Steady-state bandwidth probing
    ProbeBw(ProbeBwPhase),
    /// Periodic RTT measurement
    ProbeRtt,
}

/// Probe_BW sub-phases in BBRv3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeBwPhase {
    /// Cruising at estimated bandwidth
    Cruise,
    /// Refilling pipe after ProbeRTT
    Refill,
    /// Probing for more bandwidth
    Up,
    /// Reducing to drain any queue
    Down,
}

/// ACK phase for loss detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckPhase {
    /// Not in ACK processing
    Idle,
    /// Processing initial ACKs
    Initial,
    /// Processing ACKs normally
    Refilling,
}

/// BBR model parameters
#[derive(Debug, Clone)]
pub struct BbrModel {
    /// Estimated bottleneck bandwidth (bytes/sec)
    pub btl_bw: u64,
    /// Minimum RTT observed
    pub min_rtt: Duration,
    /// Maximum bandwidth sample
    pub max_bw: u64,
    /// Current RTT sample
    pub current_rtt: Duration,
    /// Inflight at time of max_bw sample
    pub max_inflight: u64,
    /// Timestamp of min_rtt
    pub min_rtt_stamp: Instant,
    /// Round count
    pub round_count: u64,
    /// Next round delivered count
    pub next_round_delivered: u64,
}

impl Default for BbrModel {
    fn default() -> Self {
        Self {
            btl_bw: 0,
            min_rtt: Duration::from_millis(100),
            max_bw: 0,
            current_rtt: Duration::from_millis(100),
            max_inflight: 0,
            min_rtt_stamp: Instant::now(),
            round_count: 0,
            next_round_delivered: 0,
        }
    }
}

/// BBRv3 congestion control state
#[derive(Debug)]
pub struct BbrCongestionControl {
    /// Current state
    pub state: BbrState,
    /// Model parameters
    pub model: BbrModel,
    /// Current pacing rate (bytes/sec)
    pub pacing_rate: u64,
    /// Current CWND (bytes)
    pub cwnd: u64,
    /// Bytes in flight
    pub inflight: u64,
    /// Bytes delivered total
    pub delivered: u64,
    /// Bytes lost total
    pub lost: u64,
    /// Pacing gain
    pub pacing_gain: f64,
    /// CWND gain
    pub cwnd_gain: f64,
    /// Are we in a round where we saw loss?
    pub loss_in_round: bool,
    /// ECN count in round
    pub ecn_in_round: u32,
    /// Full bandwidth reached (startup complete)
    pub full_bw_reached: bool,
    /// Full bandwidth count (consecutive rounds without growth)
    pub full_bw_count: u32,
    /// Last bandwidth sample
    pub last_bw: u64,
    /// Probe RTT done stamp
    pub probe_rtt_done_stamp: Option<Instant>,
    /// Extra ACKed during probe RTT
    pub extra_acked: u64,
    /// ACK phase
    pub ack_phase: AckPhase,
}

impl BbrCongestionControl {
    /// Create new BBRv3 instance
    pub fn new(mss: u32) -> Self {
        let initial_cwnd = (10 * mss) as u64;
        Self {
            state: BbrState::Startup,
            model: BbrModel::default(),
            pacing_rate: 0,
            cwnd: initial_cwnd,
            inflight: 0,
            delivered: 0,
            lost: 0,
            pacing_gain: STARTUP_PACING_GAIN,
            cwnd_gain: STARTUP_CWND_GAIN,
            loss_in_round: false,
            ecn_in_round: 0,
            full_bw_reached: false,
            full_bw_count: 0,
            last_bw: 0,
            probe_rtt_done_stamp: None,
            extra_acked: 0,
            ack_phase: AckPhase::Idle,
        }
    }

    /// Process an ACK
    pub fn on_ack(&mut self, acked_bytes: u64, rtt: Duration, now: Instant) {
        self.delivered += acked_bytes;
        self.inflight = self.inflight.saturating_sub(acked_bytes);
        self.model.current_rtt = rtt;

        // Update min_rtt
        if rtt < self.model.min_rtt
            || now.duration_since(self.model.min_rtt_stamp).as_secs() > MIN_RTT_FILTER_LEN_SEC
        {
            self.model.min_rtt = rtt;
            self.model.min_rtt_stamp = now;
        }

        // Estimate bandwidth
        let bw_sample = self.estimate_bandwidth(acked_bytes, rtt);
        self.update_max_bw(bw_sample);

        // Check for bandwidth plateau in startup
        if self.state == BbrState::Startup {
            self.check_full_bw_reached();
        }

        // State machine transitions
        self.update_state(now);

        // Update pacing and CWND
        self.update_pacing_rate();
        self.update_cwnd();
    }

    /// Process a loss event
    pub fn on_loss(&mut self, lost_bytes: u64) {
        self.lost += lost_bytes;
        self.inflight = self.inflight.saturating_sub(lost_bytes);
        self.loss_in_round = true;

        // BBRv3: React to loss more quickly than v1
        if self.state == BbrState::ProbeBw(ProbeBwPhase::Up) {
            // Immediately transition to Down phase
            self.state = BbrState::ProbeBw(ProbeBwPhase::Down);
            self.pacing_gain = 0.75;
        }
    }

    /// Process ECN feedback
    pub fn on_ecn(&mut self, ecn_count: u32) {
        self.ecn_in_round += ecn_count;

        // BBRv3 treats ECN similarly to loss
        if ecn_count > 0 && self.state == BbrState::ProbeBw(ProbeBwPhase::Up) {
            self.state = BbrState::ProbeBw(ProbeBwPhase::Down);
            self.pacing_gain = 0.75;
        }
    }

    /// Called when sending new data
    pub fn on_send(&mut self, bytes: u64) {
        self.inflight += bytes;
    }

    /// Estimate bandwidth from ACK
    fn estimate_bandwidth(&self, acked: u64, rtt: Duration) -> u64 {
        if rtt.as_nanos() == 0 {
            return 0;
        }
        // BW = delivered_bytes / rtt
        (acked as u128 * 1_000_000_000 / rtt.as_nanos()) as u64
    }

    /// Update maximum bandwidth filter
    fn update_max_bw(&mut self, bw_sample: u64) {
        if bw_sample > self.model.max_bw {
            self.model.max_bw = bw_sample;
            self.model.max_inflight = self.inflight;
        }
        self.model.btl_bw = self.model.max_bw;
    }

    /// Check if full bandwidth is reached (startup complete)
    fn check_full_bw_reached(&mut self) {
        if self.full_bw_reached {
            return;
        }

        let bw = self.model.btl_bw;

        // Check if BW grew by at least 25%
        if bw >= self.last_bw + (self.last_bw / 4) {
            self.last_bw = bw;
            self.full_bw_count = 0;
        } else {
            self.full_bw_count += 1;
            if self.full_bw_count >= 3 {
                self.full_bw_reached = true;
            }
        }
    }

    /// Update state machine
    fn update_state(&mut self, now: Instant) {
        match self.state {
            BbrState::Startup => {
                if self.full_bw_reached {
                    self.enter_drain();
                }
            }
            BbrState::Drain => {
                // Exit drain when inflight <= BDP
                let bdp = self.bdp();
                if self.inflight <= bdp {
                    self.enter_probe_bw();
                }
            }
            BbrState::ProbeBw(phase) => {
                self.update_probe_bw(phase, now);
            }
            BbrState::ProbeRtt => {
                self.update_probe_rtt(now);
            }
        }

        // Check if we should probe RTT
        if self.state != BbrState::ProbeRtt {
            let since_min_rtt = now.duration_since(self.model.min_rtt_stamp);
            if since_min_rtt.as_secs() > PROBE_RTT_INTERVAL_SEC {
                self.enter_probe_rtt();
            }
        }
    }

    fn enter_drain(&mut self) {
        self.state = BbrState::Drain;
        self.pacing_gain = DRAIN_PACING_GAIN;
        self.cwnd_gain = STARTUP_CWND_GAIN;
    }

    fn enter_probe_bw(&mut self) {
        self.state = BbrState::ProbeBw(ProbeBwPhase::Cruise);
        self.pacing_gain = 1.0;
        self.cwnd_gain = 2.0;
    }

    fn enter_probe_rtt(&mut self) {
        self.state = BbrState::ProbeRtt;
        self.pacing_gain = 1.0;
        self.probe_rtt_done_stamp =
            Some(Instant::now() + Duration::from_millis(PROBE_RTT_DURATION_MS));
    }

    fn update_probe_bw(&mut self, phase: ProbeBwPhase, _now: Instant) {
        // BBRv3 Probe_BW state machine
        match phase {
            ProbeBwPhase::Cruise => {
                self.pacing_gain = 1.0;
                // Periodically probe up
            }
            ProbeBwPhase::Refill => {
                self.pacing_gain = 1.0;
                // Transition to Up after refilling
            }
            ProbeBwPhase::Up => {
                self.pacing_gain = 1.25;
                // Look for loss/ECN to bound
            }
            ProbeBwPhase::Down => {
                self.pacing_gain = 0.75;
                // Return to cruise after draining
            }
        }

        // Reset loss tracking at round boundaries
        self.model.round_count += 1;
        self.loss_in_round = false;
        self.ecn_in_round = 0;
    }

    fn update_probe_rtt(&mut self, now: Instant) {
        if let Some(done_stamp) = self.probe_rtt_done_stamp {
            if now >= done_stamp {
                self.enter_probe_bw();
            }
        }
    }

    /// Update pacing rate
    fn update_pacing_rate(&mut self) {
        let rate = (self.model.btl_bw as f64 * self.pacing_gain) as u64;
        self.pacing_rate = rate.max(1);
    }

    /// Update CWND
    fn update_cwnd(&mut self) {
        let bdp = self.bdp();
        let cwnd = (bdp as f64 * self.cwnd_gain) as u64;

        // BBRv3: Limit CWND in ProbeRTT
        self.cwnd = if self.state == BbrState::ProbeRtt {
            4 * 1460 // 4 MSS
        } else {
            cwnd.max(4 * 1460)
        };
    }

    /// Calculate bandwidth-delay product
    pub fn bdp(&self) -> u64 {
        let rtt_secs = self.model.min_rtt.as_secs_f64();
        (self.model.btl_bw as f64 * rtt_secs) as u64
    }

    /// Get current pacing rate (bytes/sec)
    pub fn pacing_rate(&self) -> u64 {
        self.pacing_rate
    }

    /// Get current CWND
    pub fn cwnd(&self) -> u64 {
        self.cwnd
    }

    /// Check if can send more data
    pub fn can_send(&self) -> bool {
        self.inflight < self.cwnd
    }

    /// Get send quantum (bytes to send per pacing interval)
    pub fn send_quantum(&self) -> u64 {
        // Target 1ms pacing intervals
        (self.pacing_rate / 1000).max(1460)
    }
}

impl Default for BbrCongestionControl {
    fn default() -> Self {
        Self::new(1460)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_startup() {
        let mut bbr = BbrCongestionControl::new(1460);
        assert_eq!(bbr.state, BbrState::Startup);
        assert!(bbr.pacing_gain > 2.0);
    }

    #[test]
    fn test_bbr_ack_processing() {
        let mut bbr = BbrCongestionControl::new(1460);
        let now = Instant::now();

        bbr.on_send(10000);
        bbr.on_ack(10000, Duration::from_millis(50), now);

        assert!(bbr.model.btl_bw > 0);
        assert!(bbr.pacing_rate > 0);
    }
}
