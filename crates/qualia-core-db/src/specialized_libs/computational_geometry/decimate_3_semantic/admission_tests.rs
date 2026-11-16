//! Source-authored P06 regressions. Run at the consolidated integration milestone.
use super::super::normalized_cost::normalized_cost;
use super::*;

#[test]
fn normalization_is_invariant_under_declared_world_scale() {
    let a = SemanticVertexAttributes {
        normal: [1.0, 0.0, 0.0],
        uv: [0.0, 0.0],
        bifurcation_sensitivity: 0.2,
        ..Default::default()
    };
    let b = SemanticVertexAttributes { normal: [0.0, 1.0, 0.0], uv: [0.5, 0.25], bifurcation_sensitivity: 0.4, ..a };
    let evaluate = |scale: f64| {
        normalized_cost(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(scale, 0.0, 0.0),
            Point3::new(0.5 * scale, 0.0, 0.0),
            2.0 * scale.powi(4),
            &a,
            &b,
            &NormalizedCostProfile { length_scale: scale, ..Default::default() },
        )
        .unwrap()
    };
    let reference = evaluate(1.0);
    for scale in [1e-3, 1.0, 1e3] {
        let result = evaluate(scale);
        assert!((result.total - reference.total).abs() < 1e-12);
        assert!((result.geometric_quadric - 2.0).abs() < 1e-12);
    }
}

fn compatible(
    left: SemanticDomains,
    right: SemanticDomains,
    a: SemanticVertexAttributes,
    b: SemanticVertexAttributes,
) -> bool {
    let vertices = [Point3::default(); 3];
    let attributes = [a, b, a];
    let triangles = [[0, 1, 2]];
    let domains = [left, right, left];
    let ids = [1, 2, 3];
    let input = AdmissionInput {
        vertices: &vertices,
        attributes: &attributes,
        triangles: &triangles,
        domains: &domains,
        stable_vertex_ids: &ids,
    };
    let mut slots = [AdmissionSlot::empty(); 3];
    slots[0].attributes = a;
    slots[1].attributes = b;
    bounded_driver::hard_compatible(&input, &slots, 0, 1)
}

#[test]
fn every_hard_domain_and_protected_endpoint_precedes_ranking() {
    let domain = SemanticDomains::default();
    let a = SemanticVertexAttributes::default();
    assert!(compatible(domain, domain, a, a));
    let alternatives = [
        SemanticDomains { material: 1, ..domain },
        SemanticDomains { provenance: 1, ..domain },
        SemanticDomains { epistemic: 1, ..domain },
        SemanticDomains { subpart: 1, ..domain },
        SemanticDomains { feature_curve: 1, ..domain },
        SemanticDomains { attachment: 1, ..domain },
        SemanticDomains { protected_region: 1, ..domain },
        SemanticDomains { uv_chart: 1, ..domain },
        SemanticDomains { tangent_handedness: -1, ..domain },
    ];
    for changed in alternatives {
        assert!(!compatible(domain, changed, a, a));
        assert!(!compatible(changed, domain, a, a));
    }
    for protected in [
        SemanticDomains { feature_curve: 1, ..domain },
        SemanticDomains { attachment: 1, ..domain },
        SemanticDomains { protected_region: 1, ..domain },
    ] {
        assert!(!compatible(protected, protected, a, a));
    }
    let locked = SemanticVertexAttributes { cluster_border_locked: true, ..a };
    assert!(!compatible(domain, domain, locked, a));
    assert!(!compatible(domain, domain, a, locked));
    assert!(!compatible(domain, domain, a, SemanticVertexAttributes { skeleton_joint_id: 1, ..a }));
    assert!(!compatible(domain, domain, a, SemanticVertexAttributes { subpart_tag: 1, ..a }));
    assert!(!compatible(domain, domain, a, SemanticVertexAttributes { affordance_boundary: true, ..a }));
}

fn validate(vertices: &[Point3], triangles: &[[u32; 3]], work: u64) -> Result<(), AdmissionError> {
    let attributes = vec![SemanticVertexAttributes::default(); vertices.len()];
    let domains = vec![SemanticDomains::default(); vertices.len()];
    let ids: Vec<u64> = (0..vertices.len() as u64).collect();
    let input =
        AdmissionInput { vertices, attributes: &attributes, triangles, domains: &domains, stable_vertex_ids: &ids };
    let mut slots = vec![AdmissionSlot::empty(); vertices.len().max(triangles.len())];
    let cancellation = Cancellation::new();
    let mut budget = ProductionBudget::admit(CapacityPlan::default(), usize::MAX, work, &cancellation).unwrap();
    topology::validate_input(&input, &mut slots, &mut budget)
}
fn field(result: Result<(), AdmissionError>) -> &'static str {
    match result {
        Err(AdmissionError::Contract(e)) => e.field,
        other => panic!("expected structural rejection: {other:?}"),
    }
}

#[test]
fn strict_input_rejects_duplicate_faces_nonmanifold_edges_and_vertex_bowties() {
    let p = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(-1.0, 0.0, 0.0),
        Point3::new(0.0, -1.0, 0.0),
    ];
    assert_eq!(field(validate(&p[..3], &[[0, 1, 2], [2, 1, 0]], 1_000_000)), "duplicate_triangle");
    assert_eq!(field(validate(&p, &[[0, 1, 2], [0, 3, 4]], 1_000_000)), "disconnected_vertex_link");
    let p = [p[0], p[1], p[2], p[4], Point3::new(0.0, 0.0, 1.0)];
    assert_eq!(field(validate(&p, &[[0, 1, 2], [1, 0, 3], [0, 1, 4]], 1_000_000)), "edge_incidence_or_orientation");
    assert!(matches!(
        validate(&p[..3], &[[0, 1, 2]], 0),
        Err(AdmissionError::Budget(BudgetError::WorkLimitExceeded { .. }))
    ));
}
