# Audio Drivers

Redox OS provides a unified audio scheme for low-latency playback and recording.

## Supported Hardware

### Intel HD Audio (`ihdad`)

- **Status**: Stable
- **Features**:
  - Support for Intel High Definition Audio controllers (standard in most modern PCs).
  - Multi-channel support (Stereo, 5.1).
  - Flexible sample rate and bit depth configuration.

### Realtek AC'97 (`ac97d`)

- **Status**: Stable
- **Features**:
  - Support for legacy Realtek and other AC'97-compliant audio chips.
  - Reliable basic audio playback.

### Sound Blaster 16 (`sb16d`)

- **Status**: Stable
- **Features**:
  - Support for SB16-compatible hardware (primarily used in older VMs like QEMU and VirtualBox).

## System Architecture

### Audio Scheme (`/scheme/audio`)

- All audio drivers expose a standard `/scheme/audio` interface.
- Applications can open this scheme to stream RAW PCM data.
- The `audiod` daemon (if active) provides mixing and global volume control.

### Spatial Audio (Work in Progress)

- Integration with `bevy_kira_audio` and other Rust-based audio libraries for spatialized sound in game engines.
