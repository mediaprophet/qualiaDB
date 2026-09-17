//! Capability detection — determines host surface and available engines.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Poet-owned capability detection. Containers call these functions to
//! fail closed when an engine is unreachable (e.g. on public web vs desktop
//! webview); no Webizen Studio types are required.

use web_sys::Window;

/// Host surface type — determines which engines are reachable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostSurface {
    /// Running inside the webizen-desktop Tauri webview.
    /// All daemon endpoints, GPU surface, and native commands are reachable.
    DesktopWebview,
    /// Running on the public web (no daemon).
    /// Containers must fail closed or use public-web fallbacks.
    PublicWeb,
}

/// Detect whether the app has native acceleration available (via Tauri webview OR connected local daemon).
pub fn is_native_host() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    has_tauri_internals(&window) || super::native_daemon::is_daemon_connected()
}

/// Detect the current host surface.
pub fn current_host_surface() -> HostSurface {
    if is_native_host() {
        HostSurface::DesktopWebview
    } else {
        HostSurface::PublicWeb
    }
}

/// Browser panes are only available on the desktop host or connected native daemon.
pub fn supports_browser_pane() -> bool {
    is_native_host()
}

/// GPU surface (wgpu) is only available on the desktop host or connected native daemon.
pub fn supports_gpu_surface() -> bool {
    is_native_host()
}

/// WebSocket telemetry stream is only available when daemon is reachable.
pub fn supports_telemetry_stream() -> bool {
    is_native_host()
}

/// Construct the daemon base URL (if native host or connected daemon).
pub fn daemon_base_url() -> Option<String> {
    if let Some(url) = super::native_daemon::get_connected_daemon_url() {
        return Some(url);
    }
    if !is_native_host() {
        return None;
    }
    // The daemon port is injected by the desktop host as a global.
    let window = web_sys::window().unwrap();
    let port = js_sys::Reflect::get(window.as_ref(), &"__QUALIA_DAEMON_PORT__".into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|p| p as u16)
        .unwrap_or(8080);
    Some(format!("http://127.0.0.1:{}", port))
}

/// Construct the telemetry WebSocket URL (if native host or connected daemon).
pub fn telemetry_ws_url() -> Option<String> {
    if let Some(url) = super::native_daemon::get_connected_daemon_url() {
        let ws_url = url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        return Some(format!("{ws_url}/telemetry/ws"));
    }
    if !is_native_host() {
        return None;
    }
    let window = web_sys::window().unwrap();
    let port = js_sys::Reflect::get(window.as_ref(), &"__QUALIA_DAEMON_PORT__".into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|p| p as u16)
        .unwrap_or(8080);
    Some(format!("ws://127.0.0.1:{}/telemetry", port))
}

/// Construct the LLM handshake WebSocket URL (if native host).
pub fn llm_ws_url() -> Option<String> {
    if !is_native_host() {
        return None;
    }
    Some("ws://127.0.0.1:4242".into())
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn has_tauri_internals(window: &Window) -> bool {
    let tauri = js_sys::Reflect::get(window.as_ref(), &"__TAURI_INTERNALS__".into());
    if let Ok(val) = &tauri {
        if !val.is_undefined() && !val.is_null() {
            return true;
        }
    }
    let tauri2 = js_sys::Reflect::get(window.as_ref(), &"__TAURI__".into());
    if let Ok(val) = &tauri2 {
        return !val.is_undefined() && !val.is_null();
    }
    false
}

// ---------------------------------------------------------------------------
// Sentinel Sandbox Capabilities & Policy Gating (Subsystem 3.3)
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Granular capability permissions requested by VibeScript formulas or containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SentinelCapability {
    /// Read access to the local semantic knowledge graph (`graph.read`)
    GraphRead,
    /// Mutation / insertion access to the knowledge graph (`graph.write`)
    GraphWrite,
    /// Live SHACL / Aura validation validation passes (`aura.validate`)
    AuraValidate,
    /// Tensor algebra and Q-Tensor GPU kernels (`tensor.compute`)
    TensorCompute,
    /// Spatial 3D / 10D mesh rendering on wgpu (`spatial.render`)
    SpatialRender,
    /// Network socket / HTTP fetch capabilities (`network.access`)
    NetworkAccess,
    /// Persistent disk / SQLite storage access (`storage.persistent`)
    StoragePersistent,
    /// Direct hardware sensor / audio microphone access (`hardware.direct`)
    HardwareDirect,
    /// Cryptographic signing with agent DID (`did.sign`)
    DidSign,
}

impl SentinelCapability {
    pub fn name(&self) -> &'static str {
        match self {
            SentinelCapability::GraphRead => "graph.read",
            SentinelCapability::GraphWrite => "graph.write",
            SentinelCapability::AuraValidate => "aura.validate",
            SentinelCapability::TensorCompute => "tensor.compute",
            SentinelCapability::SpatialRender => "spatial.render",
            SentinelCapability::NetworkAccess => "network.access",
            SentinelCapability::StoragePersistent => "storage.persistent",
            SentinelCapability::HardwareDirect => "hardware.direct",
            SentinelCapability::DidSign => "did.sign",
        }
    }

    /// Whether this capability is permitted on the public web surface without native host.
    pub fn is_allowed_on_public_web(&self) -> bool {
        matches!(
            self,
            SentinelCapability::GraphRead
                | SentinelCapability::AuraValidate
                | SentinelCapability::DidSign
        )
    }
}

/// A formal capability manifest attached to a container or script execution envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub name: String,
    pub version: String,
    pub author_did: String,
    pub required_capabilities: Vec<SentinelCapability>,
    pub gas_limit: u64,
    pub memory_limit_bytes: u64,
}

impl Default for CapabilityManifest {
    fn default() -> Self {
        Self {
            name: "Default Sandbox".into(),
            version: "1.0.0".into(),
            author_did: "did:qualia:anonymous".into(),
            required_capabilities: vec![
                SentinelCapability::GraphRead,
                SentinelCapability::AuraValidate,
            ],
            gas_limit: 10_000,
            memory_limit_bytes: 42 * 1024 * 1024, // 42MB Prolog Sentinel
        }
    }
}

/// The decision outcome of an evaluation pass against the Sentinel sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SentinelGateResult {
    /// All requested capabilities are permitted on the active host surface.
    Granted,
    /// Execution denied due to missing capabilities or host surface restrictions.
    Denied {
        reason: String,
        missing: Vec<SentinelCapability>,
    },
}

/// Evaluate whether a `CapabilityManifest` can be executed on the given `HostSurface`.
pub fn evaluate_manifest(
    manifest: &CapabilityManifest,
    surface: HostSurface,
) -> SentinelGateResult {
    if manifest.memory_limit_bytes > 42 * 1024 * 1024 {
        return SentinelGateResult::Denied {
            reason: "Requested memory limit exceeds 42MB Sentinel ceiling".into(),
            missing: vec![],
        };
    }

    if surface == HostSurface::DesktopWebview {
        // Desktop webview with native daemon grants full native capabilities
        return SentinelGateResult::Granted;
    }

    // Public web surface check
    let mut missing = Vec::new();
    for cap in &manifest.required_capabilities {
        if !cap.is_allowed_on_public_web() {
            missing.push(*cap);
        }
    }

    if missing.is_empty() {
        SentinelGateResult::Granted
    } else {
        SentinelGateResult::Denied {
            reason: format!(
                "Host surface 'PublicWeb' does not permit {} capability(ies)",
                missing.len()
            ),
            missing,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_surface_equality() {
        assert_eq!(HostSurface::DesktopWebview, HostSurface::DesktopWebview);
        assert_ne!(HostSurface::DesktopWebview, HostSurface::PublicWeb);
    }

    #[test]
    fn test_capability_names_and_safety() {
        assert_eq!(SentinelCapability::GraphRead.name(), "graph.read");
        assert!(SentinelCapability::GraphRead.is_allowed_on_public_web());
        assert!(!SentinelCapability::HardwareDirect.is_allowed_on_public_web());
        assert!(!SentinelCapability::TensorCompute.is_allowed_on_public_web());
    }

    #[test]
    fn test_sentinel_manifest_evaluation() {
        let manifest = CapabilityManifest::default();
        // Default only requires GraphRead and AuraValidate -> Allowed on public web
        let res_web = evaluate_manifest(&manifest, HostSurface::PublicWeb);
        assert_eq!(res_web, SentinelGateResult::Granted);

        // Heavy native requirements
        let native_manifest = CapabilityManifest {
            name: "GPU Vision Kernel".into(),
            version: "1.0.0".into(),
            author_did: "did:qualia:developer".into(),
            required_capabilities: vec![
                SentinelCapability::GraphRead,
                SentinelCapability::TensorCompute,
                SentinelCapability::HardwareDirect,
            ],
            gas_limit: 50_000,
            memory_limit_bytes: 32 * 1024 * 1024,
        };

        // Desktop webview grants it
        assert_eq!(
            evaluate_manifest(&native_manifest, HostSurface::DesktopWebview),
            SentinelGateResult::Granted
        );

        // Public web denies it
        match evaluate_manifest(&native_manifest, HostSurface::PublicWeb) {
            SentinelGateResult::Denied { missing, .. } => {
                assert_eq!(missing.len(), 2);
                assert!(missing.contains(&SentinelCapability::TensorCompute));
                assert!(missing.contains(&SentinelCapability::HardwareDirect));
            }
            _ => panic!("Expected denial on public web for native capabilities"),
        }
    }

    #[test]
    fn test_sentinel_memory_ceiling_enforcement() {
        let oversized = CapabilityManifest {
            name: "Oversized Allocator".into(),
            version: "1.0.0".into(),
            author_did: "did:qualia:test".into(),
            required_capabilities: vec![SentinelCapability::GraphRead],
            gas_limit: 100_000,
            memory_limit_bytes: 64 * 1024 * 1024, // > 42MB
        };

        match evaluate_manifest(&oversized, HostSurface::DesktopWebview) {
            SentinelGateResult::Denied { reason, .. } => {
                assert!(reason.contains("exceeds 42MB Sentinel ceiling"));
            }
            _ => panic!("Expected rejection of >42MB memory request"),
        }
    }
}
