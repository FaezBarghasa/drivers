# System & Bus Drivers

These drivers manage the fundamental communication buses and system-level features of the computer.

## PCI & PCIe (`pcid`)

The **pcid** daemon is the primary manager for the PCI bus.

- **Discovery**: Scans the PCI/PCIe bus for connected devices.
- **Configuration Space**: Provides a safe interface for other drivers to access PCI configuration registers.
- **Interrupt Mapping**: Handles MSI (Message Signaled Interrupts) and MSI-X allocation and mapping.
- **Driver Spawning**: Detects devices and automatically launches the appropriate driver daemon (via `pcid-spawner`).

## ACPI (`acpid`)

The **acpid** daemon interfaces with the Advanced Configuration and Power Interface.

- **Power Management**: Handles system shutdown, reboot, and sleep states.
- **Thermal Monitoring**: Reports CPU and system temperatures.
- **Battery Status**: Provides battery level and charging status for portable devices.
- **AML Parsing**: Uses `amlserde` for efficient processing of the ACPI Machine Language.

## Hardware Discovery (`hwd`)

The **hwd** daemon provides high-level hardware identification and inventory.

- Aggregates information from PCI, USB, and ACPI.
- Provides a human-readable summary of system components.

## VirtualBox Guest (`vboxd`)

Optimization driver for Redox running inside a VirtualBox guest.

- Shared folders support.
- Guest additions for seamless mouse integration.
- Hardware-accelerated graphics integration.
