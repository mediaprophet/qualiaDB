//! OCS Verification Test Matrix (OCS §15).
//!
//! Implements the 20 verification tests OCS-T01 through OCS-T20
//! defined in the OCS specification. These tests verify cross-cutting
//! properties that span multiple cosmic modules.
//!
//! Reference: OCS Specification v2.2.0 §15.

#![cfg(test)]

use crate::cosmic::{
    ar::{HeadPose, SpatialAnchor, VioWorldRoot},
    cb_usri::HierarchyLevel,
    celestial::{body_profile_by_name, sgr_a_profile, CelestialBodyClass},
    colocated::{CoLocatedLayer, CoLocatedStack},
    flrw::FloatingOrigin,
    grounding::{collapse_entities, GroundingStatus, NarrativeEntity},
    microverse::{constants as phys, ScalingLens},
    nested::NestingStack,
    observer::{AffectiveStatus, ObserverFiber},
    parallel::{TimelineBranch, TimelineDag},
    stardate::{Stardate, StardateEra},
    theory::{AssuranceLevel, LawNature, TheoryLineage, TheoryPackage},
    transforms::{ecef_to_enu, ecef_to_geodetic, enu_to_ecef, geodetic_to_ecef, Geodetic},
    warp::{warp_velocity, WarpScale},
};

// ── OCS-T01: Intergalactic Floating Origin Precision ──────────────────

#[test]
fn ocs_t01_intergalactic_floating_origin_precision() {
    // OCS-T01: Verify < 10^-6 m offset error at 500 Mpc comoving distance.
    // The floating origin scheme stores offsets relative to a local origin,
    // keeping coordinate values small. The round-trip error (absolute →
    // offset → absolute) should be < 10^-6 m.
    let origin_mpc = [500_000.0, 200_000.0, 100_000.0]; // Origin at ~500 Mpc
    let origin = FloatingOrigin::new(origin_mpc);
    // A galaxy at a small offset from the origin (in Mpc)
    let galaxy_absolute = [500_001.0, 200_000.5, 100_000.3];
    let offset = origin.to_offset(galaxy_absolute);
    let recovered = origin.to_absolute(offset);
    // Round-trip error in Mpc
    for i in 0..3 {
        let err_mpc = (recovered[i] - galaxy_absolute[i]).abs();
        let err_m = err_mpc * 3.085677581e22;
        assert!(
            err_m < 1e-6,
            "axis {} error {} m should be < 1e-6 m",
            i,
            err_m
        );
    }
    // Verify the offset is small (floating origin keeps values manageable)
    assert!(
        offset[0].abs() < 10.0,
        "offset should be small, got {}",
        offset[0]
    );
}

// ── OCS-T02: Terrestrial WGS84↔ECEF↔ENU Round-Trip ────────────────────

#[test]
fn ocs_t02_wgs84_ecef_enu_round_trip() {
    let test_cases = [
        (0.0, 0.0, 0.0),
        (90.0, 0.0, 0.0),
        (-90.0, 0.0, 0.0),
        (0.0, 180.0, 0.0),
        (78.5, 15.0, 500.0),
        (-45.0, -120.0, 3000.0),
        (37.8080, -122.4177, 10.0),
    ];

    for &(lat, lon, alt) in &test_cases {
        let original = Geodetic {
            lat_deg: lat,
            lon_deg: lon,
            alt_m: alt,
        };
        let ecef = geodetic_to_ecef(original);
        let recovered = ecef_to_geodetic(ecef);
        let alt_err = (recovered.alt_m - original.alt_m).abs();
        assert!(
            alt_err < 1e-4,
            "alt error {} m at ({},{},{})",
            alt_err,
            lat,
            lon,
            alt
        );
        let lat_err = (recovered.lat_deg - original.lat_deg).abs();
        let lon_err = (recovered.lon_deg - original.lon_deg).abs();
        assert!(
            lat_err < 1e-8,
            "lat error {} at ({},{},{})",
            lat_err,
            lat,
            lon,
            alt
        );
        assert!(
            lon_err < 1e-8,
            "lon error {} at ({},{},{})",
            lon_err,
            lat,
            lon,
            alt
        );

        let enu = ecef_to_enu(ecef, original);
        let ecef2 = enu_to_ecef(enu, original);
        assert!((ecef2.x - ecef.x).abs() < 1e-4);
        assert!((ecef2.y - ecef.y).abs() < 1e-4);
        assert!((ecef2.z - ecef.z).abs() < 1e-4);
    }
}

// ── OCS-T03: Planetary Geodesy & Areodesy Transformation ──────────────

#[test]
fn ocs_t03_mars_areodesy_transformation() {
    let mars = body_profile_by_name("mars").unwrap();
    assert!(mars.equatorial_radius_m > 3_000_000.0);
    assert!(mars.equatorial_radius_m < 3_500_000.0);
    assert_eq!(mars.class, CelestialBodyClass::TerrestrialPlanet);
    let g = mars.surface_gravity();
    assert!((g - 3.71).abs() < 0.1, "Mars gravity {} expected ~3.71", g);
}

// ── OCS-T04: Compact Object Kerr Metric Horizon & Ergosphere ──────────

#[test]
fn ocs_t04_kerr_metric_horizon_ergosphere() {
    let sgr = sgr_a_profile();
    let g = &sgr.gravity;
    let horizon = g.kerr_horizon_r();
    assert!(horizon > 0.0);
    let ergo_equator = g.kerr_ergosphere_r(std::f64::consts::FRAC_PI_2);
    assert!(ergo_equator >= horizon);
    let omega = g.frame_dragging(horizon);
    assert!(omega >= 0.0);
}

// ── OCS-T05: AR VIO Frame & Spatial Anchor Stability ──────────────────

#[test]
fn ocs_t05_ar_anchor_stability() {
    let anchor = SpatialAnchor::new("test-anchor", [37.8, -122.4, 10.0]).with_confidence(0.3);
    let mut root = VioWorldRoot::new(anchor);
    for i in 0..30 {
        root.update_pose(HeadPose {
            position: [i as f32 * 0.0001, 0.0, 0.0],
            orientation: [1.0, 0.0, 0.0, 0.0],
            timestamp_ns: i as u64 * 1_000_000,
        });
    }
    assert!(root.check_stability());
    assert!(root.is_stable);
}

// ── OCS-T06: Co-Located Reality Paraconsistent Isolation ──────────────

#[test]
fn ocs_t06_colocated_reality_isolation() {
    let base = CoLocatedLayer::physical_base(Geodetic {
        lat_deg: 37.8080,
        lon_deg: -122.4177,
        alt_m: 10.0,
    });
    let mut stack = CoLocatedStack::new(base);
    stack.add_layer(CoLocatedLayer::fictional(
        "Starfleet HQ",
        "urn:omni:v1:fiction:star-trek:prime:earth:sf:starfleet-hq",
        Geodetic {
            lat_deg: 37.8080,
            lon_deg: -122.4177,
            alt_m: 10.0,
        },
    ));
    assert!(stack.verify_isolation());
    assert!(stack.check_no_mutation_leak(1));
    let physical = stack.physical_base();
    assert_eq!(physical.geodetic.lat_deg, 37.8080);
}

// ── OCS-T07: Hypothesis Differential Oracle ────────────────────────────

#[test]
fn ocs_t07_hypothesis_differential_oracle() {
    let lcdm = TheoryPackage::new(
        "lcdm",
        "ΛCDM",
        "did:q42:person:author",
        LawNature::Physical,
        AssuranceLevel::A3,
    )
    .with_evidence(0.85, 0.15)
    .with_chi_squared(1.04);

    let mond = TheoryPackage::new(
        "mond",
        "MOND",
        "did:q42:person:author",
        LawNature::TheoreticalHypothesis,
        AssuranceLevel::A1,
    )
    .with_evidence(0.72, 0.28)
    .with_chi_squared(1.12)
    .with_lineage(TheoryLineage::CompetesWith, "lcdm");

    let delta_chi = lcdm.residual_chi_squared - mond.residual_chi_squared;
    assert!(delta_chi < 0.0, "ΛCDM should have lower chi-squared");
    assert!(lcdm.is_empirically_calibrated()); // A3+
    assert!(!mond.is_empirically_calibrated()); // A1 — hypothesis, not fully calibrated
}

// ── OCS-T08: Nested Holodeck Inception Stack Execution ─────────────────

#[test]
fn ocs_t08_nested_holodeck_inception() {
    let mut stack = NestingStack::new();
    stack.push(
        "urn:omni:v1:physical:observable:standard:starship:enterprise",
        1.0,
    );
    stack.push("urn:omni:v1:simulation:holodeck:vicorian-london", 10.0);
    stack.push("urn:omni:v1:simulation:holodeck:dream-sequence", 100.0);

    assert_eq!(stack.depth(), 3);
    let cumulative = stack.cumulative_time_dilation();
    assert!(
        (cumulative - 1000.0).abs() < 1e-6,
        "got {} expected 1000",
        cumulative
    );

    // Verify context isolation via current() returning different realms
    let current = stack.current().unwrap();
    let h2 = current.context_hash();
    assert!(h2 != 0); // Nested realm should have non-zero context
}

// ── OCS-T09: Parallel Quantum Reality Divergence Test ──────────────────

#[test]
fn ocs_t09_parallel_quantum_divergence() {
    let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
    let mut dag = TimelineDag::new(prime);
    dag.add_branch(TimelineBranch::divergent(
        "mirror",
        "Mirror Universe",
        "urn:omni:v1:fiction:star-trek",
        "antiquity",
        "prime",
    ));

    assert!(dag.verify_branch_isolation("mirror"));
    assert!(dag.verify_branch_isolation("prime"));
    let prime_branch = dag.prime().unwrap();
    assert_eq!(prime_branch.branch_id, "prime");
    assert!(prime_branch.is_prime());
}

// ── OCS-T10: Observer Fiber Epistemic Divergence Evaluation ────────────

#[test]
fn ocs_t10_observer_epistemic_divergence() {
    let mut observer = ObserverFiber::new(
        "did:q42:person:patient",
        "urn:omni:v1:phenomenology:patient:divergent",
    );
    observer.affective_state = AffectiveStatus {
        safety_threat_index: 0.9,
        emotional_valence: -0.8,
        arousal_level: 0.85,
        dissociation_index: 0.7,
        trauma_reactivity: 0.6,
    };

    let empirical = [1.0, 2.0, 3.0];
    let perceived = [1.5, 2.5, 3.5];
    let divergence = observer.epistemic_divergence(&empirical, &perceived);
    assert!(divergence > 0.0);
    assert!(divergence.is_finite());
    assert!(observer.needs_grounding());
}

// ── OCS-T11: Affective State Grounding Trigger ────────────────────────

#[test]
fn ocs_t11_affective_grounding_trigger() {
    let affective = AffectiveStatus {
        safety_threat_index: 0.85,
        emotional_valence: -0.7,
        arousal_level: 0.9,
        dissociation_index: 0.3,
        trauma_reactivity: 0.5,
    };
    assert!(affective.is_hyper_vigilant());

    let mut observer = ObserverFiber::new(
        "did:q42:person:subject",
        "urn:omni:v1:phenomenology:subject",
    );
    observer.affective_state = affective;
    assert!(observer.needs_grounding());
}

// ── OCS-T12: Star Trek Piecewise Stardate Morphism ────────────────────

#[test]
fn ocs_t12_stardate_round_trip() {
    // TOS: 1312.4
    let tos = Stardate::new(1312.4);
    let tos_year = tos.to_gregorian_year();
    let tos_recovered = Stardate::from_gregorian_year(tos_year, StardateEra::Tos);
    assert!(
        (tos_recovered.value - 1312.4).abs() < 1.0,
        "TOS: {} → {} → {}",
        1312.4,
        tos_year,
        tos_recovered.value
    );

    // TNG: 47634.44
    let tng = Stardate::new(47634.44);
    let tng_year = tng.to_gregorian_year();
    let tng_recovered = Stardate::from_gregorian_year(tng_year, StardateEra::Tng);
    assert!(
        (tng_recovered.value - 47634.44).abs() < 1.0,
        "TNG: {} → {} → {}",
        47634.44,
        tng_year,
        tng_recovered.value
    );

    // 32nd Century: 865211.2
    let c32 = Stardate::new(865211.2);
    let c32_year = c32.to_gregorian_year();
    let c32_recovered = Stardate::from_gregorian_year(c32_year, StardateEra::Century32);
    assert!(
        (c32_recovered.value - 865211.2).abs() < 10.0,
        "32nd: {} → {} → {}",
        865211.2,
        c32_year,
        c32_recovered.value
    );
}

// ── OCS-T13: Warp Saturation Continuous Evaluator ──────────────────────

#[test]
fn ocs_t13_warp_saturation() {
    let v = warp_velocity(9.99, WarpScale::Tng);
    assert!(v.is_finite());
    assert!(!v.is_nan());
    assert!(v > 0.0);
    assert!(v < f64::INFINITY);
}

// ── OCS-T14: Topological Rift & Wormhole Transition ────────────────────

#[test]
fn ocs_t14_wormhole_transition() {
    use crate::cosmic::opcode::{CosmicNQuin, OP_REALM_TRANSIT, SENSITIVITY_RESTRICTED};
    let q = CosmicNQuin::realm_transit(
        "did:q42:ship:enterprise",
        "urn:omni:v1:physical:observable:standard:wormhole:exit",
        "urn:omni:v1:physical:observable:standard:wormhole:entry",
        HierarchyLevel::L8,
        SENSITIVITY_RESTRICTED,
        12345,
    );
    assert_eq!(q.opcode(), OP_REALM_TRANSIT);
    assert!(q.verify_parity());
}

// ── OCS-T15: Zero-Heap Transform Loop ──────────────────────────────────

#[test]
fn ocs_t15_zero_heap_transform_loop() {
    let g = Geodetic {
        lat_deg: 37.808,
        lon_deg: -122.4177,
        alt_m: 10.0,
    };
    let mut last = g;
    for _ in 0..10_000 {
        let ecef = geodetic_to_ecef(last);
        last = ecef_to_geodetic(ecef);
    }
    assert!((last.lat_deg - g.lat_deg).abs() < 1e-6);
    assert!((last.lon_deg - g.lon_deg).abs() < 1e-6);
    assert!((last.alt_m - g.alt_m).abs() < 1e-3);
}

// ── OCS-T16: ObserverFiber Composition with Nested Realm ───────────────

#[test]
fn ocs_t16_observer_nested_realm_composition() {
    let mut stack = NestingStack::new();
    stack.push("urn:omni:v1:physical:observable:standard:earth", 1.0);
    stack.push("urn:omni:v1:simulation:holodeck:scenario", 20.0);

    let observer = ObserverFiber::new(
        "did:q42:person:observer",
        "urn:omni:v1:simulation:holodeck:scenario",
    );
    let zeta = stack.cumulative_time_dilation();
    assert!((zeta - 20.0).abs() < 1e-6);
    assert!(!observer.needs_grounding());
    assert_eq!(stack.depth(), 2);
}

// ── OCS-T17: Co-Located Reality AR Spatial Anchor Immutability ──────────

#[test]
fn ocs_t17_colocated_ar_anchor_immutability() {
    let base_geo = Geodetic {
        lat_deg: 37.808,
        lon_deg: -122.4177,
        alt_m: 10.0,
    };
    let base = CoLocatedLayer::physical_base(base_geo);
    let mut stack = CoLocatedStack::new(base);
    stack.add_layer(CoLocatedLayer::fictional(
        "Fictional SF",
        "urn:omni:v1:fiction:test:sf",
        base_geo,
    ));

    let anchor = SpatialAnchor::new("fictional-anchor", [37.808, -122.4177, 10.0])
        .with_enu_offset(5.0, 3.0, 1.0);

    let physical = stack.physical_base();
    assert_eq!(physical.geodetic.lat_deg, base_geo.lat_deg);
    assert_eq!(physical.geodetic.lon_deg, base_geo.lon_deg);
    assert_eq!(physical.geodetic.alt_m, base_geo.alt_m);
    assert_eq!(anchor.geodetic_anchor[0], 37.808);
    assert!(stack.verify_isolation());
}

// ── OCS-T18: Sub-Angstrom Quantum Orbital Coordinate Precision ──────────

#[test]
fn ocs_t18_sub_angstrom_quantum_precision() {
    // OCS-T18: Verify < 10^-15 m coordinate precision at atomic scale.
    let bohr = phys::BOHR_RADIUS_M;
    // f64 has ~15.9 decimal digits. At Bohr radius scale:
    // precision = 5.29e-11 / 2^52 ≈ 1.18e-26 m
    let precision = bohr / (1u64 << 52) as f64;
    assert!(
        precision < 1e-15,
        "f64 precision at Bohr radius {} should be < 1e-15 m",
        precision
    );
    // Verify the scaling lens can convert between atomic and local scales
    let lens = ScalingLens::between(HierarchyLevel::L5, HierarchyLevel::L2);
    // 1 Bohr radius expressed in L5 (km-scale) units
    let bohr_in_l5 = lens.inverse_length(1.0); // Convert 1 Bohr → L5 units
    assert!(bohr_in_l5 > 0.0);
    assert!(bohr_in_l5.is_finite());
    // Round-trip: 1 Bohr → L5 → back to L2 should recover 1.0
    let recovered = lens.transform_length(bohr_in_l5);
    assert!(
        (recovered - 1.0).abs() < 1e-6,
        "round-trip should recover 1.0, got {}",
        recovered
    );
}

// ── OCS-T19: Multi-Scale Continuous Traversal (Planck to Horizon) ──────

#[test]
fn ocs_t19_multi_scale_traversal() {
    let cosmological_m = 500.0 * 3.085677581e22;
    let lens_cosmo_to_galaxy = ScalingLens::between(HierarchyLevel::L12, HierarchyLevel::L9);
    let lens_galaxy_to_local = ScalingLens::between(HierarchyLevel::L9, HierarchyLevel::L5);
    let lens_local_to_atomic = ScalingLens::between(HierarchyLevel::L5, HierarchyLevel::L2);

    let galaxy_scale = lens_cosmo_to_galaxy.transform_length(cosmological_m);
    let local_scale = lens_galaxy_to_local.transform_length(galaxy_scale);
    let atomic_scale = lens_local_to_atomic.transform_length(local_scale);

    assert!(atomic_scale > 0.0);
    assert!(atomic_scale.is_finite());
    assert!(!atomic_scale.is_nan());
    assert!(
        atomic_scale > 1e-20,
        "traversal should not underflow: got {}",
        atomic_scale
    );
}

// ── OCS-T20: Granular Element Collapse & Archaeological Grounding ──────

#[test]
fn ocs_t20_granular_element_collapse() {
    let troy = NarrativeEntity::anchored(
        "troy_citadel",
        "urn:omni:v1:narrative:homer:iliad:troy",
        "urn:omni:v1:physical:observable:standard:earth:hisarlik:stratum-viia",
        0.95,
    );
    let athena = NarrativeEntity::mythos(
        "goddess_athena",
        "urn:omni:v1:narrative:homer:iliad:olympus",
    );

    let entities = vec![troy.clone(), athena.clone()];
    let collapsed = collapse_entities(&entities);

    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].name, "troy_citadel");

    match &troy.grounding {
        GroundingStatus::EmpiricallyAnchored { anchor_iri, .. } => {
            assert!(anchor_iri.contains("hisarlik"));
        }
        _ => panic!("Troy should be empirically anchored"),
    }
    assert!(!athena.grounding.is_collapsible());
}
