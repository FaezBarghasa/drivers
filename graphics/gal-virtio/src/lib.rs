//! VirtIO-GPU Backend for the Graphics Abstraction Layer
//!
//! This crate provides a VirtIO-GPU implementation of the GAL traits,
//! enabling hardware-accelerated 2D/3D graphics in virtual machines.
//!
//! # Architecture
//!
//! The VirtIO-GPU backend translates GAL operations to VirtIO-GPU commands:
//!
//! ```text
//! ┌─────────────────────┐
//! │    Application      │
//! │   (Vulkan/WGPU)     │
//! └──────────┬──────────┘
//!            │
//! ┌──────────▼──────────┐
//! │   GAL Interface     │
//! │ (Device, Commands)  │
//! └──────────┬──────────┘
//!            │
//! ┌──────────▼──────────┐
//! │   VirtIO-GPU GAL    │◄─── This crate
//! │     Backend         │
//! └──────────┬──────────┘
//!            │
//! ┌──────────▼──────────┐
//! │   VirtIO Queues     │
//! │  (Control/Cursor)   │
//! └──────────┬──────────┘
//!            │
//! ┌──────────▼──────────┐
//! │   Host GPU/Mesa     │
//! │(virgl/venus driver) │
//! └─────────────────────┘
//! ```
//!
//! # Capabilities
//!
//! - **2D Mode**: Basic framebuffer blitting, cursor support
//! - **3D Mode (virgl)**: OpenGL ES 3.0+ via Mesa virgl
//! - **3D Mode (venus)**: Vulkan 1.2+ via Mesa Venus
//!
//! # Usage
//!
//! ```ignore
//! use gal_virtio::VirtioGpuDevice;
//!
//! let device = VirtioGpuDevice::create()?;
//! println!("Device: {}", device.info().name);
//! println!("Capabilities: {:?}", device.info().capabilities);
//! ```

#![no_std]

extern crate alloc;

mod command;
mod device;
mod protocol;
mod resource;

pub use command::VirtioCommandBuffer;
pub use device::VirtioGpuDevice;
pub use resource::{VirtioBuffer, VirtioImage};
