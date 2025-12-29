//! Upscaling support for Redox OS
//!
//! This crate provides integration for modern upscaling technologies:
//! - AMD FidelityFX Super Resolution (FSR)
//! - NVIDIA Deep Learning Super Sampling (DLSS)
//! - Intel Xe Super Sampling (XeSS)

#![no_std]

extern crate alloc;

pub mod common;

#[cfg(feature = "fsr")]
pub mod fsr;

#[cfg(feature = "dlss")]
pub mod dlss;

#[cfg(feature = "xess")]
pub mod xess;

pub use common::{UpscalingBackend, UpscalingContext, UpscalingError, UpscalingQuality};

use alloc::vec::Vec;

/// Initialize upscaling subsystem
pub fn init() -> Result<(), &'static str> {
    log::info!("Initializing upscaling subsystem");

    #[cfg(feature = "fsr")]
    {
        log::info!("FSR support enabled");
    }

    #[cfg(feature = "dlss")]
    {
        log::info!("DLSS support enabled");
    }

    #[cfg(feature = "xess")]
    {
        log::info!("XeSS support enabled");
    }

    Ok(())
}

/// Detect available upscaling backends
pub fn detect_backends() -> Vec<UpscalingBackend> {
    let mut backends = Vec::new();

    #[cfg(feature = "fsr")]
    {
        backends.push(UpscalingBackend::FSR);
    }

    #[cfg(feature = "dlss")]
    {
        // Check for NVIDIA GPU
        backends.push(UpscalingBackend::DLSS);
    }

    #[cfg(feature = "xess")]
    {
        // Check for Intel GPU
        backends.push(UpscalingBackend::XeSS);
    }

    backends
}
