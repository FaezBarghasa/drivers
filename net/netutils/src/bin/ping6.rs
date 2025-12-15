//! ping6 - ICMPv6 Echo Request utility for IPv6 connectivity testing
//!
//! This utility sends ICMPv6 Echo Request packets to test IPv6 connectivity,
//! measure round-trip time, and verify network reachability.

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::Ipv6Addr;
use std::process;
use std::time::{Duration, Instant};

/// ICMPv6 message types
const ICMPV6_ECHO_REQUEST: u8 = 128;
const ICMPV6_ECHO_REPLY: u8 = 129;

/// Default ping parameters
const DEFAULT_COUNT: usize = 4;
const DEFAULT_INTERVAL_MS: u64 = 1000;
const DEFAULT_TIMEOUT_MS: u64 = 5000;
const DEFAULT_PACKET_SIZE: usize = 56; // 56 bytes of data + 8 bytes ICMPv6 header = 64 bytes

/// ICMPv6 Echo Request/Reply header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Icmpv6EchoHeader {
    type_: u8,
    code: u8,
    checksum: u16,
    identifier: u16,
    sequence: u16,
}

impl Icmpv6EchoHeader {
    fn new(type_: u8, identifier: u16, sequence: u16) -> Self {
        Self {
            type_,
            code: 0,
            checksum: 0,
            identifier,
            sequence,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of::<Self>())
        }
    }
}

/// IPv6 pseudo-header for checksum calculation
#[repr(C, packed)]
struct Ipv6PseudoHeader {
    src_addr: [u8; 16],
    dst_addr: [u8; 16],
    upper_layer_packet_length: u32,
    zeros: [u8; 3],
    next_header: u8,
}

/// Calculate ICMPv6 checksum
fn calculate_icmpv6_checksum(src_addr: &[u8; 16], dst_addr: &[u8; 16], icmp_packet: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    // Pseudo-header
    for i in (0..16).step_by(2) {
        sum += u16::from_be_bytes([src_addr[i], src_addr[i + 1]]) as u32;
        sum += u16::from_be_bytes([dst_addr[i], dst_addr[i + 1]]) as u32;
    }

    // Upper-layer packet length
    let length = icmp_packet.len() as u32;
    sum += (length >> 16) as u32;
    sum += (length & 0xFFFF) as u32;

    // Next header (ICMPv6 = 58)
    sum += 58;

    // ICMPv6 packet
    let mut i = 0;
    while i < icmp_packet.len() {
        if i + 1 < icmp_packet.len() {
            sum += u16::from_be_bytes([icmp_packet[i], icmp_packet[i + 1]]) as u32;
        } else {
            sum += (icmp_packet[i] as u32) << 8;
        }
        i += 2;
    }

    // Fold 32-bit sum to 16 bits
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !sum as u16
}

/// Ping statistics
#[derive(Default)]
struct PingStats {
    transmitted: usize,
    received: usize,
    min_rtt_ms: f64,
    max_rtt_ms: f64,
    total_rtt_ms: f64,
}

impl PingStats {
    fn update(&mut self, rtt_ms: f64) {
        self.received += 1;
        self.total_rtt_ms += rtt_ms;

        if self.received == 1 {
            self.min_rtt_ms = rtt_ms;
            self.max_rtt_ms = rtt_ms;
        } else {
            if rtt_ms < self.min_rtt_ms {
                self.min_rtt_ms = rtt_ms;
            }
            if rtt_ms > self.max_rtt_ms {
                self.max_rtt_ms = rtt_ms;
            }
        }
    }

    fn avg_rtt_ms(&self) -> f64 {
        if self.received > 0 {
            self.total_rtt_ms / self.received as f64
        } else {
            0.0
        }
    }

    fn packet_loss_pct(&self) -> f64 {
        if self.transmitted > 0 {
            ((self.transmitted - self.received) as f64 / self.transmitted as f64) * 100.0
        } else {
            0.0
        }
    }

    fn print_summary(&self, host: &str) {
        println!("\n--- {} ping statistics ---", host);
        println!(
            "{} packets transmitted, {} received, {:.1}% packet loss",
            self.transmitted,
            self.received,
            self.packet_loss_pct()
        );

        if self.received > 0 {
            println!(
                "rtt min/avg/max = {:.3}/{:.3}/{:.3} ms",
                self.min_rtt_ms,
                self.avg_rtt_ms(),
                self.max_rtt_ms
            );
        }
    }
}

/// Ping configuration
struct PingConfig {
    target: Ipv6Addr,
    count: usize,
    interval_ms: u64,
    timeout_ms: u64,
    packet_size: usize,
    interface: Option<String>,
}

impl PingConfig {
    fn from_args() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();

        if args.len() < 2 {
            return Err(format!("Usage: {} <IPv6 address> [options]", args[0]));
        }

        let target = args[1]
            .parse::<Ipv6Addr>()
            .map_err(|e| format!("Invalid IPv6 address: {}", e))?;

        let mut config = PingConfig {
            target,
            count: DEFAULT_COUNT,
            interval_ms: DEFAULT_INTERVAL_MS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            packet_size: DEFAULT_PACKET_SIZE,
            interface: None,
        };

        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "-c" => {
                    i += 1;
                    if i < args.len() {
                        config.count = args[i]
                            .parse()
                            .map_err(|_| "Invalid count value".to_string())?;
                    }
                }
                "-i" => {
                    i += 1;
                    if i < args.len() {
                        let interval: f64 = args[i]
                            .parse()
                            .map_err(|_| "Invalid interval value".to_string())?;
                        config.interval_ms = (interval * 1000.0) as u64;
                    }
                }
                "-W" => {
                    i += 1;
                    if i < args.len() {
                        config.timeout_ms = args[i]
                            .parse::<u64>()
                            .map_err(|_| "Invalid timeout value".to_string())?
                            * 1000;
                    }
                }
                "-s" => {
                    i += 1;
                    if i < args.len() {
                        config.packet_size = args[i]
                            .parse()
                            .map_err(|_| "Invalid packet size".to_string())?;
                    }
                }
                "-I" => {
                    i += 1;
                    if i < args.len() {
                        config.interface = Some(args[i].clone());
                    }
                }
                _ => return Err(format!("Unknown option: {}", args[i])),
            }
            i += 1;
        }

        Ok(config)
    }
}

/// Send ICMPv6 Echo Request and wait for reply
fn ping_once(
    network: &mut File,
    src_addr: &[u8; 16],
    dst_addr: &[u8; 16],
    identifier: u16,
    sequence: u16,
    packet_size: usize,
    timeout: Duration,
) -> Result<Duration, String> {
    // Build ICMPv6 Echo Request packet
    let mut packet = Vec::with_capacity(8 + packet_size);

    let mut header = Icmpv6EchoHeader::new(ICMPV6_ECHO_REQUEST, identifier, sequence);

    // Add header (temporarily with zero checksum)
    packet.extend_from_slice(header.as_bytes());

    // Add data payload (pattern: incrementing bytes)
    for i in 0..packet_size {
        packet.push((i % 256) as u8);
    }

    // Calculate and set checksum
    let checksum = calculate_icmpv6_checksum(src_addr, dst_addr, &packet);
    header.checksum = checksum.to_be();

    // Update packet with correct checksum
    packet[2..4].copy_from_slice(&header.checksum.to_be_bytes());

    // Build IPv6 packet (simplified - in real implementation this would be done by the network stack)
    let mut ipv6_packet = Vec::with_capacity(40 + packet.len());

    // IPv6 header (40 bytes)
    ipv6_packet.push(0x60); // Version (6) and Traffic Class (0)
    ipv6_packet.push(0x00); // Traffic Class and Flow Label
    ipv6_packet.push(0x00); // Flow Label
    ipv6_packet.push(0x00); // Flow Label

    // Payload length
    let payload_len = packet.len() as u16;
    ipv6_packet.extend_from_slice(&payload_len.to_be_bytes());

    // Next header (ICMPv6 = 58)
    ipv6_packet.push(58);

    // Hop limit
    ipv6_packet.push(64);

    // Source address
    ipv6_packet.extend_from_slice(src_addr);

    // Destination address
    ipv6_packet.extend_from_slice(dst_addr);

    // ICMPv6 payload
    ipv6_packet.extend_from_slice(&packet);

    // Send packet
    let start = Instant::now();
    network
        .write_all(&ipv6_packet)
        .map_err(|e| format!("Failed to send packet: {}", e))?;

    // Wait for reply
    let deadline = start + timeout;
    let mut buf = vec![0u8; 2048];

    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err("Timeout".to_string());
        }

        // Try to read reply (non-blocking)
        match network.read(&mut buf) {
            Ok(n) if n > 0 => {
                // Parse IPv6 header (skip 40 bytes)
                if n < 40 + 8 {
                    continue;
                }

                let icmp_start = 40;
                let icmp_type = buf[icmp_start];

                if icmp_type == ICMPV6_ECHO_REPLY {
                    let reply_id = u16::from_be_bytes([buf[icmp_start + 4], buf[icmp_start + 5]]);
                    let reply_seq = u16::from_be_bytes([buf[icmp_start + 6], buf[icmp_start + 7]]);

                    if reply_id == identifier && reply_seq == sequence {
                        let rtt = start.elapsed();
                        return Ok(rtt);
                    }
                }
            }
            Ok(_) => {
                // No data, sleep briefly and retry
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(format!("Read error: {}", e)),
        }
    }
}

fn main() {
    let config = match PingConfig::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nUsage: ping6 <IPv6 address> [options]");
            eprintln!("Options:");
            eprintln!(
                "  -c <count>      Number of packets to send (default: {})",
                DEFAULT_COUNT
            );
            eprintln!("  -i <interval>   Interval between packets in seconds (default: 1.0)");
            eprintln!("  -W <timeout>    Timeout in seconds (default: 5)");
            eprintln!(
                "  -s <size>       Packet data size in bytes (default: {})",
                DEFAULT_PACKET_SIZE
            );
            eprintln!("  -I <interface>  Network interface to use");
            process::exit(1);
        }
    };

    // Open network interface
    let network_path = if let Some(ref iface) = config.interface {
        format!("network:{}", iface)
    } else {
        "network:".to_string()
    };

    let mut network = match File::options().read(true).write(true).open(&network_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open network interface: {}", e);
            process::exit(1);
        }
    };

    // Get source IPv6 address (simplified - would query from interface)
    let src_addr = [
        0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0x02, 0x00, 0x00, 0xff, 0xfe, 0x00, 0x00, 0x01,
    ];
    let dst_addr = config.target.octets();

    println!("PING {} {} data bytes", config.target, config.packet_size);

    let identifier = process::id() as u16;
    let mut stats = PingStats::default();

    for seq in 0..config.count {
        stats.transmitted += 1;

        match ping_once(
            &mut network,
            &src_addr,
            &dst_addr,
            identifier,
            seq as u16,
            config.packet_size,
            Duration::from_millis(config.timeout_ms),
        ) {
            Ok(rtt) => {
                let rtt_ms = rtt.as_secs_f64() * 1000.0;
                stats.update(rtt_ms);

                println!(
                    "{} bytes from {}: icmp_seq={} ttl=64 time={:.3} ms",
                    config.packet_size + 8,
                    config.target,
                    seq,
                    rtt_ms
                );
            }
            Err(e) => {
                println!("From {}: {}", config.target, e);
            }
        }

        // Sleep between packets (except after last one)
        if seq < config.count - 1 {
            std::thread::sleep(Duration::from_millis(config.interval_ms));
        }
    }

    stats.print_summary(&config.target.to_string());

    // Exit with error code if packet loss occurred
    if stats.received < stats.transmitted {
        process::exit(1);
    }
}
