//! A Rust implementation of the BBRv3 congestion control algorithm.

use std::time::{Duration, Instant};

// The BBRv3 state machine is defined by the following states:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrState {
    /// Startup is the initial state of the BBRv3 algorithm. In this state, BBR
    /// rapidly increases its sending rate to probe the available bandwidth.
    Startup,
    /// Drain is entered after the available bandwidth has been estimated. In this
    /// state, BBR drains the queue of packets that were sent during startup.
    Drain,
    /// ProbeBw is the main state of the BBRv3 algorithm. In this state, BBR
    /// probes the available bandwidth by sending at a rate that is slightly
    *// higher than the estimated bandwidth.
    ProbeBw,
    /// ProbeRtt is entered when the round-trip time (RTT) has been stable for a
    /// certain period of time. In this state, BBR reduces its sending rate to
    /// probe the minimum RTT.
    ProbeRtt,
}

/// The BBRv3 congestion control algorithm.
#[derive(Debug, Clone)]
pub struct Bbr {
    /// The current state of the BBRv3 state machine.
    state: BbrState,
    /// The estimated maximum bandwidth available, in bytes per second.
    max_bw: u64,
    /// The estimated minimum round-trip time.
    min_rtt: Duration,
    /// The time at which the current min_rtt was measured.
    min_rtt_timestamp: Instant,
    /// The current pacing rate, in bytes per second.
    pacing_rate: u64,
    /// The time at which the current ProbeBw cycle started.
    probe_bw_start: Instant,
    /// The current gain being used in the ProbeBw state.
    probe_bw_gain_idx: usize,
}

// The pacing gains to use in the ProbeBw state.
const PROBE_BW_GAINS: [f64; 8] = [1.25, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

impl Bbr {
    /// Creates a new BBRv3 instance.
    pub fn new() -> Self {
        Self {
            state: BbrState::Startup,
            max_bw: 0,
            min_rtt: Duration::from_secs(u64::MAX),
            min_rtt_timestamp: Instant::now(),
            pacing_rate: 0,
            probe_bw_start: Instant::now(),
            probe_bw_gain_idx: 0,
        }
    }

    /// Returns the current state of the BBRv3 state machine.
    pub fn state(&self) -> BbrState {
        self.state
    }

    /// This function is called when a packet is acknowledged.
    pub fn on_ack(&mut self, bytes: u64, rtt: Duration) {
        if rtt < self.min_rtt {
            self.min_rtt = rtt;
            self.min_rtt_timestamp = Instant::now();
        }
        let bw = bytes as f64 / rtt.as_secs_f64();
        if bw > self.max_bw as f64 {
            self.max_bw = bw as u64;
        }

        match self.state {
            BbrState::Startup => {
                // In startup, we increase the pacing rate by 25% for each RTT.
                self.pacing_rate += self.pacing_rate / 4;

                // If we have been in startup for 3 RTTs, transition to Drain.
                // This is a simplified condition. A real implementation would
                // check if the bandwidth has stopped growing.
                if self.min_rtt != Duration::from_secs(u64::MAX) {
                    self.state = BbrState::Drain;
                }
            }
            BbrState::Drain => {
                // In Drain, we reduce the pacing rate to the estimated bandwidth.
                self.pacing_rate = self.max_bw;
                // Once the number of packets in flight is equal to the BDP,
                // transition to ProbeBw. This is a simplified condition.
                self.state = BbrState::ProbeBw;
                self.probe_bw_start = Instant::now();
                self.probe_bw_gain_idx = 0;
            }
            BbrState::ProbeBw => {
                // In ProbeBw, we cycle through a series of pacing gains.
                let gain = PROBE_BW_GAINS[self.probe_bw_gain_idx];
                self.pacing_rate = (self.max_bw as f64 * gain) as u64;

                // Each gain is used for one RTT.
                if self.probe_bw_start.elapsed() > self.min_rtt {
                    self.probe_bw_gain_idx = (self.probe_bw_gain_idx + 1) % PROBE_BW_GAINS.len();
                    self.probe_bw_start = Instant::now();
                }

                // If the RTT has been stable for 10 seconds, transition to
                // ProbeRtt.
                if self.min_rtt_expired() {
                    self.state = BbrState::ProbeRtt;
                }
            }
            BbrState::ProbeRtt => {
                // In ProbeRtt, we reduce the sending rate to the minimum of the
                // current pacing rate and the BDP. We stay in this state for
                // 200ms.
                self.pacing_rate = self.pacing_rate.min(self.bdp());
                if self.min_rtt_timestamp.elapsed() > Duration::from_millis(200) {
                    self.state = BbrState::ProbeBw;
                    self.probe_bw_start = Instant::now();
                    self.probe_bw_gain_idx = 0;
                }
            }
        }
    }

    /// Returns the current pacing rate.
    pub fn pacing_rate(&self) -> u64 {
        self.pacing_rate
    }

    // Returns true if the RTT has not been updated for 10 seconds.
    fn min_rtt_expired(&self) -> bool {
        self.min_rtt_timestamp.elapsed() > Duration::from_secs(10)
    }

    // Returns the bandwidth-delay product.
    fn bdp(&self) -> u64 {
        (self.max_bw as f64 * self.min_rtt.as_secs_f64()) as u64
    }
}

impl Default for Bbr {
    fn default() -> Self {
        Self::new()
    }
}
