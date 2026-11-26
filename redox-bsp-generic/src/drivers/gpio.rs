//! Generic GPIO driver

use redox_hal::gpio::{GpioPin, Level, PinMode, Pull};
use redox_hal::Error;

/// Generic GPIO pin implementation
pub struct GenericGpioPin {
    base: usize,
    pin: u8,
    mode: PinMode,
}

impl GenericGpioPin {
    /// Create a new GPIO pin
    pub const fn new(base: usize, pin: u8) -> Self {
        Self {
            base,
            pin,
            mode: PinMode::Input,
        }
    }

    /// Get register offset for this pin
    fn reg_offset(&self) -> usize {
        (self.pin / 32) as usize * 4
    }

    /// Get bit mask for this pin
    fn bit_mask(&self) -> u32 {
        1 << (self.pin % 32)
    }

    /// Read a GPIO register
    unsafe fn read_reg(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.base + offset) as *const u32)
    }

    /// Write a GPIO register
    unsafe fn write_reg(&self, offset: usize, value: u32) {
        core::ptr::write_volatile((self.base + offset) as *mut u32, value);
    }
}

impl GpioPin for GenericGpioPin {
    type Error = Error;

    fn pin_number(&self) -> u8 {
        self.pin
    }

    fn set_mode(&mut self, mode: PinMode) -> Result<(), Self::Error> {
        // Generic implementation - would need board-specific offsets
        self.mode = mode;
        Ok(())
    }

    fn mode(&self) -> PinMode {
        self.mode
    }

    fn set_pull(&mut self, _pull: Pull) -> Result<(), Self::Error> {
        // Board-specific implementation needed
        Ok(())
    }

    fn read(&self) -> Result<Level, Self::Error> {
        unsafe {
            // Assume data-in register at offset 0x138 (AM335x style)
            let value = self.read_reg(0x138);
            if value & self.bit_mask() != 0 {
                Ok(Level::High)
            } else {
                Ok(Level::Low)
            }
        }
    }

    fn write(&mut self, level: Level) -> Result<(), Self::Error> {
        unsafe {
            match level {
                Level::High => {
                    // Set register
                    self.write_reg(0x194, self.bit_mask());
                }
                Level::Low => {
                    // Clear register
                    self.write_reg(0x190, self.bit_mask());
                }
            }
        }
        Ok(())
    }
}

/// GPIO port
pub struct GpioPort {
    base: usize,
    port_number: u8,
}

impl GpioPort {
    /// Create new GPIO port
    pub const fn new(base: usize, port_number: u8) -> Self {
        Self { base, port_number }
    }

    /// Get a pin from this port
    pub fn pin(&self, pin_number: u8) -> GenericGpioPin {
        GenericGpioPin::new(self.base, pin_number)
    }

    /// Read all pins
    pub fn read_all(&self) -> u32 {
        unsafe { core::ptr::read_volatile((self.base + 0x138) as *const u32) }
    }

    /// Write all pins
    pub fn write_all(&self, value: u32) {
        unsafe {
            core::ptr::write_volatile((self.base + 0x13C) as *mut u32, value);
        }
    }
}
