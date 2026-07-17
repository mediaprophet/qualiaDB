//! Phase 11 — twin substrate bridge: validated mesh → analysis eligibility.

use super::geometry_ir::MeshIR;
use super::validate::{validate_mesh_ir, MeshValidationStatus};

/// What solver domain a mesh may claim.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisDomain {
    None = 0,
    ElasticityPreview = 1,
    VisualizationOnly = 2,
}

#[derive(Debug, Clone, Copy)]
pub struct TwinEligibility {
    pub domain: AnalysisDomain,
    pub validation: MeshValidationStatus,
    pub vertex_count: u32,
    pub triangle_count: u32,
    /// If false, solvers must refuse.
    pub allowed: bool,
    pub refuse_reason: &'static str,
}

/// Gate mesh into twin/solver contracts. Soft refuse is honest completeness.
pub fn assess_twin_eligibility(mesh: &MeshIR) -> TwinEligibility {
    let rep = validate_mesh_ir(mesh);
    if !rep.ok() {
        return TwinEligibility {
            domain: AnalysisDomain::None,
            validation: rep.status,
            vertex_count: rep.vertex_count,
            triangle_count: rep.triangle_count,
            allowed: false,
            refuse_reason: "mesh failed validation",
        };
    }
    // Heightfield recon is visualization-first until FEA mesh index is wired.
    TwinEligibility {
        domain: AnalysisDomain::VisualizationOnly,
        validation: rep.status,
        vertex_count: rep.vertex_count,
        triangle_count: rep.triangle_count,
        allowed: true,
        refuse_reason: "",
    }
}

/// Explicit refuse for unsupported FEA claims.
pub fn refuse_fea_unless_eligible(elig: TwinEligibility) -> Result<(), &'static str> {
    if elig.domain == AnalysisDomain::ElasticityPreview && elig.allowed {
        Ok(())
    } else {
        Err("elasticity FEA not available for this mesh class (visualization-only or invalid)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::geometry_ir::MeshIR;

    #[test]
    fn valid_mesh_viz_only() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        m.indices = vec![0, 1, 2];
        m.recompute_bounds_and_hash();
        let e = assess_twin_eligibility(&m);
        assert!(e.allowed);
        assert_eq!(e.domain, AnalysisDomain::VisualizationOnly);
        assert!(refuse_fea_unless_eligible(e).is_err());
    }
}
