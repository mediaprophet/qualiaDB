use etalon_agent::accelerator::{
    AcceleratorKind, ExecutionStatus, HardwarePresence, ProbeStatus, RuntimeAvailability,
    WorkloadSupport,
};
use serde_json::{json, Value};

use super::NpuInventory;

pub(super) fn probe_status(status: ProbeStatus) -> &'static str {
    match status {
        ProbeStatus::NotProbed => "not_probed",
        ProbeStatus::Complete => "complete",
        ProbeStatus::Partial => "partial",
        ProbeStatus::Unavailable => "unavailable",
        ProbeStatus::Failed => "failed",
    }
}

pub(super) fn hardware_presence(presence: HardwarePresence) -> &'static str {
    match presence {
        HardwarePresence::Unknown => "unknown",
        HardwarePresence::NotDetected => "not_detected",
        HardwarePresence::Detected => "detected",
    }
}

fn runtime_status(status: RuntimeAvailability) -> &'static str {
    match status {
        RuntimeAvailability::NotProbed => "not_probed",
        RuntimeAvailability::Unavailable => "unavailable",
        RuntimeAvailability::Available => "available",
        RuntimeAvailability::Failed => "failed",
    }
}

fn workload_status(status: WorkloadSupport) -> &'static str {
    match status {
        WorkloadSupport::NotProbed => "not_probed",
        WorkloadSupport::Unsupported => "unsupported",
        WorkloadSupport::Supported => "supported",
        WorkloadSupport::Failed => "failed",
    }
}

fn execution_status(status: ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::NotRun => "not_run",
        ExecutionStatus::Verified => "verified",
        ExecutionStatus::Failed => "failed",
    }
}

pub(super) fn to_json(inventory: &NpuInventory) -> Value {
    let report = &inventory.report;
    let devices: Vec<_> = report.devices.iter().map(|device| json!({
        "id": device.id,
        "name": device.name,
        "kind": if device.kind == AcceleratorKind::Npu { "npu" } else { "other_accelerator" },
        "driver": device.driver,
        "driver_version": device.driver_version,
        "hardware": hardware_presence(device.hardware),
        "runtime": runtime_status(device.runtime),
        "workload": workload_status(device.workload),
        "execution": execution_status(device.execution),
    })).collect();
    let issues: Vec<_> = report
        .issues
        .iter()
        .map(|issue| {
            json!({
                "operation": issue.operation,
                "message": issue.message,
            })
        })
        .collect();
    json!({
        "platform": report.platform,
        "source": report.source,
        "probe_status": probe_status(report.status),
        "hardware": hardware_presence(report.npu_presence()),
        "fingerprint": inventory.fingerprint,
        "devices": devices,
        "issues": issues,
        "production_executor": "not_implemented",
    })
}
