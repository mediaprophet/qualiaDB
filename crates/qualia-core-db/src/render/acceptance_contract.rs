//! Cross-platform evidence and acceptance gates for renderer capabilities.
//!
//! Hosts own the probes: native callers can populate these values from Windows, macOS, or
//! Linux adapter/runtime observations, while browser callers can populate them from Web APIs.
//! The renderer consumes only this small, platform-neutral contract. An absent measurement is
//! never treated as support.

/// Whether a capability has been measured, ruled out, or not measured yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CapabilityEvidence {
    Unknown = 0,
    Confirmed = 1,
    Refused = 2,
}

impl Default for CapabilityEvidence {
    fn default() -> Self {
        Self::Unknown
    }
}

impl CapabilityEvidence {
    pub const fn is_confirmed(self) -> bool {
        matches!(self, Self::Confirmed)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Confirmed => "confirmed",
            Self::Refused => "refused",
        }
    }

    /// Combine independent requirements into one fail-closed gate.
    pub const fn all(self, other: Self) -> Self {
        match (self, other) {
            (Self::Refused, _) | (_, Self::Refused) => Self::Refused,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Confirmed, Self::Confirmed) => Self::Confirmed,
        }
    }
}

/// Evidence that the producers needed by a temporal resolve are actually available.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TemporalProducerEvidence {
    pub motion_vectors: CapabilityEvidence,
    pub reactive_mask: CapabilityEvidence,
    pub linear_depth: CapabilityEvidence,
}

impl TemporalProducerEvidence {
    pub const fn gate(self) -> CapabilityEvidence {
        self.motion_vectors
            .all(self.reactive_mask)
            .all(self.linear_depth)
    }
}

/// Evidence for usable, resident environment probes rather than merely authored probe metadata.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnvironmentProbeEvidence {
    pub availability: CapabilityEvidence,
    pub resident_probe_count: Option<u8>,
}

impl EnvironmentProbeEvidence {
    pub const fn gate(self) -> CapabilityEvidence {
        match self.availability {
            CapabilityEvidence::Refused => CapabilityEvidence::Refused,
            CapabilityEvidence::Unknown => CapabilityEvidence::Unknown,
            CapabilityEvidence::Confirmed => match self.resident_probe_count {
                Some(0) => CapabilityEvidence::Refused,
                Some(_) => CapabilityEvidence::Confirmed,
                None => CapabilityEvidence::Unknown,
            },
        }
    }
}

/// Browser WebGPU evidence is deliberately split into API, adapter, and device stages. A
/// detached adapter probe is not a device initialization receipt and must not be used as one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrowserWebGpuEvidence {
    pub api: CapabilityEvidence,
    pub adapter: CapabilityEvidence,
    pub device: CapabilityEvidence,
}

impl BrowserWebGpuEvidence {
    pub const fn gate(self) -> CapabilityEvidence {
        self.api.all(self.adapter).all(self.device)
    }

    pub const fn adapter_gate(self) -> CapabilityEvidence {
        self.api.all(self.adapter)
    }
}

/// Evidence that bounded pixel readback has been tested for this runtime and format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelReadbackEvidence {
    pub rgba8: CapabilityEvidence,
    pub max_bytes: Option<u32>,
}

impl PixelReadbackEvidence {
    pub const fn gate(self) -> CapabilityEvidence {
        match self.rgba8 {
            CapabilityEvidence::Refused => CapabilityEvidence::Refused,
            CapabilityEvidence::Unknown => CapabilityEvidence::Unknown,
            CapabilityEvidence::Confirmed => match self.max_bytes {
                Some(bytes) if bytes > 0 => CapabilityEvidence::Confirmed,
                Some(0) => CapabilityEvidence::Refused,
                Some(_) => CapabilityEvidence::Unknown,
                None => CapabilityEvidence::Unknown,
            },
        }
    }
}

/// Measured presentation performance. The worst observed frame is used so a passing gate is
/// conservative even when the host does not provide percentile statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerformanceEvidence {
    pub sample_count: u16,
    pub worst_frame_time_us: Option<u32>,
    pub target_frame_time_us: Option<u32>,
}

impl PerformanceEvidence {
    pub const MIN_SAMPLES: u16 = 3;

    pub const fn measured(
        sample_count: u16,
        worst_frame_time_us: u32,
        target_frame_time_us: u32,
    ) -> Self {
        Self {
            sample_count,
            worst_frame_time_us: Some(worst_frame_time_us),
            target_frame_time_us: Some(target_frame_time_us),
        }
    }

    pub const fn gate(self) -> CapabilityEvidence {
        if self.sample_count < Self::MIN_SAMPLES {
            return CapabilityEvidence::Unknown;
        }
        match (self.worst_frame_time_us, self.target_frame_time_us) {
            (Some(worst), Some(target)) if target > 0 && worst <= target => {
                CapabilityEvidence::Confirmed
            }
            (Some(_), Some(target)) if target > 0 => CapabilityEvidence::Refused,
            _ => CapabilityEvidence::Unknown,
        }
    }
}

/// Complete acceptance evidence shared by native and browser runtime callers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapabilityAcceptance {
    pub temporal_producers: TemporalProducerEvidence,
    pub environment_probes: EnvironmentProbeEvidence,
    pub browser_webgpu: BrowserWebGpuEvidence,
    pub pixel_readback: PixelReadbackEvidence,
    pub performance: PerformanceEvidence,
}

impl CapabilityAcceptance {
    pub const fn temporal_gate(self) -> CapabilityEvidence {
        self.temporal_producers.gate()
    }

    pub const fn environment_probe_gate(self) -> CapabilityEvidence {
        self.environment_probes.gate()
    }

    pub const fn browser_webgpu_gate(self) -> CapabilityEvidence {
        self.browser_webgpu.gate()
    }

    pub const fn browser_webgpu_adapter_gate(self) -> CapabilityEvidence {
        self.browser_webgpu.adapter_gate()
    }

    pub const fn pixel_readback_gate(self) -> CapabilityEvidence {
        self.pixel_readback.gate()
    }

    pub const fn performance_gate(self) -> CapabilityEvidence {
        self.performance.gate()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrowserWebGpuEvidence, CapabilityAcceptance, CapabilityEvidence, EnvironmentProbeEvidence,
        PerformanceEvidence, PixelReadbackEvidence, TemporalProducerEvidence,
    };

    #[test]
    fn every_unmeasured_gate_is_unknown() {
        let acceptance = CapabilityAcceptance::default();
        assert_eq!(acceptance.temporal_gate(), CapabilityEvidence::Unknown);
        assert_eq!(
            acceptance.environment_probe_gate(),
            CapabilityEvidence::Unknown
        );
        assert_eq!(
            acceptance.browser_webgpu_gate(),
            CapabilityEvidence::Unknown
        );
        assert_eq!(acceptance.pixel_readback_gate(), CapabilityEvidence::Unknown);
        assert_eq!(acceptance.performance_gate(), CapabilityEvidence::Unknown);
    }

    #[test]
    fn temporal_and_environment_gates_require_all_evidence() {
        let temporal = TemporalProducerEvidence {
            motion_vectors: CapabilityEvidence::Confirmed,
            reactive_mask: CapabilityEvidence::Confirmed,
            linear_depth: CapabilityEvidence::Unknown,
        };
        assert_eq!(temporal.gate(), CapabilityEvidence::Unknown);

        let environment = EnvironmentProbeEvidence {
            availability: CapabilityEvidence::Confirmed,
            resident_probe_count: Some(0),
        };
        assert_eq!(environment.gate(), CapabilityEvidence::Refused);
    }

    #[test]
    fn browser_adapter_probe_does_not_admit_an_uninitialized_device() {
        let browser = BrowserWebGpuEvidence {
            api: CapabilityEvidence::Confirmed,
            adapter: CapabilityEvidence::Confirmed,
            device: CapabilityEvidence::Unknown,
        };
        assert_eq!(browser.adapter_gate(), CapabilityEvidence::Confirmed);
        assert_eq!(browser.gate(), CapabilityEvidence::Unknown);
    }

    #[test]
    fn readback_and_performance_require_measured_bounds() {
        let readback = PixelReadbackEvidence {
            rgba8: CapabilityEvidence::Confirmed,
            max_bytes: Some(0),
        };
        assert_eq!(readback.gate(), CapabilityEvidence::Refused);

        assert_eq!(
            PerformanceEvidence::measured(2, 1_000, 16_667).gate(),
            CapabilityEvidence::Unknown
        );
        assert_eq!(
            PerformanceEvidence::measured(3, 20_000, 16_667).gate(),
            CapabilityEvidence::Refused
        );
        assert_eq!(
            PerformanceEvidence::measured(3, 12_000, 16_667).gate(),
            CapabilityEvidence::Confirmed
        );
    }
}
