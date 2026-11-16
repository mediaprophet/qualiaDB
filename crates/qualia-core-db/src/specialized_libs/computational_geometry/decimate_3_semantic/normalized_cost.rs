//! Dimensionless ranking; this score is not a surface-distance acceptance bound.
use super::SemanticVertexAttributes;
use crate::specialized_libs::computational_geometry::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedCostProfile {
    /// Positive world reference length D; area-weighted QEM scales as D^4.
    pub length_scale: f64,
    /// Unit-normal chord distance, not an angle in radians.
    pub normal_scale: f64,
    pub uv_scale: f64,
    pub sensitivity_scale: f64,
    pub geometry_weight: f64,
    pub normal_weight: f64,
    pub uv_weight: f64,
    pub sensitivity_spatial_weight: f64,
    pub sensitivity_gradient_weight: f64,
}
impl Default for NormalizedCostProfile {
    fn default() -> Self {
        Self {
            length_scale: 1.0,
            normal_scale: 1.0,
            uv_scale: 1.0,
            sensitivity_scale: 1.0,
            geometry_weight: 1.0,
            normal_weight: 1.0,
            uv_weight: 1.0,
            sensitivity_spatial_weight: 1.0,
            sensitivity_gradient_weight: 1.0,
        }
    }
}
impl NormalizedCostProfile {
    pub fn is_valid(&self) -> bool {
        let scales = [self.length_scale, self.normal_scale, self.uv_scale, self.sensitivity_scale];
        let weights = [
            self.geometry_weight,
            self.normal_weight,
            self.uv_weight,
            self.sensitivity_spatial_weight,
            self.sensitivity_gradient_weight,
        ];
        scales.iter().all(|x| x.is_finite() && *x > 0.0 && (x * x).is_finite() && x * x > 0.0)
            && weights.iter().all(|x| x.is_finite() && *x >= 0.0)
            && self.length_scale.powi(4).is_finite()
            && self.length_scale.powi(4) > 0.0
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct NormalizedCostTerms {
    pub geometric_quadric: f64,
    pub normal_chord: f64,
    pub uv_distance: f64,
    pub sensitivity_spatial: f64,
    pub sensitivity_gradient: f64,
    pub total: f64,
}
fn squared_distance(a: Point3, b: Point3) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)
}
/// Hard semantic/topology/transfer admission must precede this ranking helper.
pub fn normalized_cost(
    lo: Point3,
    hi: Point3,
    target: Point3,
    geometric_cost: f64,
    a: &SemanticVertexAttributes,
    b: &SemanticVertexAttributes,
    profile: &NormalizedCostProfile,
) -> Option<NormalizedCostTerms> {
    if !profile.is_valid() || !geometric_cost.is_finite() {
        return None;
    }
    let d2 = profile.length_scale.powi(2);
    let displacement = (squared_distance(target, lo) + squared_distance(target, hi)) / d2;
    let normal_chord = a.normal.iter().zip(b.normal).map(|(x, y)| (x - y).powi(2)).sum::<f64>();
    let uv_distance = a.uv.iter().zip(b.uv).map(|(x, y)| (x - y).powi(2)).sum::<f64>();
    let sensitivity = a.bifurcation_sensitivity.max(b.bifurcation_sensitivity);
    let mut terms = NormalizedCostTerms {
        geometric_quadric: geometric_cost.max(0.0) / profile.length_scale.powi(4),
        normal_chord: normal_chord / profile.normal_scale.powi(2) * displacement,
        uv_distance: uv_distance / profile.uv_scale.powi(2) * displacement,
        sensitivity_spatial: sensitivity / profile.sensitivity_scale * (squared_distance(lo, hi) / d2 + displacement),
        sensitivity_gradient: ((a.bifurcation_sensitivity - b.bifurcation_sensitivity) / profile.sensitivity_scale)
            .powi(2),
        total: 0.0,
    };
    terms.total = profile.geometry_weight * terms.geometric_quadric
        + profile.normal_weight * terms.normal_chord
        + profile.uv_weight * terms.uv_distance
        + profile.sensitivity_spatial_weight * terms.sensitivity_spatial
        + profile.sensitivity_gradient_weight * terms.sensitivity_gradient;
    [
        terms.geometric_quadric,
        terms.normal_chord,
        terms.uv_distance,
        terms.sensitivity_spatial,
        terms.sensitivity_gradient,
        terms.total,
    ]
    .iter()
    .all(|x| x.is_finite() && *x >= 0.0)
    .then_some(terms)
}
