//! Latency reduction and frame pacing for Redox OS
//!
//! This crate provides low-latency rendering optimizations:
//! - AMD Anti-Lag / Anti-Lag+
//! - NVIDIA Reflex
//! - Frame pacing and synchronization

#![no_std]

extern crate alloc;

pub mod common;
pub mod frame_pacing;

#[cfg(feature = "anti-lag")]
pub mod anti_lag;

#[cfg(feature = "reflex")]
pub mod reflex;

#[cfg(feature = "boost")]
pub mod boost;

pub use common::{LatencyError, LatencyMode, LatencyStats};
pub use frame_pacing::FramePacer;

/// Initialize latency reduction subsystem
pub fn init() -> Result<(), &'static str> {
    log::info!("Initializing latency reduction subsystem");

    #[cfg(feature = "anti-lag")]
    {
        log::info!("AMD Anti-Lag support enabled");
    }

    #[cfg(feature = "reflex")]
    {
        log::info!("NVIDIA Reflex support enabled");
    }

    #[cfg(feature = "boost")]
    {
        log::info!("Latency boost mode enabled");
    }

    Ok(())
}
