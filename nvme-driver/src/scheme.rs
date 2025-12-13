// SPDX-FileCopyrightText: 2024 Redox OS Developers
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::task::Future;

use anyhow::bail;
use log::{error, info};
use redox_rt::signal::block_all_signals;
use spin::Mutex;

use nvme::{Command, Controller, InterruptMethod, Nvme, SubmissionQueue, CompletionQueue, NvmeFuture, Doorbell};

use syscall::{Io, Physmap, physmap, physunmap};

// Represents a queue pair
pub struct Queue {
    sq: Mutex<SubmissionQueue>,
    cq: Mutex<CompletionQueue>,
    doorbell: Doorbell,
    pending: Mutex<BTreeMap<u16, PendingRequest>>,
}

pub struct PendingRequest {
    future: NvmeFuture,
    packet: libredox::Packet,
    phys: Option<Physmap>,
}

pub struct NvmeScheme {
    pci_handle: usize,
    nvme: Nvme,
    next_ns_id: u32,
    namespaces: BTreeMap<u32, Arc<Mutex<Controller>>>,
    next_id: u64,
    handles: BTreeMap<u64, NvmeHandle>,
    pub queues: Vec<Arc<Queue>>,
    next_queue: usize,
}

pub struct NvmeHandle {
    ns_id: u32,
    ns: Arc<Mutex<Controller>>,
    queue: Arc<Queue>,
}

impl NvmeScheme {
    pub fn new(pci_handle: usize, pci_config: &[u8]) -> anyhow::Result<Self> {
        let mut nvme = Nvme::new(pci_config)?;
        nvme.init(InterruptMethod::MsiX)?;

        let mut namespaces = BTreeMap::new();
        let mut next_ns_id = 1;

        for i in 0..nvme.identify_controller().nvm_ns_count {
            let ns_id = i + 1;
            if let Some(ctrl) = nvme.namespace(ns_id) {
                namespaces.insert(next_ns_id, Arc::new(Mutex::new(ctrl)));
                next_ns_id += 1;
            }
        }

        let num_queues = num_cpus::get();
        let queues = nvme.create_io_queues(num_queues)?.into_iter().map(|(sq, cq, doorbell)| Arc::new(Queue {
            sq: Mutex::new(sq),
            cq: Mutex::new(cq),
            doorbell,
            pending: Mutex::new(BTreeMap::new()),
        })).collect();

        Ok(Self {
            pci_handle,
            nvme,
            next_ns_id,
            namespaces,
            next_id: 1,
            handles: BTreeMap::new(),
            queues,
            next_queue: 0,
        })
    }

    pub fn irq(&mut self, queue_index: u8) {
        let queue = &self.queues[queue_index as usize];
        let mut cq = queue.cq.lock();
        let mut pending = queue.pending.lock();
        while let Some(cqe) = cq.get_completion() {
            if let Some(mut pending_req) = pending.remove(&cqe.command_id()) {
                if let Some(phys) = pending_req.phys.take() {
                    unsafe { physunmap(phys.address, phys.size).expect("nvme: failed to unmap physical memory"); }
                }
                pending_req.packet.a = cqe.status().into();
                syscall::write(self.pci_handle, &pending_req.packet).expect("nvme: failed to write packet");
            }
        }
    }

    pub fn handle(&mut self, packet: &mut libredox::Packet) {
        let (a, b, c, d) = libredox::flag::decode_usize(packet.a);
        match (a, b, c, d) {
            (libredox::flag::SYS_OPEN, _, _, _) => {
                let path = unsafe { std::str::from_utf8_unchecked(libredox::slice::from_raw_parts(packet.c as *const u8, packet.d)) };
                let parts: Vec<&str> = path.split('/').collect();
                if let Some(ns_str) = parts.get(0) {
                    if let Ok(ns_id) = ns_str.parse::<u32>() {
                        if let Some(ns) = self.namespaces.get(&ns_id) {
                            let id = self.next_id;
                            self.next_id += 1;
                            let queue = Arc::clone(&self.queues[self.next_queue]);
                            self.next_queue = (self.next_queue + 1) % self.queues.len();
                            self.handles.insert(id, NvmeHandle {
                                ns_id,
                                ns: Arc::clone(ns),
                                queue,
                            });
                            packet.a = id as usize;
                        } else {
                            packet.a = syscall::Error::new(syscall::ENODEV).to_errno();
                        }
                    } else {
                        packet.a = syscall::Error::new(syscall::ENODEV).to_errno();
                    }
                } else {
                    //TODO: return list of namespaces
                    packet.a = syscall::Error::new(syscall::ENODEV).to_errno();
                }
            }
            (libredox::flag::SYS_READ, _, _, _) => {
                let id = packet.b as u64;
                let offset = packet.e as u64;

                if let Some(handle) = self.handles.get(&id) {
                    let (phys, slice) = if packet.c & 1 == 1 {
                        let phys = unsafe { physmap(packet.c & !1, packet.d, 0).expect("nvme: failed to map physical memory") };
                        (Some(phys), unsafe { std::slice::from_raw_parts_mut(phys.address as *mut u8, phys.size) })
                    } else {
                        (None, unsafe { std::slice::from_raw_parts_mut(packet.c as *mut u8, packet.d) })
                    };

                    let mut ns = handle.ns.lock();
                    let mut sq = handle.queue.sq.lock();
                    let mut pending = handle.queue.pending.lock();
                    let future = ns.read_interrupt(offset, slice, &mut sq, &handle.queue.doorbell);
                    pending.insert(future.command_id(), PendingRequest {
                        future,
                        packet: *packet,
                        phys,
                    });
                } else {
                    packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                }
            }
            (libredox::flag::SYS_WRITE, _, _, _) => {
                let id = packet.b as u64;
                let offset = packet.e as u64;

                if let Some(handle) = self.handles.get(&id) {
                    let (phys, slice) = if packet.c & 1 == 1 {
                        let phys = unsafe { physmap(packet.c & !1, packet.d, 0).expect("nvme: failed to map physical memory") };
                        (Some(phys), unsafe { std::slice::from_raw_parts(phys.address as *const u8, phys.size) })
                    } else {
                        (None, unsafe { std::slice::from_raw_parts(packet.c as *const u8, packet.d) })
                    };

                    let mut ns = handle.ns.lock();
                    let mut sq = handle.queue.sq.lock();
                    let mut pending = handle.queue.pending.lock();
                    let future = ns.write_interrupt(offset, slice, &mut sq, &handle.queue.doorbell);
                    pending.insert(future.command_id(), PendingRequest {
                        future,
                        packet: *packet,
                        phys,
                    });
                } else {
                    packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                }
            }
            (libredox::flag::SYS_FSTAT, _, _, _) => {
                let id = packet.b as u64;
                if let Some(handle) = self.handles.get(&id) {
                    let ns = handle.ns.lock();
                    let stat = libredox::Stat {
                        st_mode: libredox::flag::MODE_FILE,
                        st_size: ns.size(),
                        ..Default::default()
                    };
                    let buf = unsafe { libredox::slice::from_raw_parts_mut(packet.c as *mut libredox::Stat, 1) };
                    buf[0] = stat;
                    packet.a = 0;
                } else {
                    packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                }
            }
            (libredox::flag::SYS_FPATH, _, _, _) => {
                let id = packet.b as u64;
                if let Some(handle) = self.handles.get(&id) {
                    let path = format!("nvme:{}/", handle.ns_id);
                    let buf = unsafe { libredox::slice::from_raw_parts_mut(packet.c as *mut u8, packet.d) };
                    let mut i = 0;
                    for b in path.bytes() {
                        if i < buf.len() {
                            buf[i] = b;
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    packet.a = i;
                } else {
                    packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                }
            }
            (libredox::flag::SYS_CLOSE, _, _, _) => {
                let id = packet.b as u64;
                if self.handles.remove(&id).is_some() {
                    packet.a = 0;
                } else {
                    packet.a = syscall::Error::new(syscall::EBADF).to_errno();
                }
            }
            _ => {
                error!("nvme: unknown syscall {}", a);
                packet.a = syscall::Error::new(syscall::ENOSYS).to_errno();
            }
        }
    }
}
