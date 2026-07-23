//! Redox OS Audio Daemon (`audiod`)
//!
//! Provides the system-wide `:audio` scheme endpoint for user applications,
//! supporting low-latency sound mixing, volume control, and hardware routing.

use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::sync::atomic::{AtomicUsize, Ordering};

use libredox::flag;
use redox_daemon::Daemon;
extern crate event;

use event::{user_data, EventFlags, EventQueue};
use syscall::Packet;

mod mixer;
use mixer::AudioMixer;

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);

user_data! {
    enum Source {
        Scheme,
        Hw,
    }
}

fn daemon(daemon: Daemon) -> ! {
    common::setup_logging(
        "audio",
        "daemon",
        "audiod",
        common::output_level(),
        common::file_level(),
    );

    log::info!("Starting Redox OS Audio Daemon (audiod)...");

    let socket_fd = match libredox::call::open(
        ":audio",
        flag::O_RDWR | flag::O_CREAT | flag::O_NONBLOCK,
        0,
    ) {
        Ok(fd) => fd,
        Err(err) => {
            log::error!("audiod: failed to create :audio scheme socket: {:?}", err);
            std::process::exit(1);
        }
    };

    let mut socket = unsafe { File::from_raw_fd(socket_fd as RawFd) };

    // Attempt to open hardware audio scheme if available
    let hw_file = libredox::call::open(":audiohw", flag::O_RDWR | flag::O_NONBLOCK, 0)
        .ok()
        .map(|fd| unsafe { File::from_raw_fd(fd as RawFd) });

    daemon.ready().expect("audiod: failed to signal daemon readiness");

    let event_queue = EventQueue::<Source>::new().expect("audiod: failed to create event queue");
    event_queue
        .subscribe(socket_fd, Source::Scheme, EventFlags::READ)
        .expect("audiod: failed to subscribe scheme socket");

    if let Some(ref hw) = hw_file {
        event_queue
            .subscribe(hw.as_raw_fd() as usize, Source::Hw, EventFlags::WRITE)
            .expect("audiod: failed to subscribe hw socket");
    }

    let mut mixer = AudioMixer::new();
    let mut mix_buffer = vec![0u8; 4096];

    log::info!("Redox OS Audio Daemon initialized successfully.");

    for event in event_queue.map(|e| e.expect("audiod event error").user_data) {
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
                            log::error!("audiod: read error: {:?}", err);
                            break;
                        }
                    }

                    // Process scheme request packet
                    let res = handle_packet(&mut packet, &mut mixer);
                    packet.a = res;
                    let _ = socket.write(&packet);
                }
            }
            Source::Hw => {
                let bytes_mixed = mixer.mix(&mut mix_buffer);
                if let Some(ref mut hw) = hw_file.as_ref() {
                    let _ = hw.write(&mix_buffer[..bytes_mixed]);
                }
            }
        }
    }

    std::process::exit(0);
}

fn handle_packet(packet: &mut Packet, mixer: &mut AudioMixer) -> usize {
    match packet.a {
        syscall::SYS_OPEN => {
            let handle_id = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
            mixer.add_client(handle_id);
            handle_id
        }
        syscall::SYS_WRITE => {
            let handle_id = packet.b;
            let ptr = packet.c as *const u8;
            let len = packet.d;
            if ptr.is_null() || len == 0 {
                return 0;
            }
            let data = unsafe { std::slice::from_raw_parts(ptr, len) };
            mixer.write_client_data(handle_id, data)
        }
        syscall::SYS_CLOSE => {
            let handle_id = packet.b;
            mixer.remove_client(handle_id);
            0
        }
        _ => usize::MAX, // ENOSYS
    }
}

fn main() {
    Daemon::new(daemon).expect("audiod: failed to daemonize");
}
