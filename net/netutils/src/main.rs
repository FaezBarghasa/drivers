#![feature(cfg_version)]

#[cfg(all(feature = "ping", not(version("1.70.0"))))]
compile_error!("The `ping` feature requires Rust 1.70.0 or later");

#[cfg(all(feature = "ping6", not(version("1.70.0"))))]
compile_error!("The `ping6` feature requires Rust 1.70.0 or later");

fn main() {
    common::setup_logging(
        "net",
        "pci",
        "netutils",
        common::output_level(),
        common::file_level(),
    );
    redox_daemon::Daemon::new(daemon).expect("netutils: failed to daemonize");
}

fn daemon(_: redox_daemon::Daemon) -> ! {
    #[cfg(feature = "ping")]
    ping::main();

    #[cfg(feature = "ping6")]
    ping6::main();

    std::process::exit(0);
}

fn checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;
    while i < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum += u32::from(word);
        i += 2;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}

#[cfg(feature = "ping")]
mod ping {
    use std::env;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::net::Ipv4Addr;
    use std::time::Instant;
    use super::checksum;

    const ICMP_ECHO_REQUEST: u8 = 8;
    const ICMP_ECHO_REPLY: u8 = 0;
    const IPPROTO_ICMP: u8 = 1;

    #[repr(C, packed)]
    struct Ipv4Header {
        version_ihl: u8,
        dscp_ecn: u8,
        total_len: u16,
        ident: u16,
        flags_frag: u16,
        ttl: u8,
        proto: u8,
        checksum: u16,
        src_addr: [u8; 4],
        dst_addr: [u8; 4],
    }

    #[repr(C, packed)]
    struct IcmpHeader {
        msg_type: u8,
        code: u8,
        checksum: u16,
        ident: u16,
        seq_num: u16,
    }

    pub fn main() {
        let args: Vec<String> = env::args().collect();
        if args.len() != 2 {
            log::error!("Usage: ping <destination>");
            return;
        }

        let dest: Ipv4Addr = match args[1].parse() {
            Ok(addr) => addr,
            Err(_) => {
                log::error!("Invalid IPv4 address: {}", args[1]);
                return;
            }
        };

        let mut net_dev = match File::create("network:") {
            Ok(file) => file,
            Err(err) => {
                log::error!("Failed to open network device: {}", err);
                return;
            }
        };
        
        let mut src_addr_file = File::open("network:ip").unwrap();
        let mut src_addr_bytes = [0; 4];
        src_addr_file.read_exact(&mut src_addr_bytes).unwrap();
        let src_addr = Ipv4Addr::from(src_addr_bytes);

        let icmp_payload = b"Hello, world!";
        let mut icmp_header = IcmpHeader {
            msg_type: ICMP_ECHO_REQUEST,
            code: 0,
            checksum: 0,
            ident: 0,
            seq_num: 0,
        };

        let mut icmp_packet = Vec::new();
        icmp_packet.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &icmp_header as *const _ as *const u8,
                std::mem::size_of::<IcmpHeader>(),
            )
        });
        icmp_packet.extend_from_slice(icmp_payload);
        
        icmp_header.checksum = checksum(&icmp_packet);

        let ipv4_header = Ipv4Header {
            version_ihl: (4 << 4) | 5,
            dscp_ecn: 0,
            total_len: (std::mem::size_of::<Ipv4Header>() + icmp_packet.len()) as u16,
            ident: 0,
            flags_frag: 0,
            ttl: 64,
            proto: IPPROTO_ICMP,
            checksum: 0,
            src_addr: src_addr.octets(),
            dst_addr: dest.octets(),
        };
        
        let mut ipv4_header_bytes = Vec::new();
        ipv4_header_bytes.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &ipv4_header as *const _ as *const u8,
                std::mem::size_of::<Ipv4Header>(),
            )
        });
        
        let ipv4_checksum = checksum(&ipv4_header_bytes);
        
        let mut packet = Vec::new();
        packet.extend_from_slice(&ipv4_header_bytes[..10]);
        packet.extend_from_slice(&ipv4_checksum.to_be_bytes());
        packet.extend_from_slice(&ipv4_header_bytes[12..]);
        packet.extend_from_slice(&icmp_packet);
        
        let start_time = Instant::now();
        
        if let Err(err) = net_dev.write_all(&packet) {
            log::error!("Failed to send packet: {}", err);
            return;
        }

        let mut buffer = [0; 1500];
        loop {
            match net_dev.read(&mut buffer) {
                Ok(0) => {}
                Ok(count) => {
                    let elapsed = start_time.elapsed();
                    let ipv4_reply: &Ipv4Header = unsafe { &*(buffer.as_ptr() as *const Ipv4Header) };
                    let icmp_reply: &IcmpHeader = unsafe { &*((buffer.as_ptr() as usize + std::mem::size_of::<Ipv4Header>()) as *const IcmpHeader) };
                    if icmp_reply.msg_type == ICMP_ECHO_REPLY {
                        log::info!(
                            "{} bytes from {}: icmp_seq={} time={:?}",
                            count,
                            dest,
                            icmp_reply.seq_num,
                            elapsed
                        );
                        break;
                    }
                }
                Err(err) => {
                    log::error!("Failed to read from network device: {}", err);
                    break;
                }
            }
        }
    }
}

#[cfg(feature = "ping6")]
mod ping6 {
    use std::env;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::net::Ipv6Addr;
    use std::time::Instant;
    use super::checksum;

    const ICMPV6_ECHO_REQUEST: u8 = 128;
    const ICMPV6_ECHO_REPLY: u8 = 129;
    const IPPROTO_ICMPV6: u8 = 58;

    #[repr(C, packed)]
    struct Ipv6Header {
        version_tc_flow: u32,
        payload_len: u16,
        next_hdr: u8,
        hop_limit: u8,
        src_addr: [u8; 16],
        dst_addr: [u8; 16],
    }

    #[repr(C, packed)]
    struct Icmpv6Header {
        msg_type: u8,
        code: u8,
        checksum: u16,
        ident: u16,
        seq_num: u16,
    }

    pub fn main() {
        let args: Vec<String> = env::args().collect();
        if args.len() != 2 {
            log::error!("Usage: ping6 <destination>");
            return;
        }

        let dest: Ipv6Addr = match args[1].parse() {
            Ok(addr) => addr,
            Err(_) => {
                log::error!("Invalid IPv6 address: {}", args[1]);
                return;
            }
        };

        let mut net_dev = match File::create("network:") {
            Ok(file) => file,
            Err(err) => {
                log::error!("Failed to open network device: {}", err);
                return;
            }
        };
        
        let mut src_addr_file = File::open("network:ipv6").unwrap();
        let mut src_addr_bytes = [0; 16];
        src_addr_file.read_exact(&mut src_addr_bytes).unwrap();
        let src_addr = Ipv6Addr::from(src_addr_bytes);

        let icmp_payload = b"Hello, world!";
        let mut icmp_header = Icmpv6Header {
            msg_type: ICMPV6_ECHO_REQUEST,
            code: 0,
            checksum: 0,
            ident: 0,
            seq_num: 0,
        };

        let ipv6_header = Ipv6Header {
            version_tc_flow: (6 << 28),
            payload_len: (std::mem::size_of::<Icmpv6Header>() + icmp_payload.len()) as u16,
            next_hdr: IPPROTO_ICMPV6,
            hop_limit: 64,
            src_addr: src_addr.octets(),
            dst_addr: dest.octets(),
        };

        let mut pseudo_header = Vec::new();
        pseudo_header.extend_from_slice(&ipv6_header.src_addr);
        pseudo_header.extend_from_slice(&ipv6_header.dst_addr);
        pseudo_header.extend_from_slice(&ipv6_header.payload_len.to_be_bytes());
        pseudo_header.extend_from_slice(&[0, 0, 0, ipv6_header.next_hdr]);
        
        let mut icmp_packet = Vec::new();
        icmp_packet.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &icmp_header as *const _ as *const u8,
                std::mem::size_of::<Icmpv6Header>(),
            )
        });
        icmp_packet.extend_from_slice(icmp_payload);
        
        let mut checksum_data = Vec::new();
        checksum_data.extend_from_slice(&pseudo_header);
        checksum_data.extend_from_slice(&icmp_packet);
        
        icmp_header.checksum = checksum(&checksum_data);

        let mut packet = Vec::new();
        packet.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &ipv6_header as *const _ as *const u8,
                std::mem::size_of::<Ipv6Header>(),
            )
        });
        packet.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &icmp_header as *const _ as *const u8,
                std::mem::size_of::<Icmpv6Header>(),
            )
        });
        packet.extend_from_slice(icmp_payload);
        
        let start_time = Instant::now();
        
        if let Err(err) = net_dev.write_all(&packet) {
            log::error!("Failed to send packet: {}", err);
            return;
        }

        let mut buffer = [0; 1500];
        loop {
            match net_dev.read(&mut buffer) {
                Ok(0) => {}
                Ok(count) => {
                    let elapsed = start_time.elapsed();
                    let ipv6_reply: &Ipv6Header = unsafe { &*(buffer.as_ptr() as *const Ipv6Header) };
                    let icmp_reply: &Icmpv6Header = unsafe { &*((buffer.as_ptr() as usize + std::mem::size_of::<Ipv6Header>()) as *const Icmpv6Header) };
                    if icmp_reply.msg_type == ICMPV6_ECHO_REPLY {
                        log::info!(
                            "{} bytes from {}: icmp_seq={} time={:?}",
                            count,
                            dest,
                            icmp_reply.seq_num,
                            elapsed
                        );
                        break;
                    }
                }
                Err(err) => {
                    log::error!("Failed to read from network device: {}", err);
                    break;
                }
            }
        }
    }
}
