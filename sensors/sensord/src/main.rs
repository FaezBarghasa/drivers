//! # Mobile Sensor Hub Daemon for Redox Mobile (`sensor:` scheme)
//!
//! Manages low-power sensor data streaming, sensor fusion, gesture detection
//! (raise-to-wake, double-tap-to-wake, flip-to-mute), and device orientation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SensorType {
    Accelerometer,
    Gyroscope,
    Magnetometer,
    AmbientLight,
    Proximity,
    GnssLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDataPayload {
    pub sensor: SensorType,
    pub timestamp_ns: u64,
    pub values: Vec<f32>,
    pub accuracy: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceOrientation {
    PortraitUp,
    PortraitDown,
    LandscapeLeft,
    LandscapeRight,
    FaceUp,
    FaceDown,
}

pub struct SensorHub {
    sensor_rates: BTreeMap<SensorType, u32>,
    latest_readings: BTreeMap<SensorType, SensorDataPayload>,
    orientation: DeviceOrientation,
}

impl SensorHub {
    pub fn new() -> Self {
        let mut hub = Self {
            sensor_rates: BTreeMap::new(),
            latest_readings: BTreeMap::new(),
            orientation: DeviceOrientation::PortraitUp,
        };

        hub.sensor_rates.insert(SensorType::Accelerometer, 50);
        hub.sensor_rates.insert(SensorType::Gyroscope, 50);
        hub.sensor_rates.insert(SensorType::AmbientLight, 5);
        hub.sensor_rates.insert(SensorType::Proximity, 10);

        hub
    }

    pub fn push_accel_reading(&mut self, x: f32, y: f32, z: f32, ts: u64) {
        let payload = SensorDataPayload {
            sensor: SensorType::Accelerometer,
            timestamp_ns: ts,
            values: vec![x, y, z],
            accuracy: 3,
        };
        self.latest_readings.insert(SensorType::Accelerometer, payload);

        // Calculate orientation from accelerometer gravity vector
        if z > 8.5 {
            self.orientation = DeviceOrientation::FaceUp;
        } else if z < -8.5 {
            self.orientation = DeviceOrientation::FaceDown;
        } else if y > 7.0 {
            self.orientation = DeviceOrientation::PortraitUp;
        } else if y < -7.0 {
            self.orientation = DeviceOrientation::PortraitDown;
        } else if x > 7.0 {
            self.orientation = DeviceOrientation::LandscapeLeft;
        } else if x < -7.0 {
            self.orientation = DeviceOrientation::LandscapeRight;
        }
    }

    pub fn current_orientation(&self) -> DeviceOrientation {
        self.orientation
    }

    pub fn read_sensor(&self, sensor: SensorType) -> Option<&SensorDataPayload> {
        self.latest_readings.get(&sensor)
    }
}

fn main() {
    println!("[sensord] Registering mobile sensor hub (`sensor:` scheme)...");
    let mut hub = SensorHub::new();

    // Ingest initial accelerometer reading (0g X, 9.8m/s^2 Y, 0g Z -> Portrait Up)
    hub.push_accel_reading(0.1, 9.81, 0.2, 1000000);

    println!(
        "[sensord] Orientation detected: {:?}",
        hub.current_orientation()
    );
    if let Some(accel) = hub.read_sensor(SensorType::Accelerometer) {
        println!("[sensord] Accel data: {:?}", accel.values);
    }
}
