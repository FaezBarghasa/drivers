use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;

use driver_network::NetworkScheme;
use event::{user_data, EventQueue};
use pcid_interface::PciFunctionHandle;

pub mod device;
pub mod ring_defs;
#[rustfmt::skip]
mod ixgbe;

fn main() {
    let mut pcid_handle = PciFunctionHandle::connect_default();
    let pci_config = pcid_handle.config();

    let mut name = pci_config.func.name();
    name.push_str("_ixgbe");

    let irq = pci_config
        .func
        .legacy_interrupt_line
        .expect("ixgbed: no legacy interrupts supported");

    println!(" + IXGBE {}", pci_config.func.display());

    redox_daemon::Daemon::new(move |daemon| {
        let mut irq_file = irq.irq_handle("ixgbed");

        // Open and setup shared ring for RDMA
        let ring_fd = libredox::call::open(
            "ring:ixgbed",
            libredox::flag::O_RDWR | libredox::flag::O_CREAT | libredox::flag::O_CLOEXEC,
            0,
        )
        .expect("ixgbed: failed to open ring");

        libredox::call::fcntl(
            ring_fd,
            libredox::flag::F_SETOWN,
            libredox::call::getpid().unwrap(),
        )
        .expect("ixgbed: failed to setown");

        let ring_size = 4096 * 2; // 8KB
        let ring_mem = unsafe {
            libredox::call::mmap(libredox::MmapArgs {
                fd: ring_fd,
                offset: 0,
                length: ring_size,
                prot: libredox::flag::PROT_READ | libredox::flag::PROT_WRITE,
                flags: libredox::flag::MAP_SHARED,
                addr: core::ptr::null_mut(),
            })
            .expect("ixgbed: failed to mmap ring")
        } as *mut ring_defs::IpcRing;

        let mapped_bar = unsafe { pcid_handle.map_bar(0) };
        let address = mapped_bar.ptr.as_ptr();
        let size = mapped_bar.bar_size;

        let device = device::Intel8259x::new(address as usize, size, ring_mem)
            .expect("ixgbed: failed to allocate device");

        let mut scheme = NetworkScheme::new(device, format!("network.{name}"));

        println!("   - BBRv3 congestion control enabled");
        println!("   - Monitoring available at network.{name}:bbr and network.{name}:bbr_raw");
        println!("   - RDMA Ring active at ring:ixgbed");

        user_data! {
            enum Source {
                Irq,
                Scheme,
                Ring,
            }
        }

        let event_queue =
            EventQueue::<Source>::new().expect("ixgbed: Could not create event queue.");
        event_queue
            .subscribe(
                irq_file.as_raw_fd() as usize,
                Source::Irq,
                event::EventFlags::READ,
            )
            .unwrap();
        event_queue
            .subscribe(
                scheme.event_handle().raw(),
                Source::Scheme,
                event::EventFlags::READ,
            )
            .unwrap();
        event_queue
            .subscribe(ring_fd, Source::Ring, event::EventFlags::READ)
            .unwrap();

        libredox::call::setrens(0, 0).expect("ixgbed: failed to enter null namespace");

        daemon
            .ready()
            .expect("ixgbed: failed to mark daemon as ready");

        scheme.tick().unwrap();

        for event in event_queue.map(|e| e.expect("ixgbed: failed to get next event")) {
            match event.user_data {
                Source::Irq => {
                    let mut irq = [0; 8];
                    irq_file.read(&mut irq).unwrap();
                    if scheme.adapter().irq() {
                        irq_file.write(&mut irq).unwrap();

                        scheme.tick().unwrap();
                    }
                }
                Source::Scheme => {
                    scheme.tick().unwrap();
                }
                Source::Ring => {
                    scheme.adapter_mut().process_ring();
                }
            }
        }
        unreachable!()
    })
    .expect("ixgbed: failed to create daemon");
}
