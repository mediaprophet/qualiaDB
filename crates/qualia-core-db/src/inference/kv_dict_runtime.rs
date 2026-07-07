//! W5b Phase 4b — runtime KV-dictionary install + reconstruction, in CORE (engine-side, no forge dep).
//!
//! Holds the certified per-layer K/V dictionaries and, when enabled, reconstructs each K/V vector on the
//! KV-cache **write** path (`reconstruct_kv`) so attention reads the dictionary-reconstructed vectors.
//! This is the engine half of "forge produces, engine runs": the forge learns + certifies + packages a
//! dictionary artifact; the engine [`load_certified`]s it (verifying the provenance gate) and installs
//! it here. Reconstruct-on-write is quality-identical to a real compressed cache (store code, reconstruct
//! on read) — the compressed GPU cache layout + shader reconstruction is the remaining Phase 4b work.
//!
//! Gated + zero-cost when off (one relaxed atomic load on the attention path).

#![cfg(not(target_arch = "wasm32"))]

use crate::kv_dict::KvDictionary;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(false);

struct Rt {
    /// Per-layer K dictionaries (`None` = layer not certified / too few vectors → passthrough).
    k: Vec<Option<KvDictionary>>,
    v: Vec<Option<KvDictionary>>,
    sparsity: usize,
}

fn rt() -> &'static Mutex<Option<Rt>> {
    static R: OnceLock<Mutex<Option<Rt>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(None))
}

/// The serialized dictionary artifact payload (what rides inside the framed `.q42art` after the
/// provenance header). Shared by the forge packager and the engine loader — the one source of truth
/// for the on-disk dictionary format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KvDictArtifact {
    pub sparsity: usize,
    pub head_dim: usize,
    pub k: Vec<Option<KvDictionary>>,
    pub v: Vec<Option<KvDictionary>>,
}

/// Install the per-layer dictionaries and turn reconstruction ON.
pub fn enable(k: Vec<Option<KvDictionary>>, v: Vec<Option<KvDictionary>>, sparsity: usize) {
    if let Ok(mut g) = rt().lock() {
        *g = Some(Rt { k, v, sparsity });
    }
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
}

#[inline]
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Free the installed dictionaries.
pub fn clear() {
    if let Ok(mut g) = rt().lock() {
        *g = None;
    }
}

/// Metadata returned by [`load_certified`] on success — the gate numbers the artifact was certified at.
#[derive(Debug, Clone)]
pub struct CertInfo {
    pub sparsity: usize,
    pub head_dim: usize,
    /// Certified ΔPPL fraction (e.g. 0.0065 = +0.65%).
    pub delta_ppl: f64,
    /// Layers with an installed K (resp. V) dictionary.
    pub k_layers: usize,
    pub v_layers: usize,
}

/// The frame magic written by the forge packager (`package::FRAME_MAGIC`). Duplicated here so the engine
/// can read the format without the forge feature; kept in sync with `wgsl_forge::calibration::package`.
const FRAME_MAGIC: &[u8; 8] = b"QCAL0001";

/// Minimal read-side view of the provenance header — just the fields the engine gates on. Extra fields
/// in the CBOR map are ignored; `#[serde(default)]` tolerates any this engine build doesn't know.
#[derive(serde::Deserialize, Default)]
struct MiniProvenance {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    delta_ppl: f64,
    #[serde(default)]
    passed: bool,
}

/// Decode a dictionary artifact payload (CBOR [`KvDictArtifact`]) and install it. The payload is the
/// bytes AFTER the provenance frame header — see [`load_certified`] for the full framed path.
pub fn install_from_cbor(payload: &[u8]) -> Result<CertInfo, String> {
    let art: KvDictArtifact =
        ciborium::from_reader(payload).map_err(|e| format!("KvDictArtifact CBOR: {e}"))?;
    let info = CertInfo {
        sparsity: art.sparsity,
        head_dim: art.head_dim,
        delta_ppl: f64::NAN, // filled by load_certified from provenance; NaN when installed raw
        k_layers: art.k.iter().filter(|d| d.is_some()).count(),
        v_layers: art.v.iter().filter(|d| d.is_some()).count(),
    };
    if info.k_layers == 0 && info.v_layers == 0 {
        return Err("artifact has no dictionaries".into());
    }
    enable(art.k, art.v, art.sparsity);
    Ok(info)
}

/// Load a certified KV-dictionary artifact from a framed `.q42art` file, **verify its provenance gate**
/// (kind == KvDictionary AND passed == true), and install it. Fail-closed: a bad frame, wrong artifact
/// kind, or an artifact that did NOT pass its ΔPPL gate is refused — the engine only runs certified
/// artifacts. Returns the certified gate numbers on success.
pub fn load_certified(path: &std::path::Path) -> Result<CertInfo, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path:?}: {e}"))?;
    if bytes.len() < 12 || &bytes[..8] != FRAME_MAGIC {
        return Err("bad frame magic (not a QCAL artifact)".into());
    }
    let prov_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let payload_start = 12usize
        .checked_add(prov_len)
        .filter(|&e| e <= bytes.len())
        .ok_or("provenance length out of range")?;
    let prov: MiniProvenance = ciborium::from_reader(&bytes[12..payload_start])
        .map_err(|e| format!("provenance CBOR: {e}"))?;
    if prov.kind != "KvDictionary" {
        return Err(format!("not a KV-dictionary artifact (kind={:?})", prov.kind));
    }
    if !prov.passed {
        return Err("artifact did NOT pass its ΔPPL gate — refusing (fail-closed)".into());
    }
    let mut info = install_from_cbor(&bytes[payload_start..])?;
    info.delta_ppl = prov.delta_ppl;
    Ok(info)
}

/// Reconstruct each of the `n_kv` head vectors in `proj` (length ≥ `n_kv * head_dim`) through this
/// layer's dictionary, in place. No-op (one atomic load) when disabled, when the layer has no
/// dictionary, or on a head_dim mismatch — so the caller stores the original vector unchanged.
#[inline]
pub fn reconstruct_kv(layer: usize, k_not_v: bool, proj: &mut [f32], n_kv: usize, head_dim: usize) {
    if !ENABLED.load(Ordering::Relaxed) || head_dim == 0 {
        return;
    }
    let Ok(g) = rt().lock() else {
        return;
    };
    let Some(rt) = g.as_ref() else {
        return;
    };
    let dicts = if k_not_v { &rt.k } else { &rt.v };
    let Some(Some(dict)) = dicts.get(layer) else {
        return;
    };
    if dict.dim != head_dim {
        return;
    }
    for h in 0..n_kv {
        let s = h * head_dim;
        if s + head_dim > proj.len() {
            break;
        }
        let code = dict.encode(&proj[s..s + head_dim], rt.sparsity);
        let recon = dict.reconstruct(&code);
        proj[s..s + head_dim].copy_from_slice(&recon);
    }
}
