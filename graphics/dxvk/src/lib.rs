//! DXVK - DirectX to Vulkan Translation Layer
//!
//! This crate provides translation from DirectX APIs (D3D9, D3D11, D3D12)
//! to Vulkan for running Windows games on Redox OS.

#![no_std]

extern crate alloc;

pub mod common;

#[cfg(feature = "d3d9")]
pub mod d3d9;

#[cfg(feature = "d3d11")]
pub mod d3d11;

#[cfg(feature = "d3d12")]
pub mod d3d12;

pub use common::{DxvkAdapter, DxvkDevice, DxvkError};

use alloc::format;
use alloc::string::String;

/// DXVK version
pub const DXVK_VERSION: (u32, u32, u32) = (2, 3, 1);

/// Initialize DXVK
pub fn init() -> Result<(), &'static str> {
    log::info!(
        "Initializing DXVK {}.{}.{}",
        DXVK_VERSION.0,
        DXVK_VERSION.1,
        DXVK_VERSION.2
    );

    // Initialize Vulkan loader
    vulkan_loader::init()?;

    log::info!("DXVK initialized successfully");
    Ok(())
}

/// Get DXVK version string
pub fn version_string() -> String {
    String::from(format!(
        "{}.{}.{}",
        DXVK_VERSION.0, DXVK_VERSION.1, DXVK_VERSION.2
    ))
}
