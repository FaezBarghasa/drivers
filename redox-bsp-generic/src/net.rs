//! Networking support for embedded profile
//!
//! Integrates with the Redox networking stack and BBRv3 congestion control.

use alloc::vec::Vec;

use crate::drivers::ethernet::{EthernetDriver, MacAddress};

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    /// Create a new IPv4 address
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Unspecified address (0.0.0.0)
    pub const UNSPECIFIED: Self = Self::new(0, 0, 0, 0);

    /// Loopback address (127.0.0.1)
    pub const LOOPBACK: Self = Self::new(127, 0, 0, 1);

    /// Broadcast address (255.255.255.255)
    pub const BROADCAST: Self = Self::new(255, 255, 255, 255);
}

impl core::fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// IP address
    pub ip_address: Ipv4Address,
    /// Subnet mask
    pub subnet_mask: Ipv4Address,
    /// Gateway address
    pub gateway: Ipv4Address,
    /// Primary DNS server
    pub dns_primary: Ipv4Address,
    /// Secondary DNS server
    pub dns_secondary: Ipv4Address,
    /// Use DHCP
    pub dhcp_enabled: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip_address: Ipv4Address::UNSPECIFIED,
            subnet_mask: Ipv4Address::new(255, 255, 255, 0),
            gateway: Ipv4Address::UNSPECIFIED,
            dns_primary: Ipv4Address::new(8, 8, 8, 8),
            dns_secondary: Ipv4Address::new(8, 8, 4, 4),
            dhcp_enabled: true,
        }
    }
}

/// Network interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// Interface is down
    Down,
    /// Interface is up but not configured
    Up,
    /// Interface is configured (has IP)
    Configured,
    /// DHCP in progress
    DhcpPending,
}

/// Network interface
pub struct NetworkInterface<E: EthernetDriver> {
    /// Underlying Ethernet driver
    driver: E,
    /// Network configuration
    config: NetworkConfig,
    /// Interface state
    state: InterfaceState,
    /// Interface name
    name: &'static str,
}

impl<E: EthernetDriver> NetworkInterface<E> {
    /// Create a new network interface
    pub fn new(driver: E, name: &'static str) -> Self {
        Self {
            driver,
            config: NetworkConfig::default(),
            state: InterfaceState::Down,
            name,
        }
    }

    /// Initialize the interface
    pub fn init(&mut self) -> Result<(), E::Error> {
        self.driver.init()?;
        self.state = if self.driver.is_link_up() {
            InterfaceState::Up
        } else {
            InterfaceState::Down
        };
        Ok(())
    }

    /// Get interface name
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Get MAC address
    pub fn mac_address(&self) -> MacAddress {
        self.driver.mac_address()
    }

    /// Get current configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    /// Set static configuration
    pub fn configure_static(&mut self, config: NetworkConfig) {
        self.config = config;
        self.config.dhcp_enabled = false;
        self.state = InterfaceState::Configured;
    }

    /// Start DHCP
    pub fn start_dhcp(&mut self) {
        self.config.dhcp_enabled = true;
        self.state = InterfaceState::DhcpPending;
        // DHCP implementation would go here
    }

    /// Get interface state
    pub fn state(&self) -> InterfaceState {
        self.state
    }

    /// Check if interface is configured
    pub fn is_configured(&self) -> bool {
        self.state == InterfaceState::Configured
    }

    /// Transmit a packet
    pub fn transmit(&mut self, data: &[u8]) -> Result<(), E::Error> {
        self.driver.transmit(data)
    }

    /// Receive a packet
    pub fn receive(&mut self) -> Result<Option<Vec<u8>>, E::Error> {
        self.driver.receive()
    }

    /// Poll for link status changes
    pub fn poll_link(&mut self) {
        let link_up = self.driver.is_link_up();
        match (self.state, link_up) {
            (InterfaceState::Down, true) => {
                self.state = InterfaceState::Up;
                if self.config.dhcp_enabled {
                    self.start_dhcp();
                }
            }
            (
                InterfaceState::Up | InterfaceState::Configured | InterfaceState::DhcpPending,
                false,
            ) => {
                self.state = InterfaceState::Down;
            }
            _ => {}
        }
    }
}

/// Simple ARP cache
pub struct ArpCache {
    entries: [(Ipv4Address, MacAddress, u32); 16],
    count: usize,
}

impl ArpCache {
    /// Create a new ARP cache
    pub const fn new() -> Self {
        Self {
            entries: [(Ipv4Address::UNSPECIFIED, MacAddress([0; 6]), 0); 16],
            count: 0,
        }
    }

    /// Lookup a MAC address
    pub fn lookup(&self, ip: Ipv4Address) -> Option<MacAddress> {
        for i in 0..self.count {
            if self.entries[i].0 == ip {
                return Some(self.entries[i].1);
            }
        }
        None
    }

    /// Insert an entry
    pub fn insert(&mut self, ip: Ipv4Address, mac: MacAddress, timestamp: u32) {
        // Check for existing entry
        for i in 0..self.count {
            if self.entries[i].0 == ip {
                self.entries[i].1 = mac;
                self.entries[i].2 = timestamp;
                return;
            }
        }

        // Add new entry
        if self.count < self.entries.len() {
            self.entries[self.count] = (ip, mac, timestamp);
            self.count += 1;
        } else {
            // Replace oldest entry
            let mut oldest = 0;
            for i in 1..self.count {
                if self.entries[i].2 < self.entries[oldest].2 {
                    oldest = i;
                }
            }
            self.entries[oldest] = (ip, mac, timestamp);
        }
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.count = 0;
    }
}

impl Default for ArpCache {
    fn default() -> Self {
        Self::new()
    }
}

/// TCP/UDP port
pub type Port = u16;

/// Socket address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddr {
    pub ip: Ipv4Address,
    pub port: Port,
}

impl SocketAddr {
    pub const fn new(ip: Ipv4Address, port: Port) -> Self {
        Self { ip, port }
    }
}

/// Calculate IP checksum
pub fn ip_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    for chunk in data.chunks(2) {
        let word = if chunk.len() == 2 {
            ((chunk[0] as u32) << 8) | (chunk[1] as u32)
        } else {
            (chunk[0] as u32) << 8
        };
        sum += word;
    }

    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}
