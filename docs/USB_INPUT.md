# USB & Input Drivers

Redox OS features a modular USB stack and a unified input event system.

## USB Stack

### xHCI (`drivers/usb/xhcid`)

- **Status**: Stable
- **Features**:
  - Support for USB 3.0 and newer controllers.
  - Multi-threaded endpoint handling.
  - Support for synchronous and asynchronous transfers.

### USB HID (`drivers/usb/usbhidd`)

- Handles human interface devices (Keyboards, Mice, Gamepads).
- Translates USB HID reports into standard Redox input events.

### USB SCSI / Mass Storage

- Support for USB flash drives and external hard disks.

## Input System

### Inputd (`drivers/inputd`)

- The central input multiplexer.
- Aggregates events from PS/2, USB HID, and other sources.
- Provides a unified stream to the windowing system (Orbital).

### PS/2 (`drivers/input/ps2d`)

- Legacy support for PS/2 keyboards and mice.
- Found in many laptops and older servers.
