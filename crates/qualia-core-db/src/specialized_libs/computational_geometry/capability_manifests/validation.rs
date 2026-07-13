use super::*;

// ───────────────────────────────────────────────────────────────────────────
//  Validation
// ───────────────────────────────────────────────────────────────────────────

/// Validate that every manifest has non-empty backends and that any op
/// advertising a GPU backend also has a deterministic fallback.
///
/// **P10.1** also checks the new capability-truth fields: `maturity` must not
/// be `Planned` for an op that is in the manifest table (a planned op has no
/// code; it should not be advertised), and `topology_critical` ops must not
/// claim `ApproximateMetric` exactness (a topology-critical op whose only
/// exactness is an approximate metric is a misrepresentation — it needs an
/// exact predicate or a topology guarantee).
pub fn validate_manifests(manifests: &[OpManifest]) -> Result<(), String> {
    for m in manifests {
        if m.backends.is_empty() {
            return Err(format!("{}: backends must be non-empty", m.op));
        }

        let has_gpu = m.backends.iter().any(|b| b.requires_gpu());
        let has_fallback = m.backends.iter().any(|b| b.is_deterministic_fallback());

        if has_gpu && !has_fallback {
            return Err(format!(
                "{}: advertises GPU backend without deterministic CPU/WASM fallback",
                m.op
            ));
        }

        if m.limits.max_output_bytes == 0 {
            return Err(format!("{}: max_output_bytes must be finite", m.op));
        }

        if m.limits.max_memory_bytes == 0 {
            return Err(format!("{}: max_memory_bytes must be finite", m.op));
        }

        // P10.1 — a manifest entry is a claim that the op exists in code.
        if matches!(m.maturity, Maturity::Planned) {
            return Err(format!(
                "{}: maturity is Planned but it appears in GEOMETRY_OP_MANIFESTS — planned ops have no code and must not be advertised",
                m.op
            ));
        }

        // P10.1 — topology-critical ops must not claim only approximate metric
        // exactness (that would be claiming a topology decision from an
        // approximate quantity — the kind of overstatement P10 exists to catch).
        if m.topology_critical && matches!(m.exactness, ExactnessClass::ApproximateMetric) {
            return Err(format!(
                "{}: topology_critical but exactness is ApproximateMetric — topology decisions require an exact predicate or topology guarantee",
                m.op
            ));
        }
    }
    Ok(())
}
