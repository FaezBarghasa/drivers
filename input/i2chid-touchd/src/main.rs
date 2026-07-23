//! # I2C-HID Multi-Touch Screen Driver for Redox Mobile
//!
//! Provides hardware digitizer multi-touch decoding, gesture recognition,
//! palm rejection, and event routing via Redox I/O scheme.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureType {
    None,
    Tap { x: u32, y: u32 },
    DoubleTap { x: u32, y: u32 },
    Swipe { dx: i32, dy: i32 },
    Pinch { factor: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub pressure: u16,
    pub major_axis: u16,
    pub is_palm: bool,
}

pub struct GestureEngine {
    active_contacts: BTreeMap<u32, TouchPoint>,
    pub screen_width: u32,
    pub screen_height: u32,
    palm_threshold_area: u32,
}

impl GestureEngine {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            active_contacts: BTreeMap::new(),
            screen_width: width,
            screen_height: height,
            palm_threshold_area: 1200,
        }
    }

    pub fn process_frame(&mut self, points: Vec<TouchPoint>) -> GestureType {
        let mut gesture = GestureType::None;

        // Filter palm touches based on contact area and pressure
        let valid_points: Vec<TouchPoint> = points
            .into_iter()
            .map(|mut pt| {
                let contact_area = (pt.major_axis as u32) * (pt.major_axis as u32);
                if contact_area > self.palm_threshold_area || pt.pressure > 950 {
                    pt.is_palm = true;
                }
                pt
            })
            .filter(|pt| !pt.is_palm)
            .collect();

        if valid_points.len() == 1 {
            let pt = valid_points[0];
            if let Some(prev) = self.active_contacts.get(&pt.id) {
                let dx = pt.x as i32 - prev.x as i32;
                let dy = pt.y as i32 - prev.y as i32;
                if dx.abs() > 15 || dy.abs() > 15 {
                    gesture = GestureType::Swipe { dx, dy };
                }
            } else {
                gesture = GestureType::Tap { x: pt.x, y: pt.y };
            }
        } else if valid_points.len() == 2 {
            let p1 = valid_points[0];
            let p2 = valid_points[1];
            let current_dist = (((p1.x as f32 - p2.x as f32).powi(2)
                + (p1.y as f32 - p2.y as f32).powi(2)))
                .sqrt();

            if let (Some(prev1), Some(prev2)) = (
                self.active_contacts.get(&p1.id),
                self.active_contacts.get(&p2.id),
            ) {
                let prev_dist = (((prev1.x as f32 - prev2.x as f32).powi(2)
                    + (prev1.y as f32 - prev2.y as f32).powi(2)))
                    .sqrt();

                if prev_dist > 1.0 {
                    let factor = current_dist / prev_dist;
                    if (factor - 1.0).abs() > 0.05 {
                        gesture = GestureType::Pinch { factor };
                    }
                }
            }
        }

        self.active_contacts.clear();
        for pt in valid_points {
            self.active_contacts.insert(pt.id, pt);
        }

        gesture
    }
}

fn main() {
    println!("[i2chid-touchd] Initializing mobile multi-touch digitizer driver...");
    let mut engine = GestureEngine::new(1080, 2400);

    // Simulated frame processing loop for multi-touch stream
    let sample_points = vec![
        TouchPoint {
            id: 0,
            x: 540,
            y: 1200,
            pressure: 500,
            major_axis: 20,
            is_palm: false,
        },
        TouchPoint {
            id: 1,
            x: 600,
            y: 1250,
            pressure: 480,
            major_axis: 22,
            is_palm: false,
        },
    ];

    let gesture = engine.process_frame(sample_points);
    println!("[i2chid-touchd] Detected touch gesture: {:?}", gesture);
}
