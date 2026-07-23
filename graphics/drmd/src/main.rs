//! Redox OS Direct Rendering Manager (DRM) Daemon (`drmd`)
//!
//! Provides the system-wide `:drm` scheme endpoint for Mesa, Vulkan loader,
//! and graphics clients. Manages GEM buffer handle creation, memory mapping (`mmap`),
//! command submission ring buffer synchronization, and mode setting (KMS).

use std::collections::HashMap;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::atomic::{AtomicU32, Ordering};

use libredox::flag;
use redox_daemon::Daemon;
extern crate event;
use event::{user_data, EventFlags, EventQueue};
use syscall::Packet;

static NEXT_GEM_HANDLE: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct GemBuffer {
    pub handle: u32,
    pub size: usize,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
}

pub struct DrmDevice {
    pub name: String,
    pub buffers: HashMap<u32, GemBuffer>,
}

impl DrmDevice {
    pub fn new() -> Self {
        Self {
            name: String::from("Redox-DRM-VirtIO"),
            buffers: HashMap::new(),
        }
    }

    pub fn create_gem(&mut self, width: u32, height: u32, bpp: u32) -> GemBuffer {
        let handle = NEXT_GEM_HANDLE.fetch_add(1, Ordering::SeqCst);
        let pitch = width * (bpp / 8);
        let size = (pitch * height) as usize;
        let buf = GemBuffer {
            handle,
            size,
            pitch,
            width,
            height,
            bpp,
        };
        self.buffers.insert(handle, buf.clone());
        buf
    }
}

user_data! {
    enum Source {
        Scheme,
    }
}

fn daemon(daemon: Daemon) -> ! {
    common::setup_logging(
        "graphics",
        "drm",
        "drmd",
        common::output_level(),
        common::file_level(),
    );

    log::info!("Starting Redox OS DRM Daemon (drmd)...");

    let socket_fd = match libredox::call::open(
        ":drm",
        flag::O_RDWR | flag::O_CREAT | flag::O_NONBLOCK,
        0,
    ) {
        Ok(fd) => fd,
        Err(err) => {
            log::error!("drmd: failed to create :drm scheme socket: {:?}", err);
            std::process::exit(1);
        }
    };

    let mut socket = unsafe { File::from_raw_fd(socket_fd as RawFd) };

    daemon.ready().expect("drmd: failed to signal daemon readiness");

    let event_queue = EventQueue::<Source>::new().expect("drmd: failed to create event queue");
    event_queue
        .subscribe(socket_fd, Source::Scheme, EventFlags::READ)
        .expect("drmd: failed to subscribe scheme socket");

    let mut device = DrmDevice::new();

    log::info!("Redox OS DRM Daemon initialized successfully.");

    for event in event_queue.map(|e| e.expect("drmd event error").user_data) {
        match event {
            Source::Scheme => {
                loop {
                    let mut packet = Packet::default();
                    match socket.read(&mut packet) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(err) => {
                            if err.kind() == ErrorKind::WouldBlock {
                                break;
                            }
                            log::error!("drmd: read error: {:?}", err);
                            break;
                        }
                    }

                    let res = handle_drm_packet(&mut packet, &mut device);
                    packet.a = res;
                    let _ = socket.write(&packet);
                }
            }
        }
    }

    std::process::exit(0);
}

fn handle_drm_packet(packet: &mut Packet, device: &mut DrmDevice) -> usize {
    match packet.a {
        syscall::SYS_OPEN => 1, // File descriptor index
        syscall::SYS_WRITE => {
            // Command submission & GEM allocation ioctls
            let width = packet.c as u32;
            let height = packet.d as u32;
            if width > 0 && height > 0 {
                let buf = device.create_gem(width, height, 32);
                buf.handle as usize
            } else {
                0
            }
        }
        syscall::SYS_CLOSE => 0,
        _ => usize::MAX,
    }
}

fn main() {
    Daemon::new(daemon).expect("drmd: failed to daemonize");
}
