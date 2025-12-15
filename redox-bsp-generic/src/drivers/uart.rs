//! Generic UART driver

use redox_hal::uart::{BaudRate, DataBits, FlowControl, Parity, StopBits, Uart, UartConfig};
use redox_hal::Error;

/// UART register offsets (16550-style)
mod regs {
    pub const RBR: usize = 0x00; // Receive Buffer Register
    pub const THR: usize = 0x00; // Transmit Holding Register
    pub const DLL: usize = 0x00; // Divisor Latch Low
    pub const IER: usize = 0x04; // Interrupt Enable Register
    pub const DLH: usize = 0x04; // Divisor Latch High
    pub const IIR: usize = 0x08; // Interrupt Identification Register
    pub const FCR: usize = 0x08; // FIFO Control Register
    pub const LCR: usize = 0x0C; // Line Control Register
    pub const MCR: usize = 0x10; // Modem Control Register
    pub const LSR: usize = 0x14; // Line Status Register
    pub const MSR: usize = 0x18; // Modem Status Register
    pub const SCR: usize = 0x1C; // Scratch Register
}

/// Line Status Register bits
mod lsr {
    pub const DR: u32 = 1 << 0; // Data Ready
    pub const OE: u32 = 1 << 1; // Overrun Error
    pub const PE: u32 = 1 << 2; // Parity Error
    pub const FE: u32 = 1 << 3; // Framing Error
    pub const BI: u32 = 1 << 4; // Break Interrupt
    pub const THRE: u32 = 1 << 5; // THR Empty
    pub const TEMT: u32 = 1 << 6; // Transmitter Empty
}

/// Line Control Register bits
mod lcr {
    pub const DLAB: u32 = 1 << 7; // Divisor Latch Access Bit
}

/// Generic 16550-style UART
pub struct GenericUart {
    base: usize,
    clock_freq: u32,
    config: UartConfig,
}

impl GenericUart {
    /// Create a new UART
    pub const fn new(base: usize, clock_freq: u32) -> Self {
        Self {
            base,
            clock_freq,
            config: UartConfig {
                baud_rate: BaudRate::Baud115200,
                data_bits: DataBits::Eight,
                parity: Parity::None,
                stop_bits: StopBits::One,
                flow_control: FlowControl::None,
            },
        }
    }

    /// Read a register
    unsafe fn read_reg(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.base + offset) as *const u32)
    }

    /// Write a register
    unsafe fn write_reg(&self, offset: usize, value: u32) {
        core::ptr::write_volatile((self.base + offset) as *mut u32, value);
    }

    /// Calculate divisor for baud rate
    fn calculate_divisor(&self, baud_rate: u32) -> u16 {
        ((self.clock_freq + 8 * baud_rate) / (16 * baud_rate)) as u16
    }

    /// Wait for transmitter ready
    fn wait_tx_ready(&self) {
        unsafe { while self.read_reg(regs::LSR) & lsr::THRE == 0 {} }
    }

    /// Wait for receiver ready
    fn wait_rx_ready(&self) -> bool {
        unsafe { self.read_reg(regs::LSR) & lsr::DR != 0 }
    }
}

impl Uart for GenericUart {
    type Error = Error;

    fn configure(&mut self, config: UartConfig) -> Result<(), Self::Error> {
        self.config = config;

        unsafe {
            // Disable interrupts
            self.write_reg(regs::IER, 0);

            // Set DLAB for divisor access
            self.write_reg(regs::LCR, lcr::DLAB);

            // Set baud rate
            let divisor = self.calculate_divisor(config.baud_rate.value());
            self.write_reg(regs::DLL, (divisor & 0xFF) as u32);
            self.write_reg(regs::DLH, ((divisor >> 8) & 0xFF) as u32);

            // Configure line control
            let mut lcr_val = 0u32;

            // Data bits
            lcr_val |= match config.data_bits {
                DataBits::Five => 0,
                DataBits::Six => 1,
                DataBits::Seven => 2,
                DataBits::Eight => 3,
                DataBits::Nine => 3, // Usually not supported
            };

            // Stop bits
            if matches!(config.stop_bits, StopBits::Two) {
                lcr_val |= 1 << 2;
            }

            // Parity
            match config.parity {
                Parity::None => {}
                Parity::Odd => lcr_val |= 1 << 3,
                Parity::Even => lcr_val |= (1 << 3) | (1 << 4),
                Parity::Mark => lcr_val |= (1 << 3) | (1 << 5),
                Parity::Space => lcr_val |= (1 << 3) | (1 << 4) | (1 << 5),
            }

            self.write_reg(regs::LCR, lcr_val);

            // Enable and reset FIFOs
            self.write_reg(regs::FCR, 0x07);
        }

        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, Self::Error> {
        for &byte in data {
            self.wait_tx_ready();
            unsafe {
                self.write_reg(regs::THR, byte as u32);
            }
        }
        Ok(data.len())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let mut count = 0;
        for byte in buffer.iter_mut() {
            if !self.wait_rx_ready() {
                break;
            }
            unsafe {
                *byte = self.read_reg(regs::RBR) as u8;
            }
            count += 1;
        }
        Ok(count)
    }

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        while !self.wait_rx_ready() {
            core::hint::spin_loop();
        }
        unsafe { Ok(self.read_reg(regs::RBR) as u8) }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        unsafe {
            while self.read_reg(regs::LSR) & lsr::TEMT == 0 {
                core::hint::spin_loop();
            }
        }
        Ok(())
    }

    fn is_rx_ready(&self) -> bool {
        unsafe { self.read_reg(regs::LSR) & lsr::DR != 0 }
    }

    fn is_tx_ready(&self) -> bool {
        unsafe { self.read_reg(regs::LSR) & lsr::THRE != 0 }
    }

    fn rx_available(&self) -> usize {
        if self.is_rx_ready() {
            1
        } else {
            0
        }
    }

    fn tx_free(&self) -> usize {
        if self.is_tx_ready() {
            16
        } else {
            0
        } // 16-byte FIFO
    }
}

/// Console UART (global instance)
static mut CONSOLE: Option<GenericUart> = None;

/// Initialize the console UART
pub unsafe fn init_console(base: usize, clock_freq: u32, baud_rate: BaudRate) {
    let mut uart = GenericUart::new(base, clock_freq);
    let _ = uart.configure(UartConfig::new_8n1(baud_rate));
    CONSOLE = Some(uart);
}

/// Get the console UART
pub fn console() -> Option<&'static mut GenericUart> {
    unsafe { CONSOLE.as_mut() }
}

/// Print to console
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        if let Some(console) = $crate::drivers::uart::console() {
            let _ = write!(console, $($arg)*);
        }
    }};
}

/// Print line to console
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\r\n") };
    ($($arg:tt)*) => {{
        $crate::print!($($arg)*);
        $crate::print!("\r\n");
    }};
}

impl core::fmt::Write for GenericUart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let _ = Uart::write(self, s.as_bytes());
        Ok(())
    }
}
