// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

#![feature(map_try_insert)]

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::Arc;
use std::thread;

use anyhow::bail;
use event::{Event, EventQueue};
use log::{error, info};
use redox_log;
use redox_rt::signal::block_all_signals;
use spin::Mutex;

use crate::scheme::NvmeScheme;

mod scheme;

fn main() {
    redox_log::init!();
    info!("starting nvme driver");

    let pci_handle = unsafe {
        libredox::call::open(
            "pci:class=010802",
            libredox::flag::O_RDWR | libredox::flag::O_CLOEXEC,
        )
        .expect("nvme: failed to open pci")
    };

    let mut pci_config = [0; 4096];
    let pci_config_slice =
        unsafe { std::slice::from_raw_parts_mut(pci_config.as_mut_ptr() as *mut u8, 4096) };
    let pci_config_len =
        libredox::call::read(pci_handle, pci_config_slice).expect("nvme: failed to read pci config");

    let scheme_file = libredox::call::open(
        "nvme:",
        libredox::flag::O_RDWR | libredox::flag::O_CREAT | libredox::flag::O_CLOEXEC,
    )
    .expect("nvme: failed to create nvme scheme");

    let mut scheme = NvmeScheme::new(pci_handle, &pci_config[..pci_config_len])
        .expect("nvme: failed to create scheme");

    let scheme_fd = scheme_file as RawFd;
    let scheme_clone = Arc::new(Mutex::new(scheme));

    let mut threads = Vec::new();
    for (i, queue) in scheme_clone.lock().queues.iter().enumerate() {
        let queue_clone = Arc::clone(queue);
        let scheme_clone = Arc::clone(&scheme_clone);
        let irq_number = i as u8;

        threads.push(thread::spawn(move || {
            let mut event_queue = EventQueue::<()>::new().expect("nvme: failed to create event queue");
            let irq_file = File::open(format!("irq:{}", irq_number)).expect("nvme: failed to open irq file");
            let irq_fd = irq_file.into_raw_fd();

            event_queue.add(irq_fd, move |_| {
                let mut irq_buf = [0; 8];
                let bytes = syscall::read(irq_fd, &mut irq_buf).expect("nvme: failed to read irq file");
                if bytes == 8 {
                    scheme_clone.lock().irq(irq_number);
                    syscall::write(irq_fd, &irq_buf).expect("nvme: failed to write irq file");
                }
                Ok(None)
            }).expect("nvme: failed to listen to irq events");

            loop {
                if let Err(err) = event_queue.run() {
                    error!("nvme: event loop failed: {}", err);
                }
            }
        }));
    }

    let mut event_queue = EventQueue::<()>::new().expect("nvme: failed to create event queue");
    event_queue.add(scheme_fd, move |_| {
        let mut packets = Vec::new();
        loop {
            let mut packet = libredox::Packet::default();
            match syscall::read(scheme_fd, &mut packet) {
                Ok(0) => break,
                Ok(_) => packets.push(packet),
                Err(err) if err.errno == syscall::EAGAIN => break,
                Err(err) => {
                    error!("failed to read scheme: {}", err);
                    break;
                }
            }
        }

        for packet in packets {
            scheme_clone.lock().handle(&packet);
            syscall::write(scheme_fd, &packet).expect("nvme: failed to write packet");
        }
        Ok(None)
    }).expect("nvme: failed to listen to scheme events");

    loop {
        if let Err(err) = event_queue.run() {
            error!("nvme: event loop failed: {}", err);
        }
    }
}
