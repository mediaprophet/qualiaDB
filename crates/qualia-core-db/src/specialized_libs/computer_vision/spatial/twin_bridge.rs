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

/// Promote a validated mesh to elasticity **preview** (not certified FEA / A4).
/// Software assurance class: A1 closed-form bar stretch check only.
pub fn promote_elasticity_preview(mesh: &MeshIR) -> TwinEligibility {
    let mut e = assess_twin_eligibility(mesh);
    if e.allowed && e.validation == MeshValidationStatus::Valid {
        e.domain = AnalysisDomain::ElasticityPreview;
        e.refuse_reason = "";
    }
    e
}

/// Closed-form axial stretch of a unit bar: δ = FL / (AE).
/// Returns displacement; fails closed if parameters invalid.
/// **Not** a mesh FEA solve — vertical slice for Phase 12 assurance notes.
#[derive(Debug, Clone, Copy)]
pub struct BarStretchInput {
    pub force_n: f32,
    pub length_m: f32,
    pub area_m2: f32,
    pub youngs_pa: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct BarStretchResult {
    pub displacement_m: f32,
    pub stress_pa: f32,
    pub strain: f32,
    pub assurance_note: &'static str,
}

pub fn closed_form_bar_stretch(inp: BarStretchInput) -> Result<BarStretchResult, &'static str> {
    if inp.length_m <= 0.0 || inp.area_m2 <= 0.0 || inp.youngs_pa <= 0.0 {
        return Err("invalid bar geometry or modulus");
    }
    let stress = inp.force_n / inp.area_m2;
    let strain = stress / inp.youngs_pa;
    let disp = strain * inp.length_m;
    Ok(BarStretchResult {
        displacement_m: disp,
        stress_pa: stress,
        strain,
        assurance_note: "A1 closed-form axial bar only; not mesh FEA; not A4 certified",
    })
}

/// Run elasticity preview only if eligible; else refuse.
pub fn run_elasticity_preview_if_eligible(
    elig: TwinEligibility,
    inp: BarStretchInput,
) -> Result<BarStretchResult, &'static str> {
    refuse_fea_unless_eligible(elig)?;
    closed_form_bar_stretch(inp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::spatial::geometry_ir::MeshIR;

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

    #[test]
    fn elasticity_preview_closed_form() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        m.indices = vec![0, 1, 2];
        m.recompute_bounds_and_hash();
        let e = promote_elasticity_preview(&m);
        assert_eq!(e.domain, AnalysisDomain::ElasticityPreview);
        let r = run_elasticity_preview_if_eligible(
            e,
            BarStretchInput {
                force_n: 1000.0,
                length_m: 1.0,
                area_m2: 0.01,
                youngs_pa: 2.0e11,
            },
        )
        .unwrap();
        assert!(r.displacement_m > 0.0);
        assert!(r.assurance_note.contains("A1"));
    }

    #[test]
    fn refuse_without_promotion() {
        let e = TwinEligibility {
            domain: AnalysisDomain::VisualizationOnly,
            validation: MeshValidationStatus::Valid,
            vertex_count: 3,
            triangle_count: 1,
            allowed: true,
            refuse_reason: "",
        };
        assert!(run_elasticity_preview_if_eligible(
            e,
            BarStretchInput {
                force_n: 1.0,
                length_m: 1.0,
                area_m2: 1.0,
                youngs_pa: 1.0,
            }
        )
        .is_err());
    }
}
