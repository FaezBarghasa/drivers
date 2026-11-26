//! Common types and utilities
//!
//! This module re-exports and provides additional utility types.

/// Handle type alias
pub type Handle = usize;

/// Version information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const fn as_packed(&self) -> u32 {
        (self.major << 22) | (self.minor << 12) | self.patch
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Color type (RGBA, normalized floats)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    pub fn to_rgba8(&self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }

    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
}

/// Size in bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteSize(pub u64);

impl ByteSize {
    pub const KB: u64 = 1024;
    pub const MB: u64 = 1024 * 1024;
    pub const GB: u64 = 1024 * 1024 * 1024;

    pub const fn bytes(n: u64) -> Self {
        Self(n)
    }

    pub const fn kilobytes(n: u64) -> Self {
        Self(n * Self::KB)
    }

    pub const fn megabytes(n: u64) -> Self {
        Self(n * Self::MB)
    }

    pub const fn gigabytes(n: u64) -> Self {
        Self(n * Self::GB)
    }

    pub const fn as_bytes(&self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for ByteSize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 >= ByteSize::GB {
            write!(f, "{:.2} GB", self.0 as f64 / ByteSize::GB as f64)
        } else if self.0 >= ByteSize::MB {
            write!(f, "{:.2} MB", self.0 as f64 / ByteSize::MB as f64)
        } else if self.0 >= ByteSize::KB {
            write!(f, "{:.2} KB", self.0 as f64 / ByteSize::KB as f64)
        } else {
            write!(f, "{} bytes", self.0)
        }
    }
}

/// Range helper
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

impl<T> Range<T> {
    pub const fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
}

impl<T: Copy + core::ops::Sub<Output = T>> Range<T> {
    pub fn size(&self) -> T {
        self.end - self.start
    }
}
