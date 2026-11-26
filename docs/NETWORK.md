# Network Drivers

Redox OS supports a wide range of ethernet and wireless hardware, integrated with a high-performance network stack.

## Ethernet Adapters

### Intel (e1000, ixgbe)

- **Status**: Stable
- **Features**:
  - Support for 1Gbps (e1000) and 10Gbps (ixgbe) adapters.
  - Multi-queue support for 10GbE devices.
  - Efficient interrupt handling and polling modes.

### Realtek (RTL8139, RTL8168)

- **Status**: Stable
- **Features**:
  - Ubiquitous support for Realtek Gigabit and Fast Ethernet chips.
  - Low overhead driver implementation.

### VirtIO-Net

- **Status**: Stable
- **Features**:
  - High-performance networking in virtualized environments.
  - Support for multi-queue and checksum offloading.

## Wireless Adapters

Support for wireless networking is conducted through various vendor-specific drivers (Work in Progress).

## Advanced Features

### BBRv3 Integration

Redox supports BBRv3 (Bottleneck Bandwidth and Round-trip propagation time) for congestion control, providing superior performance on high-latency networks.

### Network Stack (`drivers/net/smoltcp`)

Redox leverages a customized version of `smoltcp` for its TCP/IP stack, providing robustness and safety. Detailed network stack documentation can be found in `drivers/NETWORK_STACK_SUMMARY.md`.
