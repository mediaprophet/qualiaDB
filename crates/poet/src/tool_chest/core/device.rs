//! Device profile — physical device characteristics and capabilities.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DeviceType
// ---------------------------------------------------------------------------

/// The physical class of a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    /// Desktop computer — high compute, multiple displays.
    Desktop,
    /// Laptop — portable, moderate compute, single display.
    Laptop,
    /// Tablet — touch input, moderate compute, single display.
    Tablet,
    /// Phone — small screen, touch input, low compute, sensors.
    Phone,
    /// Smartwatch — tiny screen, wearable, sensors.
    Watch,
    /// Headless device — no display, compute only.
    Headless,
    /// TV or projector — display only, no input.
    Display,
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::Desktop
    }
}

impl DeviceType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Desktop => "Desktop",
            Self::Laptop => "Laptop",
            Self::Tablet => "Tablet",
            Self::Phone => "Phone",
            Self::Watch => "Watch",
            Self::Headless => "Headless",
            Self::Display => "Display",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Desktop => "\u{1F5A5}",
            Self::Laptop => "\u{1F4BB}",
            Self::Tablet => "\u{1F4F2}",
            Self::Phone => "\u{1F4F1}",
            Self::Watch => "\u{231A}",
            Self::Headless => "\u{1F916}",
            Self::Display => "\u{1F4FA}",
        }
    }
}

// ---------------------------------------------------------------------------
// DeviceCaps
// ---------------------------------------------------------------------------

/// Hardware capabilities of a device.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeviceCaps {
    /// Number of physical displays.
    pub display_count: u8,
    /// Total VRAM in MB (0 if unknown).
    pub vram_mb: u32,
    /// Total RAM in MB.
    pub ram_mb: u32,
    /// CPU core count.
    pub cpu_cores: u8,
    /// Has touch input.
    pub has_touch: bool,
    /// Has keyboard/mouse input.
    pub has_pointer: bool,
    /// Has GPS.
    pub has_gps: bool,
    /// Has accelerometer/gyroscope.
    pub has_motion: bool,
    /// Has camera.
    pub has_camera: bool,
    /// Has microphone.
    pub has_microphone: bool,
    /// Has biometric sensor (fingerprint, face).
    pub has_biometric: bool,
    /// Network bandwidth in Mbps (0 if unknown).
    pub bandwidth_mbps: u32,
    /// Battery powered (true) or wall-powered (false).
    pub is_battery: bool,
    /// Battery percentage (0-100, 0 if wall-powered).
    pub battery_pct: u8,
}

// ---------------------------------------------------------------------------
// DisplayInfo
// ---------------------------------------------------------------------------

/// A physical display attached to a device.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DisplayInfo {
    /// Unique display identifier within the device.
    pub display_id: String,
    /// Human-readable label.
    pub label: String,
    /// Width in physical pixels.
    pub width_px: u32,
    /// Height in physical pixels.
    pub height_px: u32,
    /// HiDPI scale factor (1.0 = standard, 2.0 = Retina).
    pub scale_factor: f32,
    /// Is this the primary display?
    pub is_primary: bool,
    /// Virtual desktop position (x, y) relative to primary display.
    pub position: (i32, i32),
}

// ---------------------------------------------------------------------------
// DeviceStatus
// ---------------------------------------------------------------------------

/// Connection status of a device in the device graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    /// Device is online and responsive.
    Online,
    /// Device is offline (last seen recently).
    Offline,
    /// Device is paired but not yet trusted.
    Paired,
    /// Device has been suspended (revoked by user).
    Suspended,
    /// Device is pairing (awaiting confirmation).
    Pairing,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self::Offline
    }
}

impl DeviceStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::Offline => "Offline",
            Self::Paired => "Paired",
            Self::Suspended => "Suspended",
            Self::Pairing => "Pairing",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Online => "rgba(100, 200, 100, 0.8)",
            Self::Offline => "var(--text-muted)",
            Self::Paired => "rgba(0, 200, 255, 0.8)",
            Self::Suspended => "rgba(255, 100, 100, 0.8)",
            Self::Pairing => "rgba(255, 165, 0, 0.8)",
        }
    }
}

// ---------------------------------------------------------------------------
// DeviceProfile
// ---------------------------------------------------------------------------

/// A device in the user's device graph.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Device DID — `did:qualia:device:<uuid>`.
    pub device_id: String,
    /// Device type.
    pub device_type: DeviceType,
    /// Human-readable label.
    pub label: String,
    /// Hardware capabilities.
    pub capabilities: DeviceCaps,
    /// Physical displays attached.
    pub displays: Vec<DisplayInfo>,
    /// Crypto key ID in the unified chain.
    pub crypto_key_id: String,
    /// Last seen timestamp (Unix seconds).
    pub last_seen: i64,
    /// Connection status.
    pub status: DeviceStatus,
}

impl DeviceProfile {
    /// Create a new device profile.
    pub fn new(device_id: &str, device_type: DeviceType, label: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
            device_type,
            label: label.to_string(),
            capabilities: DeviceCaps::default(),
            displays: Vec::new(),
            crypto_key_id: String::new(),
            last_seen: 0,
            status: DeviceStatus::Paired,
        }
    }

    /// Total display area in pixels across all displays.
    pub fn total_display_area(&self) -> u64 {
        self.displays
            .iter()
            .map(|d| d.width_px as u64 * d.height_px as u64)
            .sum()
    }

    /// Primary display, if any.
    pub fn primary_display(&self) -> Option<&DisplayInfo> {
        self.displays.iter().find(|d| d.is_primary)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_type_labels() {
        assert_eq!(DeviceType::Desktop.label(), "Desktop");
        assert_eq!(DeviceType::Phone.label(), "Phone");
    }

    #[test]
    fn device_profile_construction() {
        let dev = DeviceProfile::new(
            "did:qualia:device:abc123",
            DeviceType::Desktop,
            "Tim's Desktop",
        );
        assert_eq!(dev.device_type, DeviceType::Desktop);
        assert_eq!(dev.status, DeviceStatus::Paired);
    }

    #[test]
    fn display_area() {
        let dev = DeviceProfile {
            displays: vec![
                DisplayInfo {
                    width_px: 3840,
                    height_px: 2160,
                    is_primary: true,
                    ..Default::default()
                },
                DisplayInfo {
                    width_px: 1920,
                    height_px: 1080,
                    ..Default::default()
                },
            ],
            ..DeviceProfile::new("d1", DeviceType::Desktop, "Test")
        };
        assert_eq!(dev.total_display_area(), 3840 * 2160 + 1920 * 1080);
        assert!(dev.primary_display().is_some());
    }
}
