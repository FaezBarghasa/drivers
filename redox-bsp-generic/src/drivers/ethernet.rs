//! Ethernet driver for embedded platforms
//!
//! Provides a basic Ethernet MAC interface for networking.

use alloc::vec::Vec;

/// Ethernet MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// Create a new MAC address
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Check if this is a broadcast address
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    }

    /// Check if this is a multicast address
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// Link status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    /// Link is down
    Down,
    /// Link is up at 10 Mbps half duplex
    Up10HalfDuplex,
    /// Link is up at 10 Mbps full duplex
    Up10FullDuplex,
    /// Link is up at 100 Mbps half duplex
    Up100HalfDuplex,
    /// Link is up at 100 Mbps full duplex
    Up100FullDuplex,
    /// Link is up at 1000 Mbps full duplex
    Up1000FullDuplex,
}

impl LinkStatus {
    /// Check if link is up
    pub fn is_up(&self) -> bool {
        !matches!(self, LinkStatus::Down)
    }

    /// Get speed in Mbps
    pub fn speed_mbps(&self) -> u32 {
        match self {
            LinkStatus::Down => 0,
            LinkStatus::Up10HalfDuplex | LinkStatus::Up10FullDuplex => 10,
            LinkStatus::Up100HalfDuplex | LinkStatus::Up100FullDuplex => 100,
            LinkStatus::Up1000FullDuplex => 1000,
        }
    }
}

/// Ethernet driver trait
pub trait EthernetDriver {
    /// Error type
    type Error;

    /// Initialize the driver
    fn init(&mut self) -> Result<(), Self::Error>;

    /// Get MAC address
    fn mac_address(&self) -> MacAddress;

    /// Set MAC address
    fn set_mac_address(&mut self, mac: MacAddress) -> Result<(), Self::Error>;

    /// Get link status
    fn link_status(&self) -> LinkStatus;

    /// Check if link is up
    fn is_link_up(&self) -> bool {
        self.link_status().is_up()
    }

    /// Transmit a packet
    fn transmit(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Receive a packet (returns None if no packet available)
    fn receive(&mut self) -> Result<Option<Vec<u8>>, Self::Error>;

    /// Enable interrupts
    fn enable_interrupts(&mut self);

    /// Disable interrupts
    fn disable_interrupts(&mut self);

    /// Handle interrupt
    fn handle_interrupt(&mut self);

    /// Get statistics
    fn statistics(&self) -> EthernetStats;
}

/// Ethernet statistics
#[derive(Debug, Clone, Default)]
pub struct EthernetStats {
    /// Packets transmitted
    pub tx_packets: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Transmit errors
    pub tx_errors: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Bytes received
    pub rx_bytes: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Packets dropped
    pub rx_dropped: u64,
    /// CRC errors
    pub crc_errors: u64,
    /// Collisions
    pub collisions: u64,
}

/// Generic MDIO (Management Data I/O) interface
pub trait MdioInterface {
    /// Read a PHY register
    fn read(&self, phy_addr: u8, reg_addr: u8) -> u16;

    /// Write a PHY register
    fn write(&self, phy_addr: u8, reg_addr: u8, value: u16);
}

/// PHY (Physical Layer) driver
pub struct PhyDriver<M: MdioInterface> {
    mdio: M,
    phy_addr: u8,
}

impl<M: MdioInterface> PhyDriver<M> {
    /// PHY register addresses
    pub const REG_BMCR: u8 = 0; // Basic Mode Control
    pub const REG_BMSR: u8 = 1; // Basic Mode Status
    pub const REG_PHYID1: u8 = 2; // PHY ID 1
    pub const REG_PHYID2: u8 = 3; // PHY ID 2
    pub const REG_ANAR: u8 = 4; // Auto-Neg Advertisement
    pub const REG_ANLPAR: u8 = 5; // Auto-Neg Link Partner Ability

    /// Create a new PHY driver
    pub fn new(mdio: M, phy_addr: u8) -> Self {
        Self { mdio, phy_addr }
    }

    /// Read PHY ID
    pub fn read_id(&self) -> u32 {
        let id1 = self.mdio.read(self.phy_addr, Self::REG_PHYID1) as u32;
        let id2 = self.mdio.read(self.phy_addr, Self::REG_PHYID2) as u32;
        (id1 << 16) | id2
    }

    /// Reset the PHY
    pub fn reset(&self) {
        let bmcr = self.mdio.read(self.phy_addr, Self::REG_BMCR);
        self.mdio
            .write(self.phy_addr, Self::REG_BMCR, bmcr | 0x8000);

        // Wait for reset to complete
        while self.mdio.read(self.phy_addr, Self::REG_BMCR) & 0x8000 != 0 {
            core::hint::spin_loop();
        }
    }

    /// Start auto-negotiation
    pub fn start_autoneg(&self) {
        let bmcr = self.mdio.read(self.phy_addr, Self::REG_BMCR);
        self.mdio
            .write(self.phy_addr, Self::REG_BMCR, bmcr | 0x1200); // Enable ANEN + restart
    }

    /// Check if link is up
    pub fn is_link_up(&self) -> bool {
        let bmsr = self.mdio.read(self.phy_addr, Self::REG_BMSR);
        bmsr & 0x0004 != 0 // Link status bit
    }

    /// Get link speed/duplex
    pub fn link_status(&self) -> LinkStatus {
        if !self.is_link_up() {
            return LinkStatus::Down;
        }

        let anlpar = self.mdio.read(self.phy_addr, Self::REG_ANLPAR);

        if anlpar & 0x0100 != 0 {
            LinkStatus::Up100FullDuplex
        } else if anlpar & 0x0080 != 0 {
            LinkStatus::Up100HalfDuplex
        } else if anlpar & 0x0040 != 0 {
            LinkStatus::Up10FullDuplex
        } else {
            LinkStatus::Up10HalfDuplex
        }
    }
}

/// Ethernet frame header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EthernetHeader {
    /// Destination MAC address
    pub dst_mac: [u8; 6],
    /// Source MAC address
    pub src_mac: [u8; 6],
    /// EtherType
    pub ethertype: [u8; 2],
}

impl EthernetHeader {
    /// IPv4 EtherType
    pub const ETHERTYPE_IPV4: u16 = 0x0800;
    /// IPv6 EtherType
    pub const ETHERTYPE_IPV6: u16 = 0x86DD;
    /// ARP EtherType
    pub const ETHERTYPE_ARP: u16 = 0x0806;
    /// VLAN EtherType
    pub const ETHERTYPE_VLAN: u16 = 0x8100;

    /// Get EtherType as u16
    pub fn ethertype_u16(&self) -> u16 {
        u16::from_be_bytes(self.ethertype)
    }
}
