//! OS-level camera stream integrity (virtual-camera injection defense).
//!
//! Pure-landmark PAD is defeated if an attacker injects a pre-rendered 3D
//! avatar into a virtual camera. This module is the **pairing hook** for
//! hardware attestation: the PAD evaluator fails closed when attestation is
//! required and not confirmed.
//!
//! Platform-specific attestation (Windows Camera Frame Server / Media
//! Foundation device ID, Android Camera2 hardware level, AVFoundation unique
//! ID + entitlement) is implemented outside this crate; hosts set
//! [`CameraStreamAttestation`] before calling PAD.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraStreamSource {
    /// Host has not classified the stream.
    Unknown,
    /// Claimed physical CMOS / built-in or USB UVC with attestation path.
    PhysicalSensor,
    /// Known virtual camera / loopback / inject path.
    VirtualOrInjected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraStreamAttestation {
    pub source: CameraStreamSource,
    /// Host verified device identity / kernel driver path this session.
    pub hardware_attested: bool,
    /// When true, PAD must refuse VirtualOrInjected and unattested streams.
    pub require_physical: bool,
}

impl Default for CameraStreamAttestation {
    fn default() -> Self {
        Self {
            source: CameraStreamSource::Unknown,
            hardware_attested: false,
            // Default open for unit tests / mesh-only benches; production hosts
            // set require_physical = true for unlock paths.
            require_physical: false,
        }
    }
}

impl CameraStreamAttestation {
    pub fn physical_attested() -> Self {
        Self {
            source: CameraStreamSource::PhysicalSensor,
            hardware_attested: true,
            require_physical: true,
        }
    }

    pub fn virtual_camera() -> Self {
        Self {
            source: CameraStreamSource::VirtualOrInjected,
            hardware_attested: false,
            require_physical: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamIntegrityVerdict {
    Ok,
    VirtualCamera,
    UnattestedPhysicalRequired,
}

/// Gate PAD on stream origin. Fail closed when physical attestation is required.
pub fn check_camera_stream_integrity(att: CameraStreamAttestation) -> StreamIntegrityVerdict {
    if !att.require_physical {
        return StreamIntegrityVerdict::Ok;
    }
    match att.source {
        CameraStreamSource::VirtualOrInjected => StreamIntegrityVerdict::VirtualCamera,
        CameraStreamSource::Unknown => StreamIntegrityVerdict::UnattestedPhysicalRequired,
        CameraStreamSource::PhysicalSensor if !att.hardware_attested => {
            StreamIntegrityVerdict::UnattestedPhysicalRequired
        }
        CameraStreamSource::PhysicalSensor => StreamIntegrityVerdict::Ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_rejected_when_required() {
        assert_eq!(
            check_camera_stream_integrity(CameraStreamAttestation::virtual_camera()),
            StreamIntegrityVerdict::VirtualCamera
        );
    }

    #[test]
    fn physical_attested_ok() {
        assert_eq!(
            check_camera_stream_integrity(CameraStreamAttestation::physical_attested()),
            StreamIntegrityVerdict::Ok
        );
    }

    #[test]
    fn default_open_for_tests() {
        assert_eq!(
            check_camera_stream_integrity(CameraStreamAttestation::default()),
            StreamIntegrityVerdict::Ok
        );
    }
}
