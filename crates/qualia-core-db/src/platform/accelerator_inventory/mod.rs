mod projection;
#[cfg(test)]
mod tests;

use etalon_agent::accelerator::{probe_npus, AcceleratorKind, HardwarePresence, NpuProbeReport};
use sha2::{Digest, Sha256};
use std::fmt::Write;

#[derive(Clone, Debug)]
pub struct NpuInventory {
    pub report: NpuProbeReport,
    pub fingerprint: String,
}

impl NpuInventory {
    pub fn from_report(report: NpuProbeReport) -> Self {
        let fingerprint = discovery_fingerprint(&report);
        Self {
            report,
            fingerprint,
        }
    }

    pub fn has_detected_npu(&self) -> bool {
        self.report.npu_presence() == HardwarePresence::Detected
    }

    pub fn to_json(&self) -> serde_json::Value {
        projection::to_json(self)
    }
}

/// Fresh discovery never restores provider or workload verification from a passport.
pub fn capture() -> NpuInventory {
    NpuInventory::from_report(probe_npus())
}

fn discovery_fingerprint(report: &NpuProbeReport) -> String {
    let mut hash = Sha256::new();
    let mut append = |value: &str| {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    };
    append("qualia-npu-discovery-v1");
    append(report.platform);
    append(report.source);
    append(projection::probe_status(report.status));
    let mut devices: Vec<_> = report.devices.iter().collect();
    devices.sort_by(|left, right| left.id.cmp(&right.id));
    for device in devices {
        append(&device.id);
        append(&device.name);
        append(if device.kind == AcceleratorKind::Npu {
            "npu"
        } else {
            "other_accelerator"
        });
        append(device.driver.as_deref().unwrap_or(""));
        append(device.driver_version.as_deref().unwrap_or(""));
        append(projection::hardware_presence(device.hardware));
    }
    let mut fingerprint = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(fingerprint, "{byte:02x}").expect("writing to a String cannot fail");
    }
    fingerprint
}
