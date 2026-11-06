//! Unit tests for `webizen_render::wgpu_renderer` residency, capability probe,
//! pixel receipts, and `etalon_render::webizen_core` re-export parity.

use super::renderer::vertex_buffer_residency_bytes;
use super::*;

#[test]
fn renderer_residency_sizes_are_checked_and_include_row_padding() {
    assert_eq!(vertex_buffer_residency_bytes(10_000), Some(520_000));
    assert_eq!(offscreen_rgba8_bytes(1920, 1080), Some(8_294_400));
    assert_eq!(
        readback_layout(1920, 1080),
        Some((7680, 7680, 8_294_400, 8_294_400))
    );
    assert_eq!(readback_layout(1, 1), Some((256, 4, 256, 4)));
    assert_eq!(readback_layout(u32::MAX, 1), None);
}

#[test]
fn capability_report_is_deterministic_when_no_adapter_exists() {
    let report = WebGpuCapabilityReport::unavailable();
    assert_eq!(report.status, WebGpuStatus::AdapterUnavailable);
    assert!(!report.adapter_available);
    assert!(!report.device_available);
    assert_eq!(report.offscreen_format, PixelFormat::Rgba8Unorm);
    assert_eq!(report.max_offscreen_dimension, MAX_OFFSCREEN_DIMENSION);
}

#[test]
fn offscreen_extent_is_bounded_and_readback_layout_refuses_oversize() {
    assert_eq!(bounded_offscreen_extent(0, 0), (1, 1));
    assert_eq!(bounded_offscreen_extent(u32::MAX, u32::MAX), (2048, 2048));
    assert_eq!(
        readback_layout(2048, 2048).unwrap().2,
        MAX_PIXEL_READBACK_BYTES
    );
    assert_eq!(readback_layout(4096, 4096), None);
    assert_eq!(readback_layout(MAX_OFFSCREEN_DIMENSION + 1, 1), None);
}

#[test]
fn pixel_receipts_are_bounded_and_deterministic() {
    let first = pixel_receipt_from_rgba8(1, 1, &[1, 2, 3, 255]);
    let second = pixel_receipt_from_rgba8(1, 1, &[1, 2, 3, 255]);
    assert_eq!(first, second);
    assert_eq!(first.status, PixelReceiptStatus::Observed);
    assert!(!first.adapter_backed);
    assert_eq!(first.byte_len, 4);
    assert_eq!(first.nonzero_bytes, 4);

    let invalid = pixel_receipt_from_rgba8(1, 1, &[1, 2, 3]);
    assert_eq!(invalid.status, PixelReceiptStatus::InvalidExtent);

    let encoded = serde_json::to_vec(&first).expect("receipt serializes");
    assert!(encoded.len() <= 512, "receipt grew beyond bounded evidence");
    assert!(!encoded
        .windows(b"pixels".len())
        .any(|window| window == b"pixels"));
}

#[test]
fn webizen_core_camera_and_pixel_receipt_reexports_have_exact_parity() {
    let mut cam_local = Camera::default();
    let mut cam_core = etalon_render::webizen_core::Camera::default();
    cam_local.orbit(0.25, -0.15);
    cam_core.orbit(0.25, -0.15);
    cam_local.zoom(1.2);
    cam_core.zoom(1.2);
    cam_local.pan(4.0, -3.0);
    cam_core.pan(4.0, -3.0);

    let pt_local = cam_local
        .project_point(
            Vec3 {
                x: 0.2,
                y: -0.1,
                z: 10.0,
            },
            (800.0, 600.0),
        )
        .expect("projects in front of camera");
    let pt_core = cam_core
        .project_point(
            etalon_render::webizen_core::Vec3 {
                x: 0.2,
                y: -0.1,
                z: 10.0,
            },
            (800.0, 600.0),
        )
        .expect("projects in front of camera");
    assert_eq!(pt_local.x.to_bits(), pt_core.x.to_bits());
    assert_eq!(pt_local.y.to_bits(), pt_core.y.to_bits());

    let receipt_local: PixelReceipt = pixel_receipt_from_rgba8(2, 1, &[10, 20, 30, 255, 0, 0, 0, 255]);
    let receipt_core: etalon_render::webizen_core::PixelReceipt =
        etalon_render::webizen_core::pixel_receipt_from_rgba8(
            2,
            1,
            &[10, 20, 30, 255, 0, 0, 0, 255],
        );
    assert_eq!(receipt_local, receipt_core);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn capability_probe_reports_adapter_absence_without_failing_the_test() {
    let report = pollster::block_on(probe_webgpu_capabilities());
    assert_eq!(report.adapter_available, report.adapter.is_some());
    if !report.adapter_available {
        assert_eq!(report.status, WebGpuStatus::AdapterUnavailable);
    }
    eprintln!(
        "webizen-render capability probe: status={:?} adapter_available={} device_available={}",
        report.status, report.adapter_available, report.device_available
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn adapter_backed_pixel_acceptance_is_optional_and_reported() {
    let renderer = match pollster::block_on(WgpuRenderer::new_offscreen(8, 8)) {
        Ok(renderer) => renderer,
        Err(error) => {
            eprintln!(
                "webizen-render acceptance: adapter-backed pixels not observed ({error})"
            );
            return;
        }
    };

    renderer.clear("#123456");
    assert!(renderer.capabilities().adapter_available);
    assert!(renderer.capabilities().device_available);
    let readback = renderer.read_pixels_with_receipt();
    assert!(readback.pixels.is_some());
    assert_eq!(readback.receipt.status, PixelReceiptStatus::Observed);
    assert!(readback.receipt.adapter_backed);
    assert!(readback.receipt.observed);
    assert_eq!(readback.receipt.width, 8);
    assert_eq!(readback.receipt.height, 8);
    eprintln!(
        "webizen-render acceptance: adapter-backed pixels observed hash={:016x}",
        readback.receipt.content_hash
    );
}
