use std::convert::TryInto;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::{cmp, mem, ptr, slice, thread};

use driver_network::NetworkAdapter;
use syscall::error::Result;

use common::dma::Dma;

use crate::ixgbe::*;

pub struct Intel8259x {
    base: usize,
    size: usize,
    receive_buffers: Vec<[Dma<[u8; 16384]>; 32]>,
    receive_rings: Vec<Dma<[ixgbe_adv_rx_desc; 32]>>,
    receive_indexes: Vec<usize>,
    current_rx_queue: usize,
    transmit_buffer: [Dma<[u8; 16384]>; 32],
    transmit_ring: Dma<[ixgbe_adv_tx_desc; 32]>,
    transmit_ring_free: usize,
    transmit_index: usize,
    transmit_clean_index: usize,
    mac_address: [u8; 6],
    ipv4_address: [u8; 4],
    /// Track bytes in flight for BBRv3
    in_flight_bytes: AtomicU64,
    /// RDMA Ring Memory
    ring_mem: *mut crate::ring_defs::IpcRing,
}

fn wrap_ring(index: usize, ring_size: usize) -> usize {
    (index + 1) & (ring_size - 1)
}

impl NetworkAdapter for Intel8259x {
    fn mac_address(&mut self) -> [u8; 6] {
        self.mac_address
    }

    fn ipv4_address(&mut self) -> [u8; 4] {
        self.ipv4_address
    }

    fn ipv6_address(&mut self) -> [u8; 16] {
        // Generate link-local IPv6 address from MAC (EUI-64)
        let mac = self.mac_address;
        [
            0xfe,
            0x80,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            mac[0] ^ 0x02,
            mac[1],
            mac[2],
            0xff,
            0xfe,
            mac[3],
            mac[4],
            mac[5],
        ]
    }

    fn ipv6_address_global(&mut self) -> [u8; 16] {
        [0; 16] // No global IPv6 address configured
    }

    fn ipv6_address_unique_local(&mut self) -> [u8; 16] {
        [0; 16] // No unique local IPv6 address configured
    }

    fn available_for_read(&mut self) -> usize {
        self.next_read()
    }

    fn read_packet(&mut self, buf: &mut [u8]) -> Result<Option<usize>> {
        // Try all queues starting from current
        for _ in 0..self.receive_rings.len() {
            let q_idx = self.current_rx_queue;
            let idx = self.receive_indexes[q_idx];
            
            let desc = unsafe {
                &mut *(self.receive_rings[q_idx].as_ptr().add(idx) as *mut ixgbe_adv_rx_desc)
            };
    
            let status = unsafe { desc.wb.upper.status_error };
    
            if (status & IXGBE_RXDADV_STAT_DD) != 0 {
                if (status & IXGBE_RXDADV_STAT_EOP) == 0 {
                    panic!("increase buffer size or decrease MTU")
                }
    
                let data = unsafe {
                    &self.receive_buffers[q_idx][idx][..desc.wb.upper.length as usize]
                };
    
                let i = cmp::min(buf.len(), data.len());
                buf[..i].copy_from_slice(&data[..i]);
    
                // Track bytes acknowledged
                self.in_flight_bytes.fetch_sub(i as u64, Ordering::SeqCst);
    
                desc.read.pkt_addr = self.receive_buffers[q_idx][idx].physical() as u64;
                desc.read.hdr_addr = 0;
    
                self.write_reg(IXGBE_RDT(q_idx as u32), idx as u32);
                self.receive_indexes[q_idx] = wrap_ring(idx, self.receive_rings[q_idx].len());
                
                // Advance queue for next read to be round-robin
                self.current_rx_queue = (self.current_rx_queue + 1) % self.receive_rings.len();
    
                return Ok(Some(i));
            }
            
            self.current_rx_queue = (self.current_rx_queue + 1) % self.receive_rings.len();
        }

        Ok(None)
    }

    fn write_packet(&mut self, buf: &[u8], _pacing_rate: u64) -> Result<usize> {
        if self.transmit_ring_free == 0 {
            loop {
                let desc = unsafe {
                    &*(self.transmit_ring.as_ptr().add(self.transmit_clean_index)
                        as *const ixgbe_adv_tx_desc)
                };

                if (unsafe { desc.wb.status } & IXGBE_ADVTXD_STAT_DD) != 0 {
                    self.transmit_clean_index =
                        wrap_ring(self.transmit_clean_index, self.transmit_ring.len());
                    self.transmit_ring_free += 1;
                } else if self.transmit_ring_free > 0 {
                    break;
                }

                if self.transmit_ring_free >= self.transmit_ring.len() {
                    break;
                }
            }
        }

        let desc = unsafe {
            &mut *(self.transmit_ring.as_ptr().add(self.transmit_index) as *mut ixgbe_adv_tx_desc)
        };

        let data = unsafe {
            slice::from_raw_parts_mut(
                self.transmit_buffer[self.transmit_index].as_ptr() as *mut u8,
                cmp::min(buf.len(), self.transmit_buffer[self.transmit_index].len()) as usize,
            )
        };

        let i = cmp::min(buf.len(), data.len());
        data[..i].copy_from_slice(&buf[..i]);

        desc.read.cmd_type_len = IXGBE_ADVTXD_DCMD_EOP
            | IXGBE_ADVTXD_DCMD_RS
            | IXGBE_ADVTXD_DCMD_IFCS
            | IXGBE_ADVTXD_DCMD_DEXT
            | IXGBE_ADVTXD_DTYP_DATA
            | buf.len() as u32;

        desc.read.olinfo_status = (buf.len() as u32) << IXGBE_ADVTXD_PAYLEN_SHIFT;

        // Track bytes in flight
        self.in_flight_bytes.fetch_add(i as u64, Ordering::SeqCst);

        self.transmit_index = wrap_ring(self.transmit_index, self.transmit_ring.len());
        self.transmit_ring_free -= 1;

        self.write_reg(IXGBE_TDT(0), self.transmit_index as u32);

        Ok(i)
    }

    fn in_flight(&self) -> u64 {
        self.in_flight_bytes.load(Ordering::SeqCst)
    }
}

impl Intel8259x {
    /// Returns an initialized `Intel8259x` on success.
    pub fn new(base: usize, size: usize, ring_mem: *mut crate::ring_defs::IpcRing) -> Result<Self> {
        let num_queues = 4;
        
        let mut receive_buffers = Vec::new();
        let mut receive_rings = Vec::new();
        let mut receive_indexes = Vec::new();

        for _ in 0..num_queues {
            receive_buffers.push(
                (0..32)
                .map(|_| Ok(unsafe { Dma::zeroed()?.assume_init() }))
                .collect::<Result<Vec<_>>>()?
                .try_into()
                .unwrap_or_else(|_| unreachable!())
            );
            receive_rings.push(unsafe { Dma::zeroed()?.assume_init() });
            receive_indexes.push(0);
        }

        #[rustfmt::skip]
        let mut module = Intel8259x {
            base,
            size,
            receive_buffers,
            receive_rings,
            receive_indexes,
            current_rx_queue: 0,
            
            transmit_buffer: (0..32)
                .map(|_| Ok(unsafe { Dma::zeroed()?.assume_init() }))
                .collect::<Result<Vec<_>>>()?
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
            transmit_ring: unsafe { Dma::zeroed()?.assume_init() },
            transmit_ring_free: 32,
            transmit_index: 0,
            transmit_clean_index: 0,
            mac_address: [0; 6],
            ipv4_address: [10, 0, 2, 15], // Default QEMU/DHCP address
            in_flight_bytes: AtomicU64::new(0),
            ring_mem,
        };

        module.init();

        Ok(module)
    }

    pub fn process_ring(&mut self) {
        let ring = unsafe { &mut *self.ring_mem };
        let tail = ring.sq_tail.load(Ordering::Acquire);
        let mut head = ring.sq_head.load(Ordering::Acquire);

        while head != tail {
            let idx = (head & ring.sq_mask) as usize;
            let sqe_ptr =
                unsafe { (self.ring_mem.add(1) as *const crate::ring_defs::Sqe).add(idx) };
            let sqe = unsafe { *sqe_ptr };

            println!(
                "IXGBE: Processing SQE opcode={} addr={:#x} len={}",
                sqe.opcode, sqe.addr, sqe.len
            );

            match sqe.opcode {
                crate::ring_defs::IORING_OP_RDMA_READ => {
                    println!("IXGBE: RDMA READ request");
                }
                crate::ring_defs::IORING_OP_RDMA_WRITE => {
                    println!("IXGBE: RDMA WRITE request");
                }
                _ => {
                    println!("IXGBE: Unknown opcode {}", sqe.opcode);
                }
            }

            head = head.wrapping_add(1);
        }

        ring.sq_head.store(head, Ordering::Release);
    }

    pub fn irq(&self) -> bool {
        let icr = self.read_reg(IXGBE_EICR);
        icr != 0
    }

    pub fn next_read(&mut self) -> usize {
        for _ in 0..self.receive_rings.len() {
            let q_idx = self.current_rx_queue;
            let idx = self.receive_indexes[q_idx];
            let desc = unsafe {
                &*(self.receive_rings[q_idx].as_ptr().add(idx) as *const ixgbe_adv_rx_desc)
            };

            let status = unsafe { desc.wb.upper.status_error };

            if (status & IXGBE_RXDADV_STAT_DD) != 0 {
                if (status & IXGBE_RXDADV_STAT_EOP) == 0 {
                    panic!("increase buffer size or decrease MTU")
                }

                return unsafe { desc.wb.upper.length as usize };
            }
            
            self.current_rx_queue = (self.current_rx_queue + 1) % self.receive_rings.len();
        }

        0
    }

    /// Returns the mac address of this device.
    pub fn get_mac_addr(&self) -> [u8; 6] {
        let low = self.read_reg(IXGBE_RAL(0));
        let high = self.read_reg(IXGBE_RAH(0));

        [
            (low & 0xff) as u8,
            (low >> 8 & 0xff) as u8,
            (low >> 16 & 0xff) as u8,
            (low >> 24) as u8,
            (high & 0xff) as u8,
            (high >> 8 & 0xff) as u8,
        ]
    }

    /// Sets the mac address of this device.
    #[allow(dead_code)]
    pub fn set_mac_addr(&mut self, mac: [u8; 6]) {
        let low: u32 = u32::from(mac[0])
            + (u32::from(mac[1]) << 8)
            + (u32::from(mac[2]) << 16)
            + (u32::from(mac[3]) << 24);
        let high: u32 = u32::from(mac[4]) + (u32::from(mac[5]) << 8);

        self.write_reg(IXGBE_RAL(0), low);
        self.write_reg(IXGBE_RAH(0), high);

        self.mac_address = mac;
    }

    /// Returns the register at `self.base` + `register`.
    ///
    /// # Panics
    ///
    /// Panics if `self.base` + `register` does not belong to the mapped memory of the PCIe device.
    fn read_reg(&self, register: u32) -> u32 {
        assert!(
            register as usize <= self.size - 4 as usize,
            "MMIO access out of bounds"
        );

        unsafe { ptr::read_volatile((self.base + register as usize) as *mut u32) }
    }

    /// Sets the register at `self.base` + `register`.
    ///
    /// # Panics
    ///
    /// Panics if `self.base` + `register` does not belong to the mapped memory of the PCIe device.
    fn write_reg(&self, register: u32, data: u32) -> u32 {
        assert!(
            register as usize <= self.size - 4 as usize,
            "MMIO access out of bounds"
        );

        unsafe {
            ptr::write_volatile((self.base + register as usize) as *mut u32, data);
            ptr::read_volatile((self.base + register as usize) as *mut u32)
        }
    }

    fn write_flag(&self, register: u32, flags: u32) {
        self.write_reg(register, self.read_reg(register) | flags);
    }

    fn clear_flag(&self, register: u32, flags: u32) {
        self.write_reg(register, self.read_reg(register) & !flags);
    }

    fn wait_clear_reg(&self, register: u32, value: u32) {
        loop {
            let current = self.read_reg(register);
            if (current & value) == 0 {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn wait_write_reg(&self, register: u32, value: u32) {
        loop {
            let current = self.read_reg(register);
            if (current & value) == value {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Resets and initializes an ixgbe device.
    fn init(&mut self) {
        // section 4.6.3.1 - disable all interrupts
        self.write_reg(IXGBE_EIMC, 0x7fff_ffff);

        // section 4.6.3.2
        self.write_reg(IXGBE_CTRL, IXGBE_CTRL_RST_MASK);
        self.wait_clear_reg(IXGBE_CTRL, IXGBE_CTRL_RST_MASK);
        thread::sleep(Duration::from_millis(10));

        // section 4.6.3.1 - disable interrupts again after reset
        self.write_reg(IXGBE_EIMC, 0x7fff_ffff);

        let mac = self.get_mac_addr();

        println!(
            "   - MAC: {:>02X}:{:>02X}:{:>02X}:{:>02X}:{:>02X}:{:>02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );

        self.mac_address = mac;

        // section 4.6.3 - wait for EEPROM auto read completion
        self.wait_write_reg(IXGBE_EEC, IXGBE_EEC_ARD);

        // section 4.6.3 - wait for dma initialization done
        self.wait_write_reg(
            IXGBE_RDRXCTL,
            IXGBE_RDRXCTL_DMAIDONE | IXGBE_RDRXCTL_RESERVED_BITS,
        );

        // section 4.6.4 - initialize link (auto negotiation)
        self.init_link();

        // section 4.6.5 - statistical counters
        // reset-on-read registers, just read them once
        self.reset_stats();

        // section 4.6.7 - init rx
        self.init_rx();

        // section 4.6.8 - init tx
        self.init_tx();

        // start rx queues (RSS enabled, 4 queues)
        for i in 0..4 {
            self.start_rx_queue(i);
        }

        // start a single transmit queue/ring
        self.start_tx_queue(0);

        // section 4.6.3.9 - enable interrupts
        self.enable_msix_interrupt(0);

        // wait some time for the link to come up
        self.wait_for_link();
    }

    /// Resets the stats of this device.
    fn reset_stats(&self) {
        self.read_reg(IXGBE_GPRC);
        self.read_reg(IXGBE_GPTC);
        self.read_reg(IXGBE_GORCL);
        self.read_reg(IXGBE_GORCH);
        self.read_reg(IXGBE_GOTCL);
        self.read_reg(IXGBE_GOTCH);
    }

    // sections 4.6.7
    /// Initializes the rx queues of this device.
    // sections 4.6.7
    /// Initializes the rx queues of this device.
    fn init_rx(&mut self) {
        // disable rx while re-configuring it
        self.clear_flag(IXGBE_RXCTRL, IXGBE_RXCTRL_RXEN);

        // section 4.6.11.3.4 - allocate all queues and traffic to PB0
        self.write_reg(IXGBE_RXPBSIZE(0), IXGBE_RXPBSIZE_128KB);
        for i in 1..8 {
            self.write_reg(IXGBE_RXPBSIZE(i), 0);
        }

        // enable CRC offloading
        self.write_flag(IXGBE_HLREG0, IXGBE_HLREG0_RXCRCSTRP);
        self.write_flag(IXGBE_RDRXCTL, IXGBE_RDRXCTL_CRCSTRIP);

        // accept broadcast packets
        self.write_flag(IXGBE_FCTRL, IXGBE_FCTRL_BAM);

        // Configure RSS
        // 1. Redirect Table (RETA)
        // We have 4 queues. Map hash-to-queue uniformly.
        // 0..31 -> Queue 0
        // 32..63 -> Queue 1
        // 64..95 -> Queue 2
        // 96..127 -> Queue 3
        for i in 0..128 {
            let q_idx = (i / 32) & 0x3; // 0, 1, 2, 3
            // RETA registers pack 4 entries per register (8 bits each)
            // But here we access via IXGBE_RETA(i) which seems to imply index 0..31 of registers?
            // Wait, IXGBE_RETA(i) in ixgbe.rs takes index 0..31. 
            // 32 registers * 4 entries = 128 entries.
            // So we need to write to registers.
            // Let's iterate registers 0..32
        }
        
        for i in 0..32 {
            // Each register holds 4 entries.
            // Entry j (0..3) in register i corresponds to global entry 4*i + j
            let q_idx0 = ((4 * i + 0) / 32) & 0x3;
            let q_idx1 = ((4 * i + 1) / 32) & 0x3;
            let q_idx2 = ((4 * i + 2) / 32) & 0x3;
            let q_idx3 = ((4 * i + 3) / 32) & 0x3;
            
            let reta_val = q_idx0 | (q_idx1 << 8) | (q_idx2 << 16) | (q_idx3 << 24);
            self.write_reg(IXGBE_RETA(i), reta_val);
        }

        // 2. RSS Random Key (RSSRK)
        // Set a random key or default key. 
        // 40 bytes = 10 registers.
        let seed: [u32; 10] = [
            0x6d5a56da, 0x255b0ec2, 0x4167253d, 0x43a38fb0, 0xd0ca2bc,
            0xdae70b72, 0x9a29e710, 0x37ee0c54, 0x56dd3901, 0x86d06371
        ];
        for i in 0..10 {
            self.write_reg(IXGBE_RSSRK(i), seed[i as usize]);
        }

        // 3. Multi-Queue Receive Control (MRQC)
        // Enable RSS and specific hash fields (IPv4, IPv6, TCP, UDP)
        self.write_reg(IXGBE_MRQC, 
            IXGBE_MRQC_RSSEN 
            | IXGBE_MRQC_RSS_FIELD_IPV4 
            | IXGBE_MRQC_RSS_FIELD_IPV4_TCP 
            | IXGBE_MRQC_RSS_FIELD_IPV4_UDP 
            | IXGBE_MRQC_RSS_FIELD_IPV6 
            | IXGBE_MRQC_RSS_FIELD_IPV6_TCP 
            | IXGBE_MRQC_RSS_FIELD_IPV6_UDP
        );

        // Configure all 4 queues
        for i in 0..4 {
            let i = i as u32;
            
            // enable advanced rx descriptors
            self.write_reg(
                IXGBE_SRRCTL(i),
                (self.read_reg(IXGBE_SRRCTL(i)) & !IXGBE_SRRCTL_DESCTYPE_MASK)
                    | IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF,
            );
            // drop packets if no descriptor available
            self.write_flag(IXGBE_SRRCTL(i), IXGBE_SRRCTL_DROP_EN);

            self.write_reg(IXGBE_RDBAL(i), self.receive_rings[i as usize].physical() as u32);
            self.write_reg(
                IXGBE_RDBAH(i),
                ((self.receive_rings[i as usize].physical() as u64) >> 32) as u32,
            );
            self.write_reg(
                IXGBE_RDLEN(i),
                (self.receive_rings[i as usize].len() * mem::size_of::<ixgbe_adv_rx_desc>()) as u32,
            );

            // set ring to empty at start
            self.write_reg(IXGBE_RDH(i), 0);
            self.write_reg(IXGBE_RDT(i), 0);

            // last sentence of section 4.6.7
            self.clear_flag(IXGBE_DCA_RXCTRL(i), 1 << 12);
        }

        self.write_flag(IXGBE_CTRL_EXT, IXGBE_CTRL_EXT_NS_DIS);

        // enable promisc mode by default
        self.set_promisc(true);

        // start rx
        self.write_flag(IXGBE_RXCTRL, IXGBE_RXCTRL_RXEN);
    }

    // section 4.6.8
    /// Initializes the tx queues of this device.
    fn init_tx(&mut self) {
        // crc offload and small packet padding
        self.write_flag(IXGBE_HLREG0, IXGBE_HLREG0_TXCRCEN | IXGBE_HLREG0_TXPADEN);

        // section 4.6.11.3.4 - set default buffer size allocations
        self.write_reg(IXGBE_TXPBSIZE(0), IXGBE_TXPBSIZE_40KB);
        for i in 1..8 {
            self.write_reg(IXGBE_TXPBSIZE(i), 0);
        }

        // required when not using DCB/VTd
        self.write_reg(IXGBE_DTXMXSZRQ, 0xfff);
        self.clear_flag(IXGBE_RTTDCS, IXGBE_RTTDCS_ARBDIS);

        // configure a single transmit queue/ring
        let i: u32 = 0;

        // section 7.1.9 - setup descriptor ring

        self.write_reg(IXGBE_TDBAL(i), self.transmit_ring.physical() as u32);
        self.write_reg(
            IXGBE_TDBAH(i),
            ((self.transmit_ring.physical() as u64) >> 32) as u32,
        );
        self.write_reg(
            IXGBE_TDLEN(i),
            (self.transmit_ring.len() * mem::size_of::<ixgbe_adv_tx_desc>()) as u32,
        );

        // descriptor writeback magic values, important to get good performance and low PCIe overhead
        // see 7.2.3.4.1 and 7.2.3.5 for an explanation of these values and how to find good ones
        // we just use the defaults from DPDK here, but this is a potentially interesting point for optimizations
        let mut txdctl = self.read_reg(IXGBE_TXDCTL(i));
        // there are no defines for this in ixgbe.rs for some reason
        // pthresh: 6:0, hthresh: 14:8, wthresh: 22:16
        txdctl &= !(0x3F | (0x3F << 8) | (0x3F << 16));
        txdctl |= 36 | (8 << 8) | (4 << 16);

        self.write_reg(IXGBE_TXDCTL(i), txdctl);

        // final step: enable DMA
        self.write_reg(IXGBE_DMATXCTL, IXGBE_DMATXCTL_TE);
    }

    /// Sets the rx queues` descriptors and enables the queues.
    ///
    /// # Panics
    /// Panics if length of `self.receive_ring` is not a power of 2.
    fn start_rx_queue(&mut self, queue_id: u16) {
        let q_idx = queue_id as usize;
        let ring_len = self.receive_rings[q_idx].len();
        
        if ring_len & (ring_len - 1) != 0 {
            panic!("number of receive queue entries must be a power of 2");
        }

        for i in 0..ring_len {
            self.receive_rings[q_idx][i].read.pkt_addr = self.receive_buffers[q_idx][i].physical() as u64;
            self.receive_rings[q_idx][i].read.hdr_addr = 0;
        }

        // enable queue and wait if necessary
        self.write_flag(IXGBE_RXDCTL(u32::from(queue_id)), IXGBE_RXDCTL_ENABLE);
        self.wait_write_reg(IXGBE_RXDCTL(u32::from(queue_id)), IXGBE_RXDCTL_ENABLE);

        // rx queue starts out full
        self.write_reg(IXGBE_RDH(u32::from(queue_id)), 0);

        // was set to 0 before in the init function
        self.write_reg(
            IXGBE_RDT(u32::from(queue_id)),
            (ring_len - 1) as u32,
        );
    }

    /// Enables the tx queues.
    ///
    /// # Panics
    /// Panics if length of `self.transmit_ring` is not a power of 2.
    fn start_tx_queue(&mut self, queue_id: u16) {
        if self.transmit_ring.len() & (self.transmit_ring.len() - 1) != 0 {
            panic!("number of receive queue entries must be a power of 2");
        }

        for i in 0..self.transmit_ring.len() {
            self.transmit_ring[i].read.buffer_addr = self.transmit_buffer[i].physical() as u64;
        }

        // tx queue starts out empty
        self.write_reg(IXGBE_TDH(u32::from(queue_id)), 0);
        self.write_reg(IXGBE_TDT(u32::from(queue_id)), 0);

        // enable queue and wait if necessary
        self.write_flag(IXGBE_TXDCTL(u32::from(queue_id)), IXGBE_TXDCTL_ENABLE);
        self.wait_write_reg(IXGBE_TXDCTL(u32::from(queue_id)), IXGBE_TXDCTL_ENABLE);
    }

    // see section 4.6.4
    /// Initializes the link of this device.
    fn init_link(&self) {
        // link auto-configuration register should already be set correctly, we're resetting it anyway
        self.write_reg(
            IXGBE_AUTOC,
            (self.read_reg(IXGBE_AUTOC) & !IXGBE_AUTOC_LMS_MASK) | IXGBE_AUTOC_LMS_10G_SERIAL,
        );
        self.write_reg(
            IXGBE_AUTOC,
            (self.read_reg(IXGBE_AUTOC) & !IXGBE_AUTOC_10G_PMA_PMD_MASK) | IXGBE_AUTOC_10G_XAUI,
        );
        // negotiate link
        self.write_flag(IXGBE_AUTOC, IXGBE_AUTOC_AN_RESTART);
        // datasheet wants us to wait for the link here, but we can continue and wait afterwards
    }

    /// Waits for the link to come up.
    fn wait_for_link(&self) {
        println!("   - waiting for link");
        let time = Instant::now();
        let mut speed = self.get_link_speed();
        while speed == 0 && time.elapsed().as_secs() < 10 {
            thread::sleep(Duration::from_millis(100));
            speed = self.get_link_speed();
        }
        println!("   - link speed is {} Mbit/s", self.get_link_speed());
    }

    /// Enables or disables promisc mode of this device.
    fn set_promisc(&self, enabled: bool) {
        if enabled {
            self.write_flag(IXGBE_FCTRL, IXGBE_FCTRL_MPE | IXGBE_FCTRL_UPE);
        } else {
            self.clear_flag(IXGBE_FCTRL, IXGBE_FCTRL_MPE | IXGBE_FCTRL_UPE);
        }
    }

    /// Set the IVAR registers, mapping interrupt causes to vectors.
    fn set_ivar(&mut self, direction: i8, queue_id: u16, mut msix_vector: u8) {
        let index = ((16 * (queue_id & 1)) as i16 + i16::from(8 * direction)) as u32;

        msix_vector |= IXGBE_IVAR_ALLOC_VAL as u8;

        let mut ivar = self.read_reg(IXGBE_IVAR(u32::from(queue_id >> 1)));
        ivar &= !(0xFF << index);
        ivar |= u32::from(msix_vector << index);

        self.write_reg(IXGBE_IVAR(u32::from(queue_id >> 1)), ivar);
    }

    /// Enable MSI-X interrupt for a queue.
    fn enable_msix_interrupt(&mut self, queue_id: u16) {
        // Step 1: The software driver associates between interrupt causes and MSI-X vectors and the
        //throttling timers EITR[n] by programming the IVAR[n] and IVAR_MISC registers.
        // Map all queues to Vector 0 because we only have one IRQ handle
        self.set_ivar(0, queue_id, 0);

        // Step 2: Program SRRCTL[n].RDMTS (per receive queue) if software uses the receive
        // descriptor minimum threshold interrupt

        // Step 3: The EIAC[n] registers should be set to auto clear for transmit and receive interrupt
        // causes (for best performance). The EIAC bits that control the other and TCP timer
        // interrupt causes should be set to 0b (no auto clear).
        self.write_reg(IXGBE_EIAC, IXGBE_EICR_RTX_QUEUE);

        // Step 4: Set the auto mask in the EIAM register according to the preferred mode of operation.

        // Step 5: Set the interrupt throttling in EITR[n] and GPIE according to the preferred mode of operation.

        // Step 6: Software enables the required interrupt causes by setting the EIMS register
        let mut mask: u32 = self.read_reg(IXGBE_EIMS);
        // Enable Vector 0 (which all queues are mapped to)
        mask |= 1 << 0; 

        self.write_reg(IXGBE_EIMS, mask);
    }

    /// Returns the link speed of this device.
    fn get_link_speed(&self) -> u16 {
        let speed = self.read_reg(IXGBE_LINKS);
        if (speed & IXGBE_LINKS_UP) == 0 {
            return 0;
        }
        match speed & IXGBE_LINKS_SPEED_82599 {
            IXGBE_LINKS_SPEED_100_82599 => 100,
            IXGBE_LINKS_SPEED_1G_82599 => 1000,
            IXGBE_LINKS_SPEED_10G_82599 => 10000,
            _ => 0,
        }
    }
}
