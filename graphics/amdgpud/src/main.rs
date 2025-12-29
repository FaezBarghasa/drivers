//! AMD GPU Driver (AMDGPU)
//!
//! Native AMD GPU driver with GEM memory manager and kernel GAL integration.

use redox_daemon::Daemon;
use std::sync::Arc;

mod device;
mod display;
mod fence;
mod firmware;
mod gal_backend;
mod gem;
mod ring;
mod scheduler;

use device::AmdDevice;
use gal_backend::AmdGalBackend;

fn daemon(daemon: Daemon) -> ! {
    common::setup_logging(
        "gpu",
        "amd",
        "amdgpud",
        common::output_level(),
        common::file_level(),
    );

    log::info!("AMD GPU Driver starting...");

    // Initialize PCI device
    let device = match AmdDevice::new() {
        Ok(dev) => {
            log::info!(
                "AMD GPU detected: {:04x}:{:04x}",
                dev.vendor_id(),
                dev.device_id()
            );
            Arc::new(dev)
        }
        Err(e) => {
            log::error!("Failed to initialize AMD GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize GEM memory manager
    if let Err(e) = device.init_gem() {
        log::error!("Failed to initialize GEM: {}", e);
        std::process::exit(1);
    }

    // Initialize command rings
    if let Err(e) = device.init_rings() {
        log::error!("Failed to initialize rings: {}", e);
        std::process::exit(1);
    }

    // Initialize display engine
    if let Err(e) = device.init_display() {
        log::error!("Failed to initialize display: {}", e);
        std::process::exit(1);
    }

    // Create GAL backend
    let gal_backend = Arc::new(AmdGalBackend::new(device.clone()));

    // Register with kernel GAL
    if let Err(e) = gal_backend.register() {
        log::error!("Failed to register with GAL: {}", e);
        std::process::exit(1);
    }

    log::info!("AMD GPU driver ready");

    daemon.ready().expect("Failed to mark daemon as ready");

    // Main event loop
    loop {
        // Process GPU events
        device.process_events();

        // Process command submissions
        device.process_submissions();

        // Sleep briefly
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn main() {
    Daemon::new(daemon).expect("amdgpud: failed to create daemon");
}
