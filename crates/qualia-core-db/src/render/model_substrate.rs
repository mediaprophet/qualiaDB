//! Phase 6 — **model-as-substrate** *(graph–tensor duality, STELLAR §F)*.
//!
//! The renderer's acceptance for Phase 6: *project a view of a manifold that **also** holds
//! transcoded model weights — one substrate, one device.* This module demonstrates exactly that: a
//! single contiguous byte buffer co-locates
//!
//! 1. a **renderable manifold** — a `Tensor10D` buffer (`tensor::buffer_export`) the renderer
//!    projects through `render::projection` (the same `Volume3D` projection the GPU viewport draws);
//!    and
//! 2. a **Q42W weight section** — produced by the streaming transcoder
//!    (`q42_weight::transcode_safetensor_to_q42`), loadable in place via `Q42TensorIndex::from_q42`.
//!
//! One buffer, one device: the renderer projects the manifold while the weights are co-resident and
//! mappable — the unification claim, end-to-end. (The deeper claim — that a *single* node is both a
//! render primitive and a weight pointer — is the file-format-v2 work, STELLAR §C; here the two
//! sections share one substrate, which is the testable Phase-6 gate.)

use crate::q42_weight::{transcode_safetensor_to_q42, Q42TensorIndex, TranscodeReport};
use crate::render::projection::{project, ProjectionTarget};
use crate::tensor::buffer_export::{read_tensor_at, tensor_node_count, write_tensor_buffer, TensorBufferHeader};
use crate::tensor::Tensor10D;

/// Magic for the combined substrate header ("SUBQ").
pub const SUBSTRATE_MAGIC: u32 = 0x5342_5551;
/// Substrate header: magic(4) + version(2) + pad(2) + 4 × u64 section pointers = 40 bytes.
pub const SUBSTRATE_HEADER_BYTES: usize = 40;

/// Borrowed views of a substrate's two co-located sections.
#[derive(Debug, Clone, Copy)]
pub struct SubstrateSections<'a> {
    /// The renderable manifold (a `tensor::buffer_export` tensor buffer).
    pub manifold: &'a [u8],
    /// The transcoded model weights (a Q42W container).
    pub weights: &'a [u8],
}

/// Co-locate a renderable manifold buffer + a Q42W weights blob into one contiguous substrate.
pub fn compose_substrate(manifold_buf: &[u8], weights_q42: &[u8]) -> Vec<u8> {
    let manifold_off = SUBSTRATE_HEADER_BYTES;
    let weights_off = manifold_off + manifold_buf.len();
    let total = weights_off + weights_q42.len();
    let mut out = vec![0u8; total];
    out[0..4].copy_from_slice(&SUBSTRATE_MAGIC.to_le_bytes());
    out[4..6].copy_from_slice(&1u16.to_le_bytes());
    out[8..16].copy_from_slice(&(manifold_off as u64).to_le_bytes());
    out[16..24].copy_from_slice(&(manifold_buf.len() as u64).to_le_bytes());
    out[24..32].copy_from_slice(&(weights_off as u64).to_le_bytes());
    out[32..40].copy_from_slice(&(weights_q42.len() as u64).to_le_bytes());
    out[manifold_off..weights_off].copy_from_slice(manifold_buf);
    out[weights_off..total].copy_from_slice(weights_q42);
    out
}

/// Parse a substrate header and return zero-copy slices of its two sections.
pub fn read_substrate(buf: &[u8]) -> Result<SubstrateSections<'_>, String> {
    if buf.len() < SUBSTRATE_HEADER_BYTES {
        return Err("substrate: too small for header".to_string());
    }
    if u32::from_le_bytes(buf[0..4].try_into().unwrap()) != SUBSTRATE_MAGIC {
        return Err("substrate: bad magic".to_string());
    }
    let u64a = |o: usize| u64::from_le_bytes(buf[o..o + 8].try_into().unwrap()) as usize;
    let (m_off, m_len, w_off, w_len) = (u64a(8), u64a(16), u64a(24), u64a(32));
    let m_end = m_off.checked_add(m_len).ok_or("substrate: manifold overflow")?;
    let w_end = w_off.checked_add(w_len).ok_or("substrate: weights overflow")?;
    if m_end > buf.len() || w_end > buf.len() {
        return Err("substrate: section out of bounds".to_string());
    }
    Ok(SubstrateSections { manifold: &buf[m_off..m_end], weights: &buf[w_off..w_end] })
}

/// **The renderer's view of the substrate**: project every manifold node (`Volume3D`) — exactly
/// what the GPU viewport draws. Zero-copy over the manifold section.
pub fn project_manifold(sections: &SubstrateSections, time: f32) -> Result<Vec<[f32; 3]>, String> {
    let n = tensor_node_count(sections.manifold).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = read_tensor_at(sections.manifold, i).map_err(|e| e.to_string())?;
        out.push(project(&t, time, ProjectionTarget::Volume3D));
    }
    Ok(out)
}

/// Load the co-resident weights section in place (zero-copy header/manifest parse).
pub fn load_weights<'a>(sections: &SubstrateSections<'a>) -> Result<Q42TensorIndex, String> {
    Q42TensorIndex::from_q42(sections.weights)
}

/// End-to-end (Gate A): transcode `safetensor_src` → co-locate it with a renderable `geometry`
/// manifold in ONE substrate. Returns `(substrate, transcode_report)`.
pub fn build_model_substrate(
    geometry: &[Tensor10D],
    safetensor_src: &[u8],
) -> Result<(Vec<u8>, TranscodeReport), String> {
    let mut manifold = vec![0u8; TensorBufferHeader::total_bytes(geometry.len())];
    write_tensor_buffer(geometry, &mut manifold).map_err(|e| e.to_string())?;
    let mut weights = Vec::new();
    let report = transcode_safetensor_to_q42(safetensor_src, 12, &mut weights)?;
    Ok((compose_substrate(&manifold, &weights), report))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal safetensor: one F16 tensor of `nbytes` zeroed bytes.
    fn synth_safetensor(name: &str, nbytes: usize) -> Vec<u8> {
        let header = serde_json::json!({
            name: { "dtype": "F16", "shape": [nbytes / 2], "data_offsets": [0, nbytes] }
        });
        let hb = serde_json::to_vec(&header).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(&(hb.len() as u64).to_le_bytes());
        out.extend_from_slice(&hb);
        out.resize(out.len() + nbytes, 7u8); // non-zero so we can see it survived
        out
    }

    /// PHASE-6 ACCEPTANCE (Gate A): the renderer projects a manifold that ALSO holds transcoded
    /// model weights — one substrate, one device — demonstrated end-to-end.
    #[test]
    fn renders_a_manifold_that_also_holds_weights() {
        // A small renderable manifold (3 nodes at distinct positions).
        let geometry = [
            Tensor10D::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0),
        ];
        let model = synth_safetensor("blk.0.weight", 128);

        // Build ONE substrate holding both.
        let (substrate, report) = build_model_substrate(&geometry, &model).unwrap();
        assert_eq!(report.n_tensors, 1);

        // It is genuinely one contiguous buffer with two sections.
        let sections = read_substrate(&substrate).unwrap();
        assert!(sections.manifold.as_ptr() >= substrate.as_ptr());
        assert!(sections.weights.as_ptr() > sections.manifold.as_ptr());

        // 1) The renderer projects the manifold view (what the GPU viewport draws).
        let projected = project_manifold(&sections, 0.0).unwrap();
        assert_eq!(projected.len(), geometry.len());
        // each projection equals the direct projection of the same node (consistency).
        for (i, p) in projected.iter().enumerate() {
            let direct = project(&geometry[i], 0.0, ProjectionTarget::Volume3D);
            assert_eq!(*p, direct);
        }

        // 2) The transcoded weights are co-resident in the SAME buffer and load in place.
        let widx = load_weights(&sections).unwrap();
        assert_eq!(widx.header.n_tensors, 1);
        let blob = widx.blob(sections.weights, &widx.entries[0]);
        assert_eq!(blob.len(), 128);
        assert!(blob.iter().all(|&b| b == 7u8), "weight bytes survived verbatim");
    }

    #[test]
    fn substrate_round_trips_sections() {
        let m = vec![1u8, 2, 3, 4];
        let w = vec![9u8; 10];
        let s = compose_substrate(&m, &w);
        let sec = read_substrate(&s).unwrap();
        assert_eq!(sec.manifold, &m[..]);
        assert_eq!(sec.weights, &w[..]);
    }
}
