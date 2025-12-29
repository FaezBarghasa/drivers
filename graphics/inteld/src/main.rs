//! Intel GPU Driver (Xe/i915)
//!
//! Native Intel GPU driver with GEM memory manager and GuC firmware.

use redox_daemon::Daemon;
use std::sync::Arc;

mod context;
mod device;
mod display;
mod execbuf;
mod gal_backend;
mod gem;
mod gtt;
mod guc;
mod huc;
mod ring;

use device::IntelDevice;
use gal_backend::IntelGalBackend;

fn daemon(daemon: Daemon) -> ! {
    common::setup_logging(
        "gpu",
        "intel",
        "inteld",
        common::output_level(),
        common::file_level(),
    );

    log::info!("Intel GPU Driver starting...");

    // Initialize PCI device
    let device = match IntelDevice::new() {
        Ok(dev) => {
            log::info!(
                "Intel GPU detected: {:04x}:{:04x} (Gen{})",
                dev.vendor_id(),
                dev.device_id(),
                dev.generation()
            );
            Arc::new(dev)
        }
        Err(e) => {
            log::error!("Failed to initialize Intel GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Load GuC/HuC firmware
    if let Err(e) = device.load_firmware() {
        log::error!("Failed to load firmware: {}", e);
        std::process::exit(1);
    }

    // Initialize GEM
    if let Err(e) = device.init_gem() {
        log::error!("Failed to initialize GEM: {}", e);
        std::process::exit(1);
    }

    // Initialize GTT/PPGTT
    if let Err(e) = device.init_gtt() {
        log::error!("Failed to initialize GTT: {}", e);
        std::process::exit(1);
    }

    // Initialize rings
    if let Err(e) = device.init_rings() {
        log::error!("Failed to initialize rings: {}", e);
        std::process::exit(1);
    }

    // Initialize display
    if let Err(e) = device.init_display() {
        log::error!("Failed to initialize display: {}", e);
        std::process::exit(1);
    }

    // Create GAL backend
    let gal_backend = Arc::new(IntelGalBackend::new(device.clone()));

    // Register with kernel GAL
    if let Err(e) = gal_backend.register() {
        log::error!("Failed to register with GAL: {}", e);
        std::process::exit(1);
    }

    log::info!("Intel GPU driver ready");

    daemon.ready().expect("Failed to mark daemon as ready");

    // Main event loop
    loop {
        device.process_events();
        device.process_submissions();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn main() {
    Daemon::new(daemon).expect("inteld: failed to create daemon");
}
