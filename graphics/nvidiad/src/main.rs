//! NVIDIA GPU Driver (Open-Kernel)
//!
//! Native NVIDIA GPU driver with TTM memory manager and kernel GAL integration.

use redox_daemon::Daemon;
use std::sync::Arc;

mod channel;
mod device;
mod display;
mod fence;
mod firmware;
mod gal_backend;
mod pushbuf;
mod scheduler;
mod ttm;

use device::NvidiaDevice;
use gal_backend::NvidiaGalBackend;

fn daemon(daemon: Daemon) -> ! {
    common::setup_logging(
        "gpu",
        "nvidia",
        "nvidiad",
        common::output_level(),
        common::file_level(),
    );

    log::info!("NVIDIA GPU Driver starting...");

    // Initialize PCI device
    let device = match NvidiaDevice::new() {
        Ok(dev) => {
            log::info!(
                "NVIDIA GPU detected: {:04x}:{:04x}",
                dev.vendor_id(),
                dev.device_id()
            );
            Arc::new(dev)
        }
        Err(e) => {
            log::error!("Failed to initialize NVIDIA GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Load GSP-RM firmware
    if let Err(e) = device.load_firmware() {
        log::error!("Failed to load firmware: {}", e);
        std::process::exit(1);
    }

    // Initialize TTM memory manager
    if let Err(e) = device.init_ttm() {
        log::error!("Failed to initialize TTM: {}", e);
        std::process::exit(1);
    }

    // Initialize channels
    if let Err(e) = device.init_channels() {
        log::error!("Failed to initialize channels: {}", e);
        std::process::exit(1);
    }

    // Initialize display
    if let Err(e) = device.init_display() {
        log::error!("Failed to initialize display: {}", e);
        std::process::exit(1);
    }

    // Create GAL backend
    let gal_backend = Arc::new(NvidiaGalBackend::new(device.clone()));

    // Register with kernel GAL
    if let Err(e) = gal_backend.register() {
        log::error!("Failed to register with GAL: {}", e);
        std::process::exit(1);
    }

    log::info!("NVIDIA GPU driver ready");

    daemon.ready().expect("Failed to mark daemon as ready");

    // Main event loop
    loop {
        device.process_events();
        device.process_submissions();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn main() {
    Daemon::new(daemon).expect("nvidiad: failed to create daemon");
}
