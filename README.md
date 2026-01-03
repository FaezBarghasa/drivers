# Redox OS Drivers

Welcome to the central repository for Redox OS drivers. Redox follows a microkernel architecture where all drivers run as isolated, high-performance user-space daemons.

## 🚀 Core Driver Documentation

We have organized our driver documentation into specific categories based on hardware type. Please refer to the following guides:

- **[Graphics Drivers](docs/GRAPHICS.md)** - AMD, NVIDIA, Intel, VirtIO, and the Graphics Abstraction Layer (GAL).
- **[Storage Drivers](docs/STORAGE.md)** - High-performance NVMe (Multi-queue), AHCI, and VirtIO-Block.
- **[Network Drivers](docs/NETWORK.md)** - Ethernet (Intel, Realtek, Atheros) and smoltcp integration.
- **[Audio Drivers](docs/AUDIO.md)** - Intel HD Audio, AC'97, and Sound Blaster.
- **[NPU & AI Drivers](docs/NPU_AI.md)** - Native AI acceleration and Neural Processing Unit support.
- **[USB & Input Drivers](docs/USB_INPUT.md)** - xHCI, USB HID, and the unified Inputd system.
- **[System & Bus](docs/SYSTEM.md)** - PCI (pcid), ACPI (acpid), and Hardware Discovery (hwd).

## 🏗️ Architecture

A device driver on Redox is a user-space daemon that communicates via system calls and schemes. This allows for:

- **Fault Isolation**: If a graphics driver crashes, the system remains stable.
- **Security**: Drivers only have access to the hardware they need.
- **Ease of Development**: Drivers are standard Rust programs with access to the full standard library.

### Key Schemes

- `/scheme/memory/physical`: Resource mapping for MMIO.
- `/scheme/irq`: Hardware interrupt handling.
- `/scheme/event`: Asynchronous event listening.

## 🛠️ Contribution & Development

If you are interested in developing drivers for Redox:

1. Read the **[Contribution Guide](COMMUNITY-HW.md)** to see requested hardware support.
2. Explore the **[Network Stack Summary](NETWORK_STACK_SUMMARY.md)** for networking details.
3. Consult the [Redox Book: Coding and Building](https://doc.redox-os.org/book/coding-and-building.html) for local development setup.

### Reference Hardware Tables

See **[COMMUNITY-HW.md](COMMUNITY-HW.md)** for a list of hardware being tested by the community and their current driver status.

---
*Redox OS Drivers - Modern. Secure. Fast.*
