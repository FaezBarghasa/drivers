//! ADC (Analog-to-Digital Converter) HAL traits

use crate::error::Result;

/// ADC resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcResolution {
    /// 6-bit resolution
    Bits6,
    /// 8-bit resolution
    Bits8,
    /// 10-bit resolution
    Bits10,
    /// 12-bit resolution
    Bits12,
    /// 14-bit resolution
    Bits14,
    /// 16-bit resolution
    Bits16,
}

impl AdcResolution {
    /// Get the number of bits
    pub fn bits(&self) -> u8 {
        match self {
            AdcResolution::Bits6 => 6,
            AdcResolution::Bits8 => 8,
            AdcResolution::Bits10 => 10,
            AdcResolution::Bits12 => 12,
            AdcResolution::Bits14 => 14,
            AdcResolution::Bits16 => 16,
        }
    }

    /// Get the maximum value
    pub fn max_value(&self) -> u16 {
        (1 << self.bits()) - 1
    }
}

/// ADC reference voltage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcReference {
    /// Internal reference
    Internal,
    /// External reference
    External,
    /// VCC as reference
    Vcc,
    /// Custom reference voltage in mV
    Custom(u16),
}

/// ADC sample time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleTime {
    /// 1.5 cycles
    Cycles1_5,
    /// 7.5 cycles
    Cycles7_5,
    /// 13.5 cycles
    Cycles13_5,
    /// 28.5 cycles
    Cycles28_5,
    /// 55.5 cycles
    Cycles55_5,
    /// 71.5 cycles
    Cycles71_5,
    /// 239.5 cycles
    Cycles239_5,
}

/// ADC configuration
#[derive(Debug, Clone, Copy)]
pub struct AdcConfig {
    /// Resolution
    pub resolution: AdcResolution,
    /// Reference voltage
    pub reference: AdcReference,
    /// Sample time
    pub sample_time: SampleTime,
    /// Enable continuous mode
    pub continuous: bool,
}

impl Default for AdcConfig {
    fn default() -> Self {
        Self {
            resolution: AdcResolution::Bits12,
            reference: AdcReference::Vcc,
            sample_time: SampleTime::Cycles13_5,
            continuous: false,
        }
    }
}

/// ADC channel trait
pub trait AdcChannel {
    /// Channel number
    fn channel(&self) -> u8;
}

/// ADC trait
pub trait Adc {
    /// Error type
    type Error;
    /// Channel type
    type Channel: AdcChannel;

    /// Configure the ADC
    fn configure(&mut self, config: AdcConfig) -> Result<(), Self::Error>;

    /// Enable the ADC
    fn enable(&mut self) -> Result<(), Self::Error>;

    /// Disable the ADC
    fn disable(&mut self) -> Result<(), Self::Error>;

    /// Read a single sample from a channel
    fn read(&mut self, channel: &Self::Channel) -> Result<u16, Self::Error>;

    /// Read and convert to voltage (in mV)
    fn read_voltage(&mut self, channel: &Self::Channel, vref_mv: u16) -> Result<u16, Self::Error>;

    /// Start continuous conversion
    fn start_continuous(&mut self, channel: &Self::Channel) -> Result<(), Self::Error>;

    /// Stop continuous conversion
    fn stop_continuous(&mut self) -> Result<(), Self::Error>;

    /// Get the latest sample (in continuous mode)
    fn latest_sample(&self) -> u16;

    /// Check if conversion is complete
    fn is_conversion_complete(&self) -> bool;

    /// Get the reference voltage in mV
    fn reference_voltage_mv(&self) -> u16;
}

/// ADC with DMA support
pub trait AdcDma: Adc {
    /// Start DMA transfer
    fn start_dma(
        &mut self,
        buffer: &mut [u16],
        channels: &[Self::Channel],
    ) -> Result<(), Self::Error>;

    /// Check if DMA transfer is complete
    fn is_dma_complete(&self) -> bool;

    /// Stop DMA transfer
    fn stop_dma(&mut self) -> Result<(), Self::Error>;
}

/// ADC controller
pub trait AdcController {
    /// Error type
    type Error;
    /// ADC type
    type Adc: Adc;

    /// Get an ADC instance
    fn adc(&mut self, adc_number: u8) -> Result<Self::Adc, Self::Error>;

    /// Get the number of available ADCs
    fn adc_count(&self) -> u8;
}

/// Internal temperature sensor
pub trait TemperatureSensor: Adc {
    /// Read temperature in millidegrees Celsius
    fn read_temperature(&mut self) -> Result<i32, Self::Error>;
}
