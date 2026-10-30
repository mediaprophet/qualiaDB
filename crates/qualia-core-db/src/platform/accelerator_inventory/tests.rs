use etalon_agent::accelerator::{
    AcceleratorDevice, AcceleratorKind, HardwarePresence, NpuProbeReport, ProbeStatus,
};

use super::NpuInventory;

fn report() -> NpuProbeReport {
    let mut report = NpuProbeReport::new("fixture", "fixture");
    report.status = ProbeStatus::Complete;
    let mut device =
        AcceleratorDevice::detected("npu:1".into(), "NPU".into(), AcceleratorKind::Npu);
    device.driver = Some("fixture-driver".into());
    device.driver_version = Some("1".into());
    report.devices.push(device);
    report
}

#[test]
fn discovery_projection_does_not_claim_runtime_or_execution() {
    let inventory = NpuInventory::from_report(report());
    assert!(inventory.has_detected_npu());
    let json = inventory.to_json();
    assert_eq!(json["hardware"], "detected");
    assert_eq!(json["devices"][0]["runtime"], "not_probed");
    assert_eq!(json["devices"][0]["workload"], "not_probed");
    assert_eq!(json["devices"][0]["execution"], "not_run");
    assert_eq!(json["production_executor"], "not_implemented");
}

#[test]
fn a_failed_probe_is_distinct_from_no_visible_devices() {
    let mut empty = NpuProbeReport::new("fixture", "fixture");
    empty.status = ProbeStatus::Complete;
    let mut failed = empty.clone();
    failed.status = ProbeStatus::Failed;
    assert_eq!(empty.npu_presence(), HardwarePresence::NotDetected);
    assert_eq!(failed.npu_presence(), HardwarePresence::Unknown);
    assert_ne!(
        NpuInventory::from_report(empty).fingerprint,
        NpuInventory::from_report(failed).fingerprint
    );
}

#[test]
fn device_or_driver_changes_invalidate_discovery_fingerprint() {
    let original = report();
    let fingerprint = NpuInventory::from_report(original.clone()).fingerprint;
    let mut changed_driver = original.clone();
    changed_driver.devices[0].driver_version = Some("2".into());
    assert_ne!(
        fingerprint,
        NpuInventory::from_report(changed_driver).fingerprint
    );
    let mut changed_device = original;
    changed_device.devices[0].id = "npu:2".into();
    assert_ne!(
        fingerprint,
        NpuInventory::from_report(changed_device).fingerprint
    );
}

#[test]
fn enumeration_order_does_not_change_fingerprint() {
    let mut original = report();
    let mut second = original.devices[0].clone();
    second.id = "npu:2".into();
    original.devices.push(second);
    let mut reversed = original.clone();
    reversed.devices.reverse();
    assert_eq!(
        NpuInventory::from_report(original).fingerprint,
        NpuInventory::from_report(reversed).fingerprint
    );
}
